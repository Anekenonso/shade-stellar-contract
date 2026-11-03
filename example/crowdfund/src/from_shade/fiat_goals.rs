//! Fiat-pegged funding goals for crowdfunding campaigns.
//!
//! A campaign's `goal_amount` is denominated in its token, which makes "raise
//! $10,000" impossible to express: the token's fiat value drifts under the
//! campaign for its whole duration. A merchant can instead peg the target to a
//! fiat currency here. Contributions still arrive in the token, but each is
//! valued in fiat through the token's price oracle — the same oracle registry
//! the fiat-invoice engine uses — and it is that fiat total the target is
//! measured against.
//!
//! Pegs attach to the campaign registry in [`crate::from_shade::campaigns`]
//! (the [`Campaign`] record that stretch goals also build on), not to the
//! all-or-nothing pledge campaigns in [`crate::from_shade::pledge`].
//!
//! # Valuation model
//!
//! Every contribution is valued **once**, at the price of the ledger it landed
//! on, and that snapshot accrues into `raised_amount`. The alternative —
//! revaluing the whole raise at the current price — would let a price swing
//! move a campaign back below a target it had already met, so a goal could be
//! reached, unreached, then reached again. Snapshotting makes progress
//! monotonic: the total only ever grows, and `reached_at` is set once.
//!
//! The token→fiat conversion is the exact inverse of the fiat→token conversion
//! [`shade::components::invoice`] applies to fiat invoices, so a fiat invoice
//! and a fiat goal price the same trade identically.
//!
//! # Invariants
//!
//! - Only the campaign's owning merchant may publish a peg or refresh its
//!   quote; the merchant or the contract admin may close one.
//! - A campaign has at most one peg, and its `currency`, `goal_amount` and
//!   `decimals` never change once published — backers can rely on the target
//!   they were shown. Closing is the one state change available.
//! - `raised_amount`, `raised_tokens` and `contribution_count` only ever grow,
//!   and `status` only ever moves forward (`Active` → `Reached` → `Closed`).
//! - A peg cannot be published against a token with no configured oracle, or
//!   one whose oracle has no usable price, so a published target is always
//!   measurable.
//!
//! # Storage
//!
//! Keys live in [`FiatGoalKey`], a dedicated enum, so this feature adds no
//! cases to the near-full `CampaignKey` (Soroban caps every enum at 50 cases).
//! A peg is keyed by its `campaign_id` rather than a fresh ID, which avoids a
//! counter entry entirely and keeps lookups to one read. Live valuations are
//! derived on demand rather than stored, so nothing pays rent to hold a figure
//! that the next price tick invalidates.

use shade::components::invoice::PriceOracleClient;
use shade::components::admin;
use crate::from_shade::campaigns;
use shade::components::core;
use shade::components::reentrancy;
use crate::from_shade::errors::{CampaignError, ContractError, FiatGoalError};
use crate::from_shade::events;
use shade::types::{OracleConfig};
use crate::from_shade::types::{Campaign, CampaignFiatGoal, FiatGoalKey, FiatGoalQuote, FiatGoalStatus};
use soroban_sdk::{panic_with_error, Address, Env, String, Vec};

/// Basis-point denominator; 10_000 bps = 100%.
const BPS_DENOMINATOR: i128 = 10_000;

/// Longest accepted quote-currency code. ISO 4217 codes are three characters;
/// the slack covers non-standard tickers without letting an unbounded string
/// into every event this component emits.
const MAX_CURRENCY_LEN: u32 = 8;

/// Most fractional digits a fiat target may carry. Real currencies use at most
/// four; the cap bounds the conversion's scale factors well clear of `i128`.
const MAX_FIAT_DECIMALS: u32 = 18;

// ── Conversion helpers ────────────────────────────────────────────────────────

/// `a * b`, panicking with a contract error rather than trapping on overflow.
fn mul(env: &Env, a: i128, b: i128) -> i128 {
    a.checked_mul(b)
        .unwrap_or_else(|| panic_with_error!(env, shade::errors::ContractError::InvalidAmount))
}

/// `10^decimals`. Overflows only on an oracle configured with nonsensical
/// decimals, which surfaces as [`FiatGoalError::InvalidFiatDecimals`] rather
/// than an unrecoverable trap.
fn scale_factor(env: &Env, decimals: u32) -> i128 {
    let mut factor = 1i128;
    for _ in 0..decimals {
        factor = factor
            .checked_mul(10)
            .unwrap_or_else(|| panic_with_error!(env, FiatGoalError::InvalidFiatDecimals));
    }
    factor
}

/// Reads `token`'s oracle price for `currency`, rejecting a non-positive quote.
///
/// Returns the oracle config alongside the price: callers need its decimals to
/// interpret the figure, and re-reading it would cost a second storage read.
fn read_price(env: &Env, token: &Address, currency: &String) -> (OracleConfig, i128) {
    let config = admin::get_token_oracle(env, token);
    let price = PriceOracleClient::new(env, &config.contract).get_price(token, currency);
    if price <= 0 {
        panic_with_error!(env, shade::errors::ContractError::OraclePriceUnavailable);
    }
    (config, price)
}

/// Fiat value of `token_amount` base units at `price`, in minor units scaled by
/// `fiat_decimals`.
///
/// `price` quotes one *whole* token, scaled by the oracle's `price_decimals`,
/// so the base-unit and price scales both divide out.
fn token_to_fiat(
    env: &Env,
    token_amount: i128,
    price: i128,
    fiat_decimals: u32,
    config: &OracleConfig,
) -> i128 {
    let numerator = mul(
        env,
        mul(env, token_amount, price),
        scale_factor(env, fiat_decimals),
    );
    let denominator = mul(
        env,
        scale_factor(env, config.token_decimals),
        scale_factor(env, config.price_decimals),
    );
    numerator / denominator
}

/// Token base units worth `fiat_amount` at `price` — the inverse of
/// [`token_to_fiat`], rounded **up** so contributing the result always covers
/// the fiat figure rather than landing a minor unit short of it.
///
/// Callers guarantee `price > 0` (see [`read_price`]).
fn fiat_to_token(
    env: &Env,
    fiat_amount: i128,
    price: i128,
    fiat_decimals: u32,
    config: &OracleConfig,
) -> i128 {
    if fiat_amount <= 0 {
        return 0;
    }
    let numerator = mul(
        env,
        mul(env, fiat_amount, scale_factor(env, config.token_decimals)),
        scale_factor(env, config.price_decimals),
    );
    let denominator = mul(env, price, scale_factor(env, fiat_decimals));
    // Ceiling division; both operands are positive here.
    (numerator + denominator - 1) / denominator
}

/// Progress toward `goal` in basis points, capped at 100%. `goal` is always
/// positive, and `raised` never negative.
fn progress_bps(env: &Env, raised: i128, goal: i128) -> u32 {
    if raised >= goal {
        return BPS_DENOMINATOR as u32;
    }
    let bps = mul(env, raised, BPS_DENOMINATOR) / goal;
    u32::try_from(bps).unwrap_or(BPS_DENOMINATOR as u32)
}

// ── Storage helpers ───────────────────────────────────────────────────────────

fn save_goal(env: &Env, goal: &CampaignFiatGoal) {
    env.storage()
        .persistent()
        .set(&FiatGoalKey::CampaignFiatGoal(goal.campaign_id), goal);
}

/// Loads a campaign's peg, panicking with [`FiatGoalError::FiatGoalNotFound`]
/// if it has none.
pub fn get_campaign_fiat_goal(env: &Env, campaign_id: u64) -> CampaignFiatGoal {
    env.storage()
        .persistent()
        .get(&FiatGoalKey::CampaignFiatGoal(campaign_id))
        .unwrap_or_else(|| panic_with_error!(env, FiatGoalError::FiatGoalNotFound))
}

/// Whether `campaign_id` has a fiat-pegged goal. A cheap pre-check for callers
/// that must not panic on an unpegged campaign.
pub fn has_campaign_fiat_goal(env: &Env, campaign_id: u64) -> bool {
    env.storage()
        .persistent()
        .has(&FiatGoalKey::CampaignFiatGoal(campaign_id))
}

/// Cumulative fiat value `backer` has contributed to `campaign_id`, in the
/// goal's minor units. `0` for a backer who has not contributed.
pub fn get_backer_fiat_contribution(env: &Env, campaign_id: u64, backer: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&FiatGoalKey::BackerFiatContribution(
            campaign_id,
            backer.clone(),
        ))
        .unwrap_or(0)
}

/// Asserts `caller` owns `campaign_id` and returns the campaign.
fn load_owned_campaign(env: &Env, campaign_id: u64, caller: &Address) -> Campaign {
    let campaign = campaigns::get_campaign(env, campaign_id);
    if campaign.merchant != *caller {
        panic_with_error!(env, FiatGoalError::NotFiatGoalOwner);
    }
    campaign
}

// ── Actions ───────────────────────────────────────────────────────────────────

/// Pegs `campaign_id`'s funding target to `goal_amount` of `currency`.
///
/// `goal_amount` is in minor units scaled by `decimals`, so $10,000.00 is
/// `1_000_000` with `decimals = 2`. The campaign's token must already have an
/// oracle quoting `currency`, and that oracle is read here so a peg can never
/// be published against a price feed that does not answer.
///
/// Callable only by the campaign's owning merchant, once per campaign: the
/// terms are fixed at publication, exactly as vesting schedules are, so backers
/// can rely on the target they pledged against.
pub fn set_campaign_fiat_goal(
    env: &Env,
    merchant: &Address,
    campaign_id: u64,
    currency: &String,
    goal_amount: i128,
    decimals: u32,
) {
    merchant.require_auth();

    let campaign = load_owned_campaign(env, campaign_id, merchant);
    if !campaign.active {
        panic_with_error!(env, CampaignError::CampaignInactive);
    }

    let now = env.ledger().timestamp();
    // A peg on an already-expired campaign could never be contributed to.
    if now > campaign.deadline {
        panic_with_error!(env, CampaignError::CampaignExpired);
    }
    if has_campaign_fiat_goal(env, campaign_id) {
        panic_with_error!(env, FiatGoalError::FiatGoalAlreadyExists);
    }
    if goal_amount <= 0 {
        panic_with_error!(env, FiatGoalError::InvalidFiatGoalAmount);
    }
    if currency.is_empty() || currency.len() > MAX_CURRENCY_LEN {
        panic_with_error!(env, FiatGoalError::InvalidFiatCurrency);
    }
    if decimals > MAX_FIAT_DECIMALS {
        panic_with_error!(env, FiatGoalError::InvalidFiatDecimals);
    }

    let (config, price) = read_price(env, &campaign.token, currency);
    // Proves the target is convertible in both directions before publishing it,
    // rather than failing on the first contribution.
    let token_goal_estimate = fiat_to_token(env, goal_amount, price, decimals, &config);

    let goal = CampaignFiatGoal {
        campaign_id,
        merchant: merchant.clone(),
        token: campaign.token.clone(),
        currency: currency.clone(),
        goal_amount,
        decimals,
        raised_amount: 0,
        raised_tokens: 0,
        contribution_count: 0,
        status: FiatGoalStatus::Active,
        created_at: now,
        last_price: price,
        last_priced_at: now,
        reached_at: 0,
    };
    save_goal(env, &goal);

    events::publish_campaign_fiat_goal_set_event(
        env,
        campaign_id,
        merchant.clone(),
        campaign.token,
        currency.clone(),
        goal_amount,
        decimals,
        config.contract,
        price,
        config.price_decimals,
        token_goal_estimate,
        campaign.deadline,
        now,
    );
}

/// Values `token_amount` base units against `campaign_id`'s fiat peg, credits
/// the fiat figure to the goal, and returns it.
///
/// Like [`campaigns::record_contribution`], which it delegates the campaign's
/// token-denominated accounting to, this is an accounting entry and moves no
/// tokens: campaigns settle over whatever payment rail they like while keeping
/// their totals on-chain. It inherits that function's checks, so a campaign
/// that has been deactivated or has passed its deadline is rejected.
///
/// The oracle is a foreign contract, so the call is guarded: a token or oracle
/// that re-enters here mid-valuation is rejected rather than allowed to fold a
/// second contribution in against the pre-update state.
pub fn record_fiat_contribution(
    env: &Env,
    contributor: &Address,
    campaign_id: u64,
    token_amount: i128,
) -> i128 {
    reentrancy::enter(env);
    contributor.require_auth();

    let mut goal = get_campaign_fiat_goal(env, campaign_id);
    if goal.status == FiatGoalStatus::Closed {
        panic_with_error!(env, FiatGoalError::FiatGoalClosed);
    }
    if token_amount <= 0 {
        panic_with_error!(env, shade::errors::ContractError::InvalidAmount);
    }

    let (config, price) = read_price(env, &goal.token, &goal.currency);
    let fiat_amount = token_to_fiat(env, token_amount, price, goal.decimals, &config);
    // Dust that rounds to nothing would advance `contribution_count` and the
    // token total while crediting no fiat at all; reject it outright.
    if fiat_amount <= 0 {
        panic_with_error!(env, FiatGoalError::FiatValueTooSmall);
    }

    // Runs before the goal is updated so its campaign-state checks (active, not
    // past deadline) reject the contribution before anything here accrues.
    campaigns::record_contribution(env, campaign_id, contributor, token_amount);

    // Saturating throughout: a raise large enough to overflow these counters is
    // unreachable, and clamping beats trapping on the release profile's
    // overflow checks if it ever were reached.
    let now = env.ledger().timestamp();
    goal.raised_amount = goal.raised_amount.saturating_add(fiat_amount);
    goal.raised_tokens = goal.raised_tokens.saturating_add(token_amount);
    goal.contribution_count = goal.contribution_count.saturating_add(1);
    goal.last_price = price;
    goal.last_priced_at = now;

    let newly_reached =
        goal.status == FiatGoalStatus::Active && goal.raised_amount >= goal.goal_amount;
    if newly_reached {
        goal.status = FiatGoalStatus::Reached;
        goal.reached_at = now;
    }
    save_goal(env, &goal);

    let backer_key = FiatGoalKey::BackerFiatContribution(campaign_id, contributor.clone());
    let backer_total =
        get_backer_fiat_contribution(env, campaign_id, contributor).saturating_add(fiat_amount);
    env.storage().persistent().set(&backer_key, &backer_total);

    events::publish_fiat_contribution_event(
        env,
        campaign_id,
        contributor.clone(),
        goal.token.clone(),
        token_amount,
        fiat_amount,
        goal.currency.clone(),
        price,
        config.price_decimals,
        goal.raised_amount,
        goal.goal_amount,
        progress_bps(env, goal.raised_amount, goal.goal_amount),
        now,
    );

    if newly_reached {
        events::publish_fiat_goal_reached_event(
            env,
            campaign_id,
            goal.merchant.clone(),
            goal.currency.clone(),
            goal.goal_amount,
            goal.raised_amount,
            goal.raised_tokens,
            goal.contribution_count,
            now,
        );
    }

    reentrancy::exit(env);
    fiat_amount
}

/// Re-reads the oracle and publishes a fresh valuation of `campaign_id`'s peg,
/// returning the quote.
///
/// [`get_campaign_fiat_goal_quote`] answers the same question without writing,
/// and is what a UI should call. This exists for indexers that want the
/// shortfall on the ledger between contributions; it is merchant-only because
/// it writes, and a permissionless write is a way to burn someone else's rent.
pub fn refresh_campaign_fiat_quote(
    env: &Env,
    merchant: &Address,
    campaign_id: u64,
) -> FiatGoalQuote {
    merchant.require_auth();

    let mut goal = get_campaign_fiat_goal(env, campaign_id);
    if goal.merchant != *merchant {
        panic_with_error!(env, FiatGoalError::NotFiatGoalOwner);
    }
    if goal.status == FiatGoalStatus::Closed {
        panic_with_error!(env, FiatGoalError::FiatGoalClosed);
    }

    let (config, price) = read_price(env, &goal.token, &goal.currency);
    let now = env.ledger().timestamp();
    goal.last_price = price;
    goal.last_priced_at = now;
    save_goal(env, &goal);

    let quote = build_quote(env, &goal, &config, price, now);
    events::publish_fiat_goal_quote_event(
        env,
        campaign_id,
        goal.token.clone(),
        goal.currency.clone(),
        price,
        config.price_decimals,
        goal.raised_amount,
        goal.goal_amount,
        quote.remaining_amount,
        quote.tokens_required,
        quote.progress_bps,
        now,
    );

    quote
}

/// Winds a peg down so no further contributions are valued against it.
///
/// Callable by the owning merchant or the contract admin. Whatever the goal had
/// raised stays recorded — closing stops accrual, it does not reset the total —
/// and the closing event carries `goal_reached` so off-chain fulfilment can act
/// on the outcome without a follow-up read.
pub fn close_campaign_fiat_goal(env: &Env, caller: &Address, campaign_id: u64) {
    let mut goal = get_campaign_fiat_goal(env, campaign_id);
    if goal.merchant == *caller {
        caller.require_auth();
    } else {
        core::assert_admin(env, caller);
    }

    if goal.status == FiatGoalStatus::Closed {
        panic_with_error!(env, FiatGoalError::FiatGoalClosed);
    }

    let goal_reached = goal.status == FiatGoalStatus::Reached;
    goal.status = FiatGoalStatus::Closed;
    save_goal(env, &goal);

    events::publish_fiat_goal_closed_event(
        env,
        campaign_id,
        caller.clone(),
        goal.currency.clone(),
        goal.goal_amount,
        goal.raised_amount,
        goal.raised_tokens,
        progress_bps(env, goal.raised_amount, goal.goal_amount),
        goal_reached,
        env.ledger().timestamp(),
    );
}

// ── Read accessors ────────────────────────────────────────────────────────────

fn build_quote(
    env: &Env,
    goal: &CampaignFiatGoal,
    config: &OracleConfig,
    price: i128,
    at: u64,
) -> FiatGoalQuote {
    let remaining_amount = if goal.raised_amount >= goal.goal_amount {
        0
    } else {
        goal.goal_amount - goal.raised_amount
    };

    FiatGoalQuote {
        campaign_id: goal.campaign_id,
        token: goal.token.clone(),
        currency: goal.currency.clone(),
        goal_amount: goal.goal_amount,
        raised_amount: goal.raised_amount,
        remaining_amount,
        price,
        price_decimals: config.price_decimals,
        tokens_required: fiat_to_token(env, remaining_amount, price, goal.decimals, config),
        progress_bps: progress_bps(env, goal.raised_amount, goal.goal_amount),
        status: goal.status,
        quoted_at: at,
    }
}

/// Values `campaign_id`'s peg at the current oracle price without writing
/// anything — the read a UI should render progress from.
pub fn get_campaign_fiat_goal_quote(env: &Env, campaign_id: u64) -> FiatGoalQuote {
    let goal = get_campaign_fiat_goal(env, campaign_id);
    let (config, price) = read_price(env, &goal.token, &goal.currency);
    build_quote(env, &goal, &config, price, env.ledger().timestamp())
}

/// Fiat value of `token_amount` base units at `campaign_id`'s current price, in
/// the goal's minor units. What [`record_fiat_contribution`] would credit,
/// letting a backer preview a contribution before making it.
pub fn quote_fiat_contribution(env: &Env, campaign_id: u64, token_amount: i128) -> i128 {
    if token_amount <= 0 {
        return 0;
    }
    let goal = get_campaign_fiat_goal(env, campaign_id);
    let (config, price) = read_price(env, &goal.token, &goal.currency);
    token_to_fiat(env, token_amount, price, goal.decimals, &config)
}

/// The pegs belonging to `merchant`'s campaigns, in campaign-creation order.
///
/// Walks the merchant's existing campaign index rather than maintaining a
/// second reverse index, so publishing a peg writes one entry, not two.
pub fn get_merchant_fiat_goals(env: &Env, merchant_id: u64) -> Vec<CampaignFiatGoal> {
    let mut goals: Vec<CampaignFiatGoal> = Vec::new(env);
    for campaign in campaigns::get_merchant_campaigns(env, merchant_id).iter() {
        if let Some(goal) = env
            .storage()
            .persistent()
            .get(&FiatGoalKey::CampaignFiatGoal(campaign.id))
        {
            goals.push_back(goal);
        }
    }
    goals
}
