//! Vesting for crowdfunding payouts.
//!
//! Two independent facilities live here:
//!
//! - **Admin vesting timelines** (below): a registry of reusable cliff/duration
//!   templates and manually-scheduled tranches, managed by the contract admin.
//! - **Creator fund vesting** ([`create_creator_vesting`] onward): a campaign
//!   creator commits the funds their backer campaign raised to a cliff-plus-
//!   linear schedule and draws them down over time instead of withdrawing the
//!   whole raise at once. Backers get a public, immutable payout calendar.
//!
//! # Creator vesting invariants
//!
//! - Only the campaign's owning merchant may create a schedule or release from
//!   it; only the contract admin may revoke one.
//! - A campaign has at most one schedule, and its terms never change once
//!   created — backers can rely on what was published. Revocation is the single
//!   exception and is admin-only.
//! - `released_amount` only ever grows, and never past the amount vested at the
//!   current ledger timestamp. Two releases in the same instant therefore cannot
//!   pay out twice: the second sees the first's write and finds nothing vested.
//! - A schedule may commit no more than the campaign has actually raised, so the
//!   contract always holds the tokens it has promised.
//!
//! # Storage
//!
//! Keys live in [`VestingKey`], a dedicated enum, so this feature adds no cases
//! to the near-full `CampaignKey` (Soroban caps every enum at 50 cases). A
//! schedule is keyed by its `campaign_id` rather than a fresh ID, which avoids
//! a counter entry entirely and keeps lookups to one read.

use crate::from_shade::backer_rewards;
use shade::components::core;
use crate::from_shade::errors::{ContractError, VestingError};
use crate::from_shade::events;
use crate::from_shade::types::{CampaignKey, CreatorVesting, CreatorVestingStatus, CrowdfundVestingConfig, VestingKey, VestingSchedule, VestingTimeline};
use soroban_sdk::{panic_with_error, token, Address, Env, String, Vec};

/// Basis-point denominator; 10_000 bps = 100%.
const BPS_DENOMINATOR: i128 = 10_000;

/// Upper bound on concurrent schedules per creator. Bounds the reverse-index
/// entry so its reads and writes stay within a predictable rent/CPU budget.
const MAX_SCHEDULES_PER_CREATOR: u32 = 64;

// ── Admin vesting timelines ───────────────────────────────────────────────────

fn get_vesting_timeline_count(env: &Env) -> u64 {
    env.storage()
        .persistent()
        .get(&CampaignKey::VestingTimelineCount)
        .unwrap_or(0)
}

fn set_vesting_timeline_count(env: &Env, count: u64) {
    env.storage()
        .persistent()
        .set(&CampaignKey::VestingTimelineCount, &count);
}

pub fn create_vesting_timeline(
    env: &Env,
    admin: Address,
    name: String,
    cliff_duration: u64,
    vesting_duration: u64,
    unlock_percentage: i128,
) -> u64 {
    admin.require_auth();
    let contract_admin = core::get_admin(env);
    if admin != contract_admin {
        panic_with_error!(env, shade::errors::ContractError::NotAuthorized);
    }

    if cliff_duration == 0 || vesting_duration == 0 {
        panic_with_error!(env, ContractError::InvalidInterval);
    }
    if unlock_percentage <= 0 || unlock_percentage > 10000 {
        panic_with_error!(env, shade::errors::ContractError::InvalidAmount);
    }

    let timeline_id = get_vesting_timeline_count(env) + 1;
    set_vesting_timeline_count(env, timeline_id);

    let now = env.ledger().timestamp();
    let timeline = VestingTimeline {
        id: timeline_id,
        name: name.clone(),
        cliff_duration,
        vesting_duration,
        unlock_percentage,
        admin: admin.clone(),
        created_at: now,
    };

    env.storage()
        .persistent()
        .set(&CampaignKey::VestingTimeline(timeline_id), &timeline);

    events::publish_vesting_timeline_created_event(
        env,
        timeline_id,
        name,
        cliff_duration,
        vesting_duration,
        admin,
        now,
    );

    timeline_id
}

pub fn get_vesting_timeline(env: &Env, timeline_id: u64) -> VestingTimeline {
    env.storage()
        .persistent()
        .get(&CampaignKey::VestingTimeline(timeline_id))
        .unwrap_or_else(|| panic_with_error!(env, ContractError::InvalidInterval))
}

pub fn update_vesting_timeline(
    env: &Env,
    admin: Address,
    timeline_id: u64,
    cliff_duration: u64,
    vesting_duration: u64,
) {
    admin.require_auth();
    let contract_admin = core::get_admin(env);
    if admin != contract_admin {
        panic_with_error!(env, shade::errors::ContractError::NotAuthorized);
    }

    if cliff_duration == 0 || vesting_duration == 0 {
        panic_with_error!(env, ContractError::InvalidInterval);
    }

    let mut timeline = get_vesting_timeline(env, timeline_id);
    timeline.cliff_duration = cliff_duration;
    timeline.vesting_duration = vesting_duration;

    env.storage()
        .persistent()
        .set(&CampaignKey::VestingTimeline(timeline_id), &timeline);

    let now = env.ledger().timestamp();
    events::publish_vesting_timeline_updated_event(
        env,
        timeline_id,
        cliff_duration,
        vesting_duration,
        admin,
        now,
    );
}

pub fn configure_crowdfund_vesting(
    env: &Env,
    admin: Address,
    crowdfund_id: u64,
    timeline_id: u64,
    total_vesting_amount: i128,
) {
    admin.require_auth();
    let contract_admin = core::get_admin(env);
    if admin != contract_admin {
        panic_with_error!(env, shade::errors::ContractError::NotAuthorized);
    }

    if total_vesting_amount <= 0 {
        panic_with_error!(env, shade::errors::ContractError::InvalidAmount);
    }

    let _timeline = get_vesting_timeline(env, timeline_id);

    let config = CrowdfundVestingConfig {
        crowdfund_id,
        timeline_id,
        total_vesting_amount,
        configured_at: env.ledger().timestamp(),
    };

    env.storage()
        .persistent()
        .set(&CampaignKey::CrowdfundVestingConfig(crowdfund_id), &config);

    let now = env.ledger().timestamp();
    events::publish_crowdfund_vesting_configured_event(
        env,
        crowdfund_id,
        timeline_id,
        total_vesting_amount,
        admin,
        now,
    );
}

pub fn get_crowdfund_vesting_config(env: &Env, crowdfund_id: u64) -> CrowdfundVestingConfig {
    env.storage()
        .persistent()
        .get(&CampaignKey::CrowdfundVestingConfig(crowdfund_id))
        .unwrap_or_else(|| panic_with_error!(env, shade::errors::ContractError::InvoiceNotFound))
}

pub fn get_vesting_schedule(env: &Env, timeline_id: u64, tranche_index: u64) -> VestingSchedule {
    env.storage()
        .persistent()
        .get(&DataKey::VestingSchedule(timeline_id, tranche_index))
        .unwrap_or_else(|| panic_with_error!(env, shade::errors::ContractError::InvoiceNotFound))
}

pub fn add_vesting_schedule(
    env: &Env,
    admin: Address,
    timeline_id: u64,
    tranche_index: u64,
    unlock_amount: i128,
    unlock_timestamp: u64,
) {
    admin.require_auth();
    let contract_admin = core::get_admin(env);
    if admin != contract_admin {
        panic_with_error!(env, shade::errors::ContractError::NotAuthorized);
    }

    if unlock_amount <= 0 {
        panic_with_error!(env, shade::errors::ContractError::InvalidAmount);
    }

    let _timeline = get_vesting_timeline(env, timeline_id);

    let schedule = VestingSchedule {
        timeline_id,
        tranche_index,
        unlock_amount,
        unlock_timestamp,
        released: false,
    };

    env.storage().persistent().set(
        &CampaignKey::VestingSchedule(timeline_id, tranche_index),
        &schedule,
    );
}

pub fn release_vesting_schedule(env: &Env, admin: Address, timeline_id: u64, tranche_index: u64) {
    admin.require_auth();
    let contract_admin = core::get_admin(env);
    if admin != contract_admin {
        panic_with_error!(env, shade::errors::ContractError::NotAuthorized);
    }

    let mut schedule = env
        .storage()
        .persistent()
        .get::<_, VestingSchedule>(&CampaignKey::VestingSchedule(timeline_id, tranche_index))
        .unwrap_or_else(|| panic_with_error!(env, shade::errors::ContractError::InvoiceNotFound));

    if schedule.released {
        panic_with_error!(env, shade::errors::ContractError::InvalidInvoiceStatus);
    }

    let now = env.ledger().timestamp();
    if now < schedule.unlock_timestamp {
        panic_with_error!(env, shade::errors::ContractError::InvoiceExpired);
    }

    schedule.released = true;
    env.storage().persistent().set(
        &CampaignKey::VestingSchedule(timeline_id, tranche_index),
        &schedule,
    );

    events::publish_vesting_schedule_released_event(
        env,
        timeline_id,
        tranche_index,
        schedule.unlock_amount,
        now,
    );
}

// ── Creator fund vesting ──────────────────────────────────────────────────────

fn save_creator_vesting(env: &Env, vesting: &CreatorVesting) {
    env.storage()
        .persistent()
        .set(&VestingKey::CreatorVesting(vesting.campaign_id), vesting);
}

/// Loads a campaign's vesting schedule, panicking with
/// [`VestingError::VestingNotFound`] if it has none.
pub fn get_creator_vesting(env: &Env, campaign_id: u64) -> CreatorVesting {
    env.storage()
        .persistent()
        .get(&VestingKey::CreatorVesting(campaign_id))
        .unwrap_or_else(|| panic_with_error!(env, VestingError::VestingNotFound))
}

/// Campaign IDs this creator vests funds from, in creation order.
pub fn get_creator_vesting_campaigns(env: &Env, creator: Address) -> Vec<u64> {
    env.storage()
        .persistent()
        .get(&VestingKey::CreatorVestingList(creator))
        .unwrap_or_else(|| Vec::new(env))
}

/// `a * b / d`, panicking rather than wrapping if the product overflows.
/// Callers guarantee `d != 0`.
fn mul_div(env: &Env, a: i128, b: i128, d: i128) -> i128 {
    a.checked_mul(b)
        .unwrap_or_else(|| panic_with_error!(env, shade::errors::ContractError::InvalidAmount))
        / d
}

/// Amount vested as of `at`: nothing before the cliff, `initial_unlock_bps` in a
/// lump at it, then the remainder straight-lined to `start_time +
/// vesting_duration`.
///
/// Monotonically non-decreasing in `at`, which is what makes releases safe to
/// compute as `vested - released`.
pub fn vested_amount_at(env: &Env, vesting: &CreatorVesting, at: u64) -> i128 {
    // A revoked schedule had its total lowered to whatever was vested at the
    // moment of revocation, so that total is now fully vested and frozen.
    if vesting.status == CreatorVestingStatus::Revoked {
        return vesting.total_amount;
    }

    let cliff_timestamp = vesting.start_time.saturating_add(vesting.cliff_duration);
    if at < cliff_timestamp {
        return 0;
    }

    let end_timestamp = vesting.start_time.saturating_add(vesting.vesting_duration);
    if at >= end_timestamp {
        return vesting.total_amount;
    }

    // Reaching here means cliff_timestamp < end_timestamp, so the linear window
    // below is non-zero.
    let initial_unlock = mul_div(
        env,
        vesting.total_amount,
        i128::from(vesting.initial_unlock_bps),
        BPS_DENOMINATOR,
    );
    let linear_pool = vesting.total_amount - initial_unlock;
    let window = i128::from(vesting.vesting_duration - vesting.cliff_duration);
    let elapsed = i128::from(at - cliff_timestamp);

    initial_unlock + mul_div(env, linear_pool, elapsed, window)
}

/// Amount vested as of the current ledger timestamp, released or not.
pub fn get_vested_amount(env: &Env, campaign_id: u64) -> i128 {
    let vesting = get_creator_vesting(env, campaign_id);
    vested_amount_at(env, &vesting, env.ledger().timestamp())
}

/// Amount the creator could release right now.
pub fn get_releasable_amount(env: &Env, campaign_id: u64) -> i128 {
    let vesting = get_creator_vesting(env, campaign_id);
    vested_amount_at(env, &vesting, env.ledger().timestamp()) - vesting.released_amount
}

/// Commits a backer campaign's raised funds to a vesting schedule paying out to
/// the creator.
///
/// `total_amount` may not exceed what the campaign has raised, so the contract
/// always holds the tokens the schedule promises. The terms are fixed at
/// creation: there is deliberately no update path, so backers can rely on the
/// published calendar. Callable only by the campaign's owning merchant.
pub fn create_creator_vesting(
    env: &Env,
    creator: Address,
    campaign_id: u64,
    total_amount: i128,
    start_time: u64,
    cliff_duration: u64,
    vesting_duration: u64,
    initial_unlock_bps: u32,
) {
    creator.require_auth();

    let campaign = backer_rewards::assert_backer_campaign_owner(env, campaign_id, &creator);

    if env
        .storage()
        .persistent()
        .has(&VestingKey::CreatorVesting(campaign_id))
    {
        panic_with_error!(env, VestingError::VestingAlreadyExists);
    }

    if total_amount <= 0 {
        panic_with_error!(env, shade::errors::ContractError::InvalidAmount);
    }
    if total_amount > campaign.raised {
        panic_with_error!(env, VestingError::VestingAmountExceedsRaised);
    }
    if vesting_duration == 0 || cliff_duration > vesting_duration {
        panic_with_error!(env, VestingError::InvalidVestingDuration);
    }
    if i128::from(initial_unlock_bps) > BPS_DENOMINATOR {
        panic_with_error!(env, VestingError::InvalidUnlockBps);
    }

    let now = env.ledger().timestamp();
    // Backdating would unlock funds the moment the schedule is published,
    // defeating the commitment it makes to backers.
    if start_time < now {
        panic_with_error!(env, VestingError::InvalidVestingStart);
    }

    let mut campaigns = get_creator_vesting_campaigns(env, creator.clone());
    if campaigns.len() >= MAX_SCHEDULES_PER_CREATOR {
        panic_with_error!(env, VestingError::TooManyVestingSchedules);
    }

    let vesting = CreatorVesting {
        campaign_id,
        creator: creator.clone(),
        token: campaign.token.clone(),
        total_amount,
        released_amount: 0,
        start_time,
        cliff_duration,
        vesting_duration,
        initial_unlock_bps,
        status: CreatorVestingStatus::Active,
        created_at: now,
        last_release_at: 0,
    };
    save_creator_vesting(env, &vesting);

    campaigns.push_back(campaign_id);
    env.storage()
        .persistent()
        .set(&VestingKey::CreatorVestingList(creator.clone()), &campaigns);

    let initial_unlock_amount = mul_div(
        env,
        total_amount,
        i128::from(initial_unlock_bps),
        BPS_DENOMINATOR,
    );
    events::publish_creator_vesting_created_event(
        env,
        campaign_id,
        creator,
        campaign.token,
        total_amount,
        start_time,
        start_time.saturating_add(cliff_duration),
        start_time.saturating_add(vesting_duration),
        initial_unlock_bps,
        initial_unlock_amount,
        campaign.raised,
        now,
    );
}

/// Pays the creator everything that has vested since their last release, and
/// returns that amount.
///
/// The schedule is written back *before* the token transfer, so a token that
/// re-enters the contract mid-transfer finds `released_amount` already advanced
/// and nothing further vested. Two calls at the same timestamp behave the same
/// way: the second finds nothing releasable and reverts.
pub fn release_creator_vesting(env: &Env, creator: Address, campaign_id: u64) -> i128 {
    creator.require_auth();

    let mut vesting = get_creator_vesting(env, campaign_id);
    if vesting.creator != creator {
        panic_with_error!(env, VestingError::NotVestingBeneficiary);
    }
    if vesting.status == CreatorVestingStatus::Completed {
        panic_with_error!(env, VestingError::VestingCompleted);
    }

    let now = env.ledger().timestamp();
    let vested = vested_amount_at(env, &vesting, now);
    let releasable = vested - vesting.released_amount;
    if releasable <= 0 {
        panic_with_error!(env, VestingError::NothingToRelease);
    }

    vesting.released_amount += releasable;
    vesting.last_release_at = now;
    let completed = vesting.released_amount >= vesting.total_amount;
    // A drained schedule that was revoked keeps the `Revoked` status: the reason
    // it stopped accruing is worth more to an auditor than the fact it is empty,
    // and either status blocks further releases.
    if completed && vesting.status == CreatorVestingStatus::Active {
        vesting.status = CreatorVestingStatus::Completed;
    }
    save_creator_vesting(env, &vesting);

    let contract_address = env.current_contract_address();
    token::TokenClient::new(env, &vesting.token).transfer(&contract_address, &creator, &releasable);

    events::publish_creator_vesting_released_event(
        env,
        campaign_id,
        creator,
        vesting.token.clone(),
        releasable,
        vesting.released_amount,
        vested,
        vesting.total_amount - vesting.released_amount,
        completed,
        now,
    );

    releasable
}

/// Freezes a schedule so nothing further vests. Admin-only.
///
/// Whatever had vested at this instant stays claimable by the creator — the
/// contract does not claw back funds the schedule had already earned. The
/// unvested remainder simply stops accruing.
pub fn revoke_creator_vesting(env: &Env, admin: Address, campaign_id: u64) {
    core::assert_admin(env, &admin);

    let mut vesting = get_creator_vesting(env, campaign_id);
    match vesting.status {
        CreatorVestingStatus::Revoked => panic_with_error!(env, VestingError::VestingRevoked),
        CreatorVestingStatus::Completed => panic_with_error!(env, VestingError::VestingCompleted),
        CreatorVestingStatus::Active => {}
    }

    let now = env.ledger().timestamp();
    let vested = vested_amount_at(env, &vesting, now);
    let forfeited = vesting.total_amount - vested;

    // Collapsing the total onto the vested figure is what freezes the curve:
    // `vested_amount_at` reports a revoked schedule's total as fully vested.
    vesting.total_amount = vested;
    vesting.status = CreatorVestingStatus::Revoked;
    save_creator_vesting(env, &vesting);

    events::publish_creator_vesting_revoked_event(
        env,
        campaign_id,
        vesting.creator.clone(),
        admin,
        vested,
        vested - vesting.released_amount,
        forfeited,
        now,
    );
}
