#![cfg(test)]
//! Fiat-pegged campaign goals: a merchant sets the target in fiat and
//! contributions arriving in the campaign token are valued through its price
//! oracle.

use shade::shade::{Shade, ShadeClient};
use shade::types::{OracleConfig};
use crate::from_shade::types::{FiatGoalStatus};
use soroban_sdk::testutils::{Address as _, Events as _, Ledger as _};
use soroban_sdk::{contract, contractimpl, Address, Env, FromVal, String, Symbol, Vec};

/// Ledger time the fixture starts at.
const START: u64 = 1_000;
const DEADLINE: u64 = START + 86_400;

/// $2.00 per whole token, at the oracle's 8 price decimals.
const PRICE: i128 = 200_000_000;
const PRICE_DECIMALS: u32 = 8;
const TOKEN_DECIMALS: u32 = 7;

/// One whole token in base units, at [`TOKEN_DECIMALS`].
const ONE_TOKEN: i128 = 10_000_000;

/// $10,000.00, at [`GOAL_DECIMALS`].
const GOAL: i128 = 1_000_000;
const GOAL_DECIMALS: u32 = 2;

// ── Mock oracle ───────────────────────────────────────────────────────────────

#[contract]
pub struct MockFiatGoalOracle;

#[contractimpl]
impl MockFiatGoalOracle {
    pub fn get_price(env: Env, _token: Address, _quote_currency: String) -> i128 {
        env.storage().instance().get(&"price").unwrap_or(PRICE)
    }

    pub fn set_price(env: Env, price: i128) {
        env.storage().instance().set(&"price", &price);
    }
}

// ── Fixture ───────────────────────────────────────────────────────────────────

struct Fixture<'a> {
    env: Env,
    client: ShadeClient<'a>,
    admin: Address,
    token: Address,
    oracle: Address,
    merchant: Address,
    merchant_id: u64,
    campaign_id: u64,
}

impl Fixture<'_> {
    fn usd(&self) -> String {
        String::from_str(&self.env, "USD")
    }

    /// Pegs the fixture's campaign to $10,000.00.
    fn set_default_goal(&self) {
        self.client.set_campaign_fiat_goal(
            &self.merchant,
            &self.campaign_id,
            &self.usd(),
            &GOAL,
            &GOAL_DECIMALS,
        );
    }

    fn set_price(&self, price: i128) {
        MockFiatGoalOracleClient::new(&self.env, &self.oracle).set_price(&price);
    }

    fn set_time(&self, timestamp: u64) {
        self.env.ledger().with_mut(|l| l.timestamp = timestamp);
    }

    fn contribute(&self, contributor: &Address, token_amount: i128) -> i128 {
        self.client
            .record_fiat_contribution(contributor, &self.campaign_id, &token_amount)
    }

    /// Registers a second campaign owned by the fixture's merchant.
    fn second_campaign(&self, title: &str) -> u64 {
        self.client.create_campaign(
            &self.merchant,
            &String::from_str(&self.env, title),
            &String::from_str(&self.env, "another raise"),
            &1,
            &Vec::new(&self.env),
            &(500 * ONE_TOKEN),
            &self.token,
            &DEADLINE,
        )
    }

    /// How many events carrying `topic` the **most recent** invocation emitted.
    /// The test env clears the buffer per invocation, so this has to be read
    /// before any further client call, including a read-only one.
    fn event_count(&self, topic: &str) -> u32 {
        let wanted = Symbol::new(&self.env, topic);
        let mut count = 0u32;
        for event in self.env.events().all().iter() {
            if let Some(first) = event.1.get(0) {
                if Symbol::from_val(&self.env, &first) == wanted {
                    count += 1;
                }
            }
        }
        count
    }

    fn last_topic(&self) -> Symbol {
        let events = self.env.events().all();
        let last = events.last().unwrap();
        Symbol::from_val(&self.env, &last.1.get(0).unwrap())
    }
}

/// A registered merchant with an active campaign whose token has a live oracle
/// quoting USD at $2.00.
fn setup() -> Fixture<'static> {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = START);

    let contract_id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    client.add_accepted_token(&admin, &token);

    let oracle = env.register(MockFiatGoalOracle, ());
    client.set_token_oracle(
        &admin,
        &token,
        &OracleConfig {
            contract: oracle.clone(),
            price_decimals: PRICE_DECIMALS,
            token_decimals: TOKEN_DECIMALS,
        },
    );

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);
    let merchant_id = client.find_merchant_id(&merchant);

    let category_id = client.create_campaign_category(
        &admin,
        &String::from_str(&env, "Hardware"),
        &String::from_str(&env, "Open hardware projects"),
    );
    let campaign_id = client.create_campaign(
        &merchant,
        &String::from_str(&env, "Open Hardware Rev 2"),
        &String::from_str(&env, "A fiat-pegged raise"),
        &category_id,
        &Vec::new(&env),
        &(5_000 * ONE_TOKEN),
        &token,
        &DEADLINE,
    );

    Fixture {
        env,
        client,
        admin,
        token,
        oracle,
        merchant,
        merchant_id,
        campaign_id,
    }
}

// ── Publishing a peg ──────────────────────────────────────────────────────────

#[test]
fn test_set_goal_stores_terms() {
    let f = setup();
    f.set_default_goal();

    let goal = f.client.get_campaign_fiat_goal(&f.campaign_id);
    assert_eq!(goal.campaign_id, f.campaign_id);
    assert_eq!(goal.merchant, f.merchant);
    assert_eq!(goal.token, f.token);
    assert_eq!(goal.currency, f.usd());
    assert_eq!(goal.goal_amount, GOAL);
    assert_eq!(goal.decimals, GOAL_DECIMALS);
    assert_eq!(goal.raised_amount, 0);
    assert_eq!(goal.raised_tokens, 0);
    assert_eq!(goal.contribution_count, 0);
    assert_eq!(goal.status, FiatGoalStatus::Active);
    assert_eq!(goal.created_at, START);
    assert_eq!(goal.reached_at, 0);
}

#[test]
fn test_set_goal_seeds_the_price_from_the_oracle() {
    let f = setup();
    f.set_default_goal();

    let goal = f.client.get_campaign_fiat_goal(&f.campaign_id);
    assert_eq!(goal.last_price, PRICE);
    assert_eq!(goal.last_priced_at, START);
}

#[test]
fn test_set_goal_emits_event() {
    let f = setup();
    f.set_default_goal();

    assert_eq!(
        f.last_topic(),
        Symbol::new(&f.env, "campaign_fiat_goal_set_event")
    );
}

#[test]
fn test_has_goal_reports_the_peg() {
    let f = setup();
    assert!(!f.client.has_campaign_fiat_goal(&f.campaign_id));
    f.set_default_goal();
    assert!(f.client.has_campaign_fiat_goal(&f.campaign_id));
}

#[test]
#[should_panic(expected = "Error(Contract, #281)")] // FiatGoalAlreadyExists
fn test_campaign_can_only_have_one_peg() {
    let f = setup();
    f.set_default_goal();
    f.set_default_goal();
}

#[test]
#[should_panic(expected = "Error(Contract, #286)")] // NotFiatGoalOwner
fn test_non_owner_cannot_peg_a_campaign() {
    let f = setup();
    let stranger = Address::generate(&f.env);
    f.client.register_merchant(&stranger);

    f.client
        .set_campaign_fiat_goal(&stranger, &f.campaign_id, &f.usd(), &GOAL, &GOAL_DECIMALS);
}

#[test]
#[should_panic(expected = "Error(Contract, #282)")] // InvalidFiatGoalAmount
fn test_peg_target_must_be_positive() {
    let f = setup();
    f.client
        .set_campaign_fiat_goal(&f.merchant, &f.campaign_id, &f.usd(), &0, &GOAL_DECIMALS);
}

#[test]
#[should_panic(expected = "Error(Contract, #283)")] // InvalidFiatCurrency
fn test_currency_cannot_be_empty() {
    let f = setup();
    f.client.set_campaign_fiat_goal(
        &f.merchant,
        &f.campaign_id,
        &String::from_str(&f.env, ""),
        &GOAL,
        &GOAL_DECIMALS,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #283)")] // InvalidFiatCurrency
fn test_currency_is_length_capped() {
    let f = setup();
    f.client.set_campaign_fiat_goal(
        &f.merchant,
        &f.campaign_id,
        &String::from_str(&f.env, "NOT-A-CURRENCY-CODE"),
        &GOAL,
        &GOAL_DECIMALS,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #284)")] // InvalidFiatDecimals
fn test_decimals_are_capped() {
    let f = setup();
    f.client
        .set_campaign_fiat_goal(&f.merchant, &f.campaign_id, &f.usd(), &GOAL, &19);
}

#[test]
#[should_panic(expected = "Error(Contract, #34)")] // OracleNotConfigured
fn test_cannot_peg_a_token_with_no_oracle() {
    let f = setup();
    let other_token = env_token(&f);
    let campaign_id = f.client.create_campaign(
        &f.merchant,
        &String::from_str(&f.env, "Unpriced"),
        &String::from_str(&f.env, "no oracle for this token"),
        &1,
        &Vec::new(&f.env),
        &(100 * ONE_TOKEN),
        &other_token,
        &DEADLINE,
    );

    f.client
        .set_campaign_fiat_goal(&f.merchant, &campaign_id, &f.usd(), &GOAL, &GOAL_DECIMALS);
}

/// A second accepted token, deliberately left without an oracle.
fn env_token(f: &Fixture<'_>) -> Address {
    let issuer = Address::generate(&f.env);
    let token = f.env.register_stellar_asset_contract_v2(issuer).address();
    f.client.add_accepted_token(&f.admin, &token);
    token
}

#[test]
#[should_panic(expected = "Error(Contract, #35)")] // OraclePriceUnavailable
fn test_cannot_peg_when_the_oracle_has_no_price() {
    let f = setup();
    f.set_price(0);
    f.set_default_goal();
}

#[test]
#[should_panic(expected = "Error(Contract, #210)")] // CampaignExpired
fn test_cannot_peg_an_expired_campaign() {
    let f = setup();
    f.set_time(DEADLINE + 1);
    f.set_default_goal();
}

#[test]
#[should_panic(expected = "Error(Contract, #208)")] // CampaignInactive
fn test_cannot_peg_a_deactivated_campaign() {
    let f = setup();
    f.client
        .set_campaign_active(&f.merchant, &f.campaign_id, &false);
    f.set_default_goal();
}

#[test]
#[should_panic(expected = "Error(Contract, #200)")] // CampaignNotFound
fn test_cannot_peg_an_unknown_campaign() {
    let f = setup();
    f.client
        .set_campaign_fiat_goal(&f.merchant, &9_999, &f.usd(), &GOAL, &GOAL_DECIMALS);
}

// ── Valuing contributions ─────────────────────────────────────────────────────

#[test]
fn test_contribution_is_valued_at_the_oracle_price() {
    let f = setup();
    f.set_default_goal();
    let backer = Address::generate(&f.env);

    // 100 tokens at $2.00 = $200.00.
    let credited = f.contribute(&backer, 100 * ONE_TOKEN);
    assert_eq!(credited, 20_000);

    let goal = f.client.get_campaign_fiat_goal(&f.campaign_id);
    assert_eq!(goal.raised_amount, 20_000);
    assert_eq!(goal.raised_tokens, 100 * ONE_TOKEN);
    assert_eq!(goal.contribution_count, 1);
    assert_eq!(goal.status, FiatGoalStatus::Active);
}

#[test]
fn test_contribution_also_advances_the_token_total() {
    let f = setup();
    f.set_default_goal();
    let backer = Address::generate(&f.env);
    f.contribute(&backer, 100 * ONE_TOKEN);

    let campaign = f.client.get_campaign(&f.campaign_id);
    assert_eq!(campaign.raised_amount, 100 * ONE_TOKEN);
}

#[test]
fn test_contribution_tracks_per_backer_fiat() {
    let f = setup();
    f.set_default_goal();
    let backer = Address::generate(&f.env);
    let other = Address::generate(&f.env);

    f.contribute(&backer, 100 * ONE_TOKEN);
    f.contribute(&backer, 50 * ONE_TOKEN);
    f.contribute(&other, 10 * ONE_TOKEN);

    assert_eq!(
        f.client
            .get_backer_fiat_contribution(&f.campaign_id, &backer),
        30_000
    );
    assert_eq!(
        f.client
            .get_backer_fiat_contribution(&f.campaign_id, &other),
        2_000
    );
}

#[test]
fn test_untracked_backer_reads_zero() {
    let f = setup();
    f.set_default_goal();
    let stranger = Address::generate(&f.env);

    assert_eq!(
        f.client
            .get_backer_fiat_contribution(&f.campaign_id, &stranger),
        0
    );
}

#[test]
fn test_contribution_emits_event() {
    let f = setup();
    f.set_default_goal();
    let backer = Address::generate(&f.env);
    f.contribute(&backer, 100 * ONE_TOKEN);

    assert_eq!(f.event_count("fiat_contribution_event"), 1);
}

#[test]
fn test_each_contribution_keeps_the_price_it_landed_at() {
    let f = setup();
    f.set_default_goal();
    let backer = Address::generate(&f.env);

    // $200.00 at $2.00 per token.
    f.contribute(&backer, 100 * ONE_TOKEN);
    // The token halves; the same 100 tokens are now only worth $100.00, but the
    // first contribution keeps the value it was credited at.
    f.set_price(PRICE / 2);
    f.contribute(&backer, 100 * ONE_TOKEN);

    let goal = f.client.get_campaign_fiat_goal(&f.campaign_id);
    assert_eq!(goal.raised_amount, 30_000);
    assert_eq!(goal.raised_tokens, 200 * ONE_TOKEN);
    assert_eq!(goal.last_price, PRICE / 2);
    assert_eq!(goal.last_priced_at, START);
}

#[test]
fn test_repeated_contributions_in_one_ledger_all_count() {
    let f = setup();
    f.set_default_goal();
    let backer = Address::generate(&f.env);

    for _ in 0..5 {
        f.contribute(&backer, 10 * ONE_TOKEN);
    }

    let goal = f.client.get_campaign_fiat_goal(&f.campaign_id);
    assert_eq!(goal.contribution_count, 5);
    assert_eq!(goal.raised_amount, 10_000);
    assert_eq!(goal.raised_tokens, 50 * ONE_TOKEN);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")] // InvalidAmount
fn test_contribution_must_be_positive() {
    let f = setup();
    f.set_default_goal();
    let backer = Address::generate(&f.env);
    f.contribute(&backer, 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #287)")] // FiatValueTooSmall
fn test_dust_worth_less_than_a_cent_is_rejected() {
    let f = setup();
    f.set_default_goal();
    let backer = Address::generate(&f.env);

    // At $2.00 a token, one base unit is worth $0.0000002 — nothing at two
    // fractional digits.
    f.contribute(&backer, 1);
}

#[test]
#[should_panic(expected = "Error(Contract, #280)")] // FiatGoalNotFound
fn test_cannot_contribute_to_an_unpegged_campaign() {
    let f = setup();
    let backer = Address::generate(&f.env);
    f.contribute(&backer, ONE_TOKEN);
}

#[test]
#[should_panic(expected = "Error(Contract, #210)")] // CampaignExpired
fn test_cannot_contribute_after_the_deadline() {
    let f = setup();
    f.set_default_goal();
    f.set_time(DEADLINE + 1);

    let backer = Address::generate(&f.env);
    f.contribute(&backer, ONE_TOKEN);
}

#[test]
#[should_panic(expected = "Error(Contract, #208)")] // CampaignInactive
fn test_cannot_contribute_to_a_deactivated_campaign() {
    let f = setup();
    f.set_default_goal();
    f.client
        .set_campaign_active(&f.merchant, &f.campaign_id, &false);

    let backer = Address::generate(&f.env);
    f.contribute(&backer, ONE_TOKEN);
}

#[test]
#[should_panic(expected = "Error(Contract, #35)")] // OraclePriceUnavailable
fn test_cannot_contribute_while_the_price_feed_is_down() {
    let f = setup();
    f.set_default_goal();
    f.set_price(0);

    let backer = Address::generate(&f.env);
    f.contribute(&backer, ONE_TOKEN);
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")] // ContractPaused
fn test_contribution_is_blocked_while_paused() {
    let f = setup();
    f.set_default_goal();
    f.client.pause(&f.admin);

    let backer = Address::generate(&f.env);
    f.contribute(&backer, ONE_TOKEN);
}

// ── Reaching the target ───────────────────────────────────────────────────────

#[test]
fn test_goal_is_reached_when_fiat_target_is_met() {
    let f = setup();
    f.set_default_goal();
    let backer = Address::generate(&f.env);

    // 5,000 tokens at $2.00 is exactly $10,000.00.
    f.contribute(&backer, 5_000 * ONE_TOKEN);
    assert_eq!(f.event_count("fiat_goal_reached_event"), 1);

    let goal = f.client.get_campaign_fiat_goal(&f.campaign_id);
    assert_eq!(goal.raised_amount, GOAL);
    assert_eq!(goal.status, FiatGoalStatus::Reached);
    assert_eq!(goal.reached_at, START);
}

#[test]
fn test_a_price_rise_reaches_the_target_on_fewer_tokens() {
    let f = setup();
    f.set_default_goal();
    let backer = Address::generate(&f.env);

    // $4.00 a token: 2,500 tokens now cover the whole $10,000.00 target.
    f.set_price(PRICE * 2);
    f.contribute(&backer, 2_500 * ONE_TOKEN);

    let goal = f.client.get_campaign_fiat_goal(&f.campaign_id);
    assert_eq!(goal.raised_amount, GOAL);
    assert_eq!(goal.status, FiatGoalStatus::Reached);
    assert_eq!(goal.raised_tokens, 2_500 * ONE_TOKEN);
}

#[test]
fn test_reached_event_fires_once_even_when_overfunded() {
    let f = setup();
    f.set_default_goal();
    let backer = Address::generate(&f.env);

    f.contribute(&backer, 5_000 * ONE_TOKEN);
    assert_eq!(f.event_count("fiat_goal_reached_event"), 1);

    f.contribute(&backer, 1_000 * ONE_TOKEN);
    // The top-up is still recorded, but the milestone does not fire again.
    assert_eq!(f.event_count("fiat_contribution_event"), 1);
    assert_eq!(f.event_count("fiat_goal_reached_event"), 0);

    let goal = f.client.get_campaign_fiat_goal(&f.campaign_id);
    assert_eq!(goal.raised_amount, GOAL + 200_000);
    assert_eq!(goal.status, FiatGoalStatus::Reached);
    // The instant it was first met, not the instant of the latest top-up.
    assert_eq!(goal.reached_at, START);
}

#[test]
fn test_a_later_price_crash_cannot_unreach_the_target() {
    let f = setup();
    f.set_default_goal();
    let backer = Address::generate(&f.env);
    f.contribute(&backer, 5_000 * ONE_TOKEN);

    // The token loses 90% of its value after the raise closed the target.
    f.set_price(PRICE / 10);
    f.set_time(START + 500);

    let goal = f.client.get_campaign_fiat_goal(&f.campaign_id);
    assert_eq!(goal.status, FiatGoalStatus::Reached);
    assert_eq!(goal.raised_amount, GOAL);

    let quote = f.client.get_campaign_fiat_goal_quote(&f.campaign_id);
    assert_eq!(quote.remaining_amount, 0);
    assert_eq!(quote.tokens_required, 0);
    assert_eq!(quote.progress_bps, 10_000);
}

// ── Quoting ───────────────────────────────────────────────────────────────────

#[test]
fn test_quote_reports_the_shortfall_in_tokens() {
    let f = setup();
    f.set_default_goal();

    let quote = f.client.get_campaign_fiat_goal_quote(&f.campaign_id);
    assert_eq!(quote.campaign_id, f.campaign_id);
    assert_eq!(quote.token, f.token);
    assert_eq!(quote.currency, f.usd());
    assert_eq!(quote.goal_amount, GOAL);
    assert_eq!(quote.raised_amount, 0);
    assert_eq!(quote.remaining_amount, GOAL);
    assert_eq!(quote.price, PRICE);
    assert_eq!(quote.price_decimals, PRICE_DECIMALS);
    // $10,000.00 at $2.00 per token.
    assert_eq!(quote.tokens_required, 5_000 * ONE_TOKEN);
    assert_eq!(quote.progress_bps, 0);
    assert_eq!(quote.status, FiatGoalStatus::Active);
    assert_eq!(quote.quoted_at, START);
}

#[test]
fn test_quote_follows_the_price() {
    let f = setup();
    f.set_default_goal();

    // Halving the price doubles the tokens the same target costs.
    f.set_price(PRICE / 2);
    let quote = f.client.get_campaign_fiat_goal_quote(&f.campaign_id);
    assert_eq!(quote.tokens_required, 10_000 * ONE_TOKEN);
}

#[test]
fn test_quote_tracks_progress_in_basis_points() {
    let f = setup();
    f.set_default_goal();
    let backer = Address::generate(&f.env);

    // A quarter of $10,000.00.
    f.contribute(&backer, 1_250 * ONE_TOKEN);

    let quote = f.client.get_campaign_fiat_goal_quote(&f.campaign_id);
    assert_eq!(quote.raised_amount, 250_000);
    assert_eq!(quote.remaining_amount, 750_000);
    assert_eq!(quote.progress_bps, 2_500);
    assert_eq!(quote.tokens_required, 3_750 * ONE_TOKEN);
}

#[test]
fn test_quote_rounds_the_shortfall_up() {
    let f = setup();
    f.set_default_goal();
    let backer = Address::generate(&f.env);
    f.contribute(&backer, ONE_TOKEN);

    // $3.00 a token leaves $9,998.00 outstanding, which is not a whole number
    // of base units — the quote must round up, never leaving the goal short.
    f.set_price(300_000_000);
    let quote = f.client.get_campaign_fiat_goal_quote(&f.campaign_id);
    assert_eq!(quote.remaining_amount, 999_800);
    assert_eq!(quote.tokens_required, 33_326_666_667);

    let credited = f.contribute(&backer, quote.tokens_required);
    assert!(credited >= quote.remaining_amount);
    assert_eq!(
        f.client.get_campaign_fiat_goal(&f.campaign_id).status,
        FiatGoalStatus::Reached
    );
}

#[test]
fn test_quote_is_read_only() {
    let f = setup();
    f.set_default_goal();
    f.set_price(PRICE * 3);

    f.client.get_campaign_fiat_goal_quote(&f.campaign_id);

    let goal = f.client.get_campaign_fiat_goal(&f.campaign_id);
    assert_eq!(goal.last_price, PRICE);
    assert_eq!(goal.last_priced_at, START);
}

#[test]
fn test_contribution_preview_matches_what_gets_credited() {
    let f = setup();
    f.set_default_goal();
    let backer = Address::generate(&f.env);

    let preview = f
        .client
        .quote_fiat_contribution(&f.campaign_id, &(37 * ONE_TOKEN));
    let credited = f.contribute(&backer, 37 * ONE_TOKEN);
    assert_eq!(preview, credited);
    assert_eq!(preview, 7_400);
}

#[test]
fn test_contribution_preview_of_nothing_is_zero() {
    let f = setup();
    f.set_default_goal();
    assert_eq!(f.client.quote_fiat_contribution(&f.campaign_id, &0), 0);
}

// ── Refreshing the on-ledger quote ────────────────────────────────────────────

#[test]
fn test_refresh_records_the_new_price_and_emits() {
    let f = setup();
    f.set_default_goal();
    f.set_price(PRICE * 2);
    f.set_time(START + 60);

    let quote = f
        .client
        .refresh_campaign_fiat_quote(&f.merchant, &f.campaign_id);
    assert_eq!(f.event_count("fiat_goal_quote_event"), 1);
    assert_eq!(quote.price, PRICE * 2);
    assert_eq!(quote.tokens_required, 2_500 * ONE_TOKEN);
    assert_eq!(quote.quoted_at, START + 60);

    let goal = f.client.get_campaign_fiat_goal(&f.campaign_id);
    assert_eq!(goal.last_price, PRICE * 2);
    assert_eq!(goal.last_priced_at, START + 60);
}

#[test]
fn test_refresh_leaves_the_raise_untouched() {
    let f = setup();
    f.set_default_goal();
    let backer = Address::generate(&f.env);
    f.contribute(&backer, 100 * ONE_TOKEN);

    f.set_price(PRICE * 5);
    f.client
        .refresh_campaign_fiat_quote(&f.merchant, &f.campaign_id);

    let goal = f.client.get_campaign_fiat_goal(&f.campaign_id);
    assert_eq!(goal.raised_amount, 20_000);
    assert_eq!(goal.raised_tokens, 100 * ONE_TOKEN);
    assert_eq!(goal.contribution_count, 1);
    assert_eq!(goal.status, FiatGoalStatus::Active);
}

#[test]
#[should_panic(expected = "Error(Contract, #286)")] // NotFiatGoalOwner
fn test_non_owner_cannot_refresh() {
    let f = setup();
    f.set_default_goal();
    let stranger = Address::generate(&f.env);
    f.client
        .refresh_campaign_fiat_quote(&stranger, &f.campaign_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #280)")] // FiatGoalNotFound
fn test_cannot_refresh_an_unpegged_campaign() {
    let f = setup();
    f.client
        .refresh_campaign_fiat_quote(&f.merchant, &f.campaign_id);
}

// ── Closing ───────────────────────────────────────────────────────────────────

#[test]
fn test_merchant_can_close_a_peg() {
    let f = setup();
    f.set_default_goal();
    let backer = Address::generate(&f.env);
    f.contribute(&backer, 100 * ONE_TOKEN);

    f.client
        .close_campaign_fiat_goal(&f.merchant, &f.campaign_id);
    assert_eq!(f.event_count("fiat_goal_closed_event"), 1);

    let goal = f.client.get_campaign_fiat_goal(&f.campaign_id);
    assert_eq!(goal.status, FiatGoalStatus::Closed);
    // Closing stops accrual; it does not erase what was raised.
    assert_eq!(goal.raised_amount, 20_000);
    assert_eq!(goal.raised_tokens, 100 * ONE_TOKEN);
}

#[test]
fn test_admin_can_close_a_peg() {
    let f = setup();
    f.set_default_goal();

    f.client.close_campaign_fiat_goal(&f.admin, &f.campaign_id);
    assert_eq!(
        f.client.get_campaign_fiat_goal(&f.campaign_id).status,
        FiatGoalStatus::Closed
    );
}

#[test]
fn test_closing_a_reached_peg_keeps_the_outcome() {
    let f = setup();
    f.set_default_goal();
    let backer = Address::generate(&f.env);
    f.contribute(&backer, 5_000 * ONE_TOKEN);

    f.client
        .close_campaign_fiat_goal(&f.merchant, &f.campaign_id);

    let goal = f.client.get_campaign_fiat_goal(&f.campaign_id);
    assert_eq!(goal.status, FiatGoalStatus::Closed);
    assert_eq!(goal.reached_at, START);
    assert_eq!(goal.raised_amount, GOAL);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")] // NotAuthorized
fn test_stranger_cannot_close_a_peg() {
    let f = setup();
    f.set_default_goal();
    let stranger = Address::generate(&f.env);
    f.client.close_campaign_fiat_goal(&stranger, &f.campaign_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #285)")] // FiatGoalClosed
fn test_a_peg_cannot_be_closed_twice() {
    let f = setup();
    f.set_default_goal();
    f.client
        .close_campaign_fiat_goal(&f.merchant, &f.campaign_id);
    f.client
        .close_campaign_fiat_goal(&f.merchant, &f.campaign_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #285)")] // FiatGoalClosed
fn test_closed_peg_rejects_contributions() {
    let f = setup();
    f.set_default_goal();
    f.client
        .close_campaign_fiat_goal(&f.merchant, &f.campaign_id);

    let backer = Address::generate(&f.env);
    f.contribute(&backer, ONE_TOKEN);
}

#[test]
#[should_panic(expected = "Error(Contract, #285)")] // FiatGoalClosed
fn test_closed_peg_rejects_refresh() {
    let f = setup();
    f.set_default_goal();
    f.client
        .close_campaign_fiat_goal(&f.merchant, &f.campaign_id);
    f.client
        .refresh_campaign_fiat_quote(&f.merchant, &f.campaign_id);
}

// ── Merchant-wide listing ─────────────────────────────────────────────────────

#[test]
fn test_merchant_goals_list_pegged_campaigns_in_order() {
    let f = setup();
    f.set_default_goal();

    let second = f.second_campaign("Rev 3");
    f.client
        .set_campaign_fiat_goal(&f.merchant, &second, &f.usd(), &(GOAL * 2), &GOAL_DECIMALS);

    let goals = f.client.get_merchant_fiat_goals(&f.merchant_id);
    assert_eq!(goals.len(), 2);
    assert_eq!(goals.get(0).unwrap().campaign_id, f.campaign_id);
    assert_eq!(goals.get(1).unwrap().campaign_id, second);
    assert_eq!(goals.get(1).unwrap().goal_amount, GOAL * 2);
}

#[test]
fn test_merchant_goals_skip_unpegged_campaigns() {
    let f = setup();
    f.set_default_goal();
    f.second_campaign("Unpegged");

    let goals = f.client.get_merchant_fiat_goals(&f.merchant_id);
    assert_eq!(goals.len(), 1);
    assert_eq!(goals.get(0).unwrap().campaign_id, f.campaign_id);
}

#[test]
fn test_merchant_with_no_pegs_lists_nothing() {
    let f = setup();
    let goals = f.client.get_merchant_fiat_goals(&f.merchant_id);
    assert_eq!(goals.len(), 0);
}
