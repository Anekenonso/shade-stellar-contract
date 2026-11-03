#![cfg(test)]
//! Creator fund vesting: a campaign creator draws the raise down over a
//! cliff-plus-linear schedule instead of withdrawing it in one go.

use shade::shade::{Shade, ShadeClient};
use crate::from_shade::types::{CreatorVestingStatus};
use soroban_sdk::testutils::{Address as _, Events as _, Ledger as _};
use soroban_sdk::token::{StellarAssetClient, TokenClient};
use soroban_sdk::{Address, Env, FromVal, String, Symbol};

/// Ledger time the fixture starts at. Non-zero so "before the start" is a
/// representable instant.
const START: u64 = 1_000;
/// Everything the fixture's campaign raises, and the most a schedule may commit.
const RAISED: i128 = 1_000_000;
const CLIFF: u64 = 100;
const DURATION: u64 = 500;
/// 20% at the cliff, the other 80% straight-lined over the remaining 400s.
const UNLOCK_BPS: u32 = 2_000;
const INITIAL_UNLOCK: i128 = 200_000;

struct Fixture<'a> {
    env: Env,
    client: ShadeClient<'a>,
    admin: Address,
    token: Address,
    merchant: Address,
    campaign_id: u64,
}

impl Fixture<'_> {
    /// Creates the fixture's default schedule: starts now, 100s cliff at 20%,
    /// fully vested 500s in.
    fn create_default_vesting(&self) {
        self.client.create_creator_vesting(
            &self.merchant,
            &self.campaign_id,
            &RAISED,
            &START,
            &CLIFF,
            &DURATION,
            &UNLOCK_BPS,
        );
    }

    fn set_time(&self, timestamp: u64) {
        self.env.ledger().with_mut(|l| l.timestamp = timestamp);
    }

    fn balance(&self, who: &Address) -> i128 {
        TokenClient::new(&self.env, &self.token).balance(who)
    }
}

/// A registered merchant with a funded backer campaign that has already raised
/// [`RAISED`] into the contract, so vesting has something real to pay out.
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

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    let campaign_id = client.create_backer_campaign(
        &merchant,
        &String::from_str(&env, "Open Hardware Rev 2"),
        &token,
        &(START + 86_400),
    );

    let backer = Address::generate(&env);
    StellarAssetClient::new(&env, &token).mint(&backer, &RAISED);
    client.pledge_to_campaign(&backer, &campaign_id, &RAISED);

    Fixture {
        env,
        client,
        admin,
        token,
        merchant,
        campaign_id,
    }
}

// ── Creation ──────────────────────────────────────────────────────────────────

#[test]
fn test_create_vesting_stores_terms() {
    let f = setup();
    f.create_default_vesting();

    let vesting = f.client.get_creator_vesting(&f.campaign_id);
    assert_eq!(vesting.campaign_id, f.campaign_id);
    assert_eq!(vesting.creator, f.merchant);
    assert_eq!(vesting.token, f.token);
    assert_eq!(vesting.total_amount, RAISED);
    assert_eq!(vesting.released_amount, 0);
    assert_eq!(vesting.start_time, START);
    assert_eq!(vesting.cliff_duration, CLIFF);
    assert_eq!(vesting.vesting_duration, DURATION);
    assert_eq!(vesting.initial_unlock_bps, UNLOCK_BPS);
    assert_eq!(vesting.status, CreatorVestingStatus::Active);
    assert_eq!(vesting.created_at, START);
    assert_eq!(vesting.last_release_at, 0);
}

#[test]
fn test_create_vesting_indexes_campaign_under_creator() {
    let f = setup();
    f.create_default_vesting();

    let campaigns = f.client.get_creator_vesting_campaigns(&f.merchant);
    assert_eq!(campaigns.len(), 1);
    assert_eq!(campaigns.get(0).unwrap(), f.campaign_id);
}

#[test]
fn test_create_vesting_emits_event_with_absolute_unlock_dates() {
    let f = setup();
    f.create_default_vesting();

    let events = f.env.events().all();
    let last = events.last().unwrap();
    assert_eq!(last.0, f.client.address);
    let topic = Symbol::from_val(&f.env, &last.1.get(0).unwrap());
    assert_eq!(topic, Symbol::new(&f.env, "creator_vesting_created_event"));
}

#[test]
#[should_panic(expected = "Error(Contract, #261)")] // VestingAlreadyExists
fn test_campaign_can_only_have_one_schedule() {
    let f = setup();
    f.create_default_vesting();
    f.create_default_vesting();
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")] // NotAuthorized
fn test_non_owner_cannot_create_vesting() {
    let f = setup();
    let stranger = Address::generate(&f.env);
    f.client.register_merchant(&stranger);

    f.client.create_creator_vesting(
        &stranger,
        &f.campaign_id,
        &RAISED,
        &START,
        &CLIFF,
        &DURATION,
        &UNLOCK_BPS,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #265)")] // VestingAmountExceedsRaised
fn test_cannot_commit_more_than_campaign_raised() {
    let f = setup();
    f.client.create_creator_vesting(
        &f.merchant,
        &f.campaign_id,
        &(RAISED + 1),
        &START,
        &CLIFF,
        &DURATION,
        &UNLOCK_BPS,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")] // InvalidAmount
fn test_cannot_commit_zero() {
    let f = setup();
    f.client.create_creator_vesting(
        &f.merchant,
        &f.campaign_id,
        &0,
        &START,
        &CLIFF,
        &DURATION,
        &UNLOCK_BPS,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #263)")] // InvalidVestingDuration
fn test_cliff_cannot_outlast_the_schedule() {
    let f = setup();
    f.client.create_creator_vesting(
        &f.merchant,
        &f.campaign_id,
        &RAISED,
        &START,
        &(DURATION + 1),
        &DURATION,
        &UNLOCK_BPS,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #263)")] // InvalidVestingDuration
fn test_zero_duration_rejected() {
    let f = setup();
    f.client
        .create_creator_vesting(&f.merchant, &f.campaign_id, &RAISED, &START, &0, &0, &0);
}

#[test]
#[should_panic(expected = "Error(Contract, #264)")] // InvalidUnlockBps
fn test_unlock_over_one_hundred_percent_rejected() {
    let f = setup();
    f.client.create_creator_vesting(
        &f.merchant,
        &f.campaign_id,
        &RAISED,
        &START,
        &CLIFF,
        &DURATION,
        &10_001,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #269)")] // InvalidVestingStart
fn test_backdated_start_rejected() {
    let f = setup();
    f.client.create_creator_vesting(
        &f.merchant,
        &f.campaign_id,
        &RAISED,
        &(START - 1),
        &CLIFF,
        &DURATION,
        &UNLOCK_BPS,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #200)")] // CampaignNotFound
fn test_vesting_requires_an_existing_campaign() {
    let f = setup();
    f.client.create_creator_vesting(
        &f.merchant,
        &999,
        &RAISED,
        &START,
        &CLIFF,
        &DURATION,
        &UNLOCK_BPS,
    );
}

// ── Vesting curve ─────────────────────────────────────────────────────────────

#[test]
fn test_nothing_vests_before_the_cliff() {
    let f = setup();
    f.create_default_vesting();

    assert_eq!(f.client.get_vested_amount(&f.campaign_id), 0);
    f.set_time(START + CLIFF - 1);
    assert_eq!(f.client.get_vested_amount(&f.campaign_id), 0);
    assert_eq!(f.client.get_releasable_amount(&f.campaign_id), 0);
}

#[test]
fn test_cliff_unlocks_the_initial_share() {
    let f = setup();
    f.create_default_vesting();

    f.set_time(START + CLIFF);
    assert_eq!(f.client.get_vested_amount(&f.campaign_id), INITIAL_UNLOCK);
}

#[test]
fn test_remainder_vests_linearly_after_the_cliff() {
    let f = setup();
    f.create_default_vesting();

    // Quarter of the way through the 400s linear window: 20% + 80% * 1/4.
    f.set_time(START + CLIFF + 100);
    assert_eq!(f.client.get_vested_amount(&f.campaign_id), 400_000);

    // Halfway: 20% + 80% * 1/2.
    f.set_time(START + CLIFF + 200);
    assert_eq!(f.client.get_vested_amount(&f.campaign_id), 600_000);
}

#[test]
fn test_everything_vests_at_and_after_the_end() {
    let f = setup();
    f.create_default_vesting();

    f.set_time(START + DURATION);
    assert_eq!(f.client.get_vested_amount(&f.campaign_id), RAISED);
    f.set_time(START + DURATION * 10);
    assert_eq!(f.client.get_vested_amount(&f.campaign_id), RAISED);
}

#[test]
fn test_zero_cliff_vests_from_the_start() {
    let f = setup();
    f.client
        .create_creator_vesting(&f.merchant, &f.campaign_id, &RAISED, &START, &0, &400, &0);

    f.set_time(START + 100);
    assert_eq!(f.client.get_vested_amount(&f.campaign_id), 250_000);
}

#[test]
fn test_cliff_equal_to_duration_is_a_single_unlock() {
    let f = setup();
    f.client.create_creator_vesting(
        &f.merchant,
        &f.campaign_id,
        &RAISED,
        &START,
        &DURATION,
        &DURATION,
        &0,
    );

    f.set_time(START + DURATION - 1);
    assert_eq!(f.client.get_vested_amount(&f.campaign_id), 0);
    f.set_time(START + DURATION);
    assert_eq!(f.client.get_vested_amount(&f.campaign_id), RAISED);
}

// ── Releasing ─────────────────────────────────────────────────────────────────

#[test]
fn test_release_pays_the_creator_from_contract_funds() {
    let f = setup();
    f.create_default_vesting();
    f.set_time(START + CLIFF);

    let contract_before = f.balance(&f.client.address);
    let released = f
        .client
        .release_creator_vesting(&f.merchant, &f.campaign_id);

    assert_eq!(released, INITIAL_UNLOCK);
    assert_eq!(f.balance(&f.merchant), INITIAL_UNLOCK);
    assert_eq!(
        f.balance(&f.client.address),
        contract_before - INITIAL_UNLOCK
    );

    let vesting = f.client.get_creator_vesting(&f.campaign_id);
    assert_eq!(vesting.released_amount, INITIAL_UNLOCK);
    assert_eq!(vesting.last_release_at, START + CLIFF);
    assert_eq!(vesting.status, CreatorVestingStatus::Active);
}

#[test]
fn test_successive_releases_pay_only_the_newly_vested() {
    let f = setup();
    f.create_default_vesting();

    f.set_time(START + CLIFF);
    assert_eq!(
        f.client
            .release_creator_vesting(&f.merchant, &f.campaign_id),
        INITIAL_UNLOCK
    );

    // Halfway through the linear window: 600_000 vested, 200_000 already paid.
    f.set_time(START + CLIFF + 200);
    assert_eq!(
        f.client
            .release_creator_vesting(&f.merchant, &f.campaign_id),
        400_000
    );

    assert_eq!(f.balance(&f.merchant), 600_000);
    assert_eq!(f.client.get_releasable_amount(&f.campaign_id), 0);
}

#[test]
fn test_final_release_drains_and_completes_the_schedule() {
    let f = setup();
    f.create_default_vesting();

    f.set_time(START + DURATION);
    let released = f
        .client
        .release_creator_vesting(&f.merchant, &f.campaign_id);

    assert_eq!(released, RAISED);
    assert_eq!(f.balance(&f.merchant), RAISED);
    assert_eq!(f.balance(&f.client.address), 0);

    let vesting = f.client.get_creator_vesting(&f.campaign_id);
    assert_eq!(vesting.released_amount, RAISED);
    assert_eq!(vesting.status, CreatorVestingStatus::Completed);
}

#[test]
fn test_release_emits_event() {
    let f = setup();
    f.create_default_vesting();
    f.set_time(START + CLIFF);
    f.client
        .release_creator_vesting(&f.merchant, &f.campaign_id);

    let events = f.env.events().all();
    let last = events.last().unwrap();
    let topic = Symbol::from_val(&f.env, &last.1.get(0).unwrap());
    assert_eq!(topic, Symbol::new(&f.env, "creator_vesting_released_event"));
}

#[test]
#[should_panic(expected = "Error(Contract, #266)")] // NothingToRelease
fn test_release_before_cliff_reverts() {
    let f = setup();
    f.create_default_vesting();
    f.client
        .release_creator_vesting(&f.merchant, &f.campaign_id);
}

/// Two releases landing at the same ledger timestamp must not pay twice: the
/// second sees the first's `released_amount` and finds nothing newly vested.
#[test]
fn test_second_release_in_same_instant_reverts() {
    let f = setup();
    f.create_default_vesting();
    f.set_time(START + CLIFF);

    f.client
        .release_creator_vesting(&f.merchant, &f.campaign_id);
    assert!(f
        .client
        .try_release_creator_vesting(&f.merchant, &f.campaign_id)
        .is_err());

    assert_eq!(f.balance(&f.merchant), INITIAL_UNLOCK);
}

#[test]
#[should_panic(expected = "Error(Contract, #268)")] // VestingCompleted
fn test_release_after_completion_reverts() {
    let f = setup();
    f.create_default_vesting();

    f.set_time(START + DURATION);
    f.client
        .release_creator_vesting(&f.merchant, &f.campaign_id);
    f.client
        .release_creator_vesting(&f.merchant, &f.campaign_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #262)")] // NotVestingBeneficiary
fn test_stranger_cannot_release_creators_funds() {
    let f = setup();
    f.create_default_vesting();
    f.set_time(START + CLIFF);

    let stranger = Address::generate(&f.env);
    f.client.release_creator_vesting(&stranger, &f.campaign_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #260)")] // VestingNotFound
fn test_release_without_a_schedule_reverts() {
    let f = setup();
    f.client
        .release_creator_vesting(&f.merchant, &f.campaign_id);
}

// ── Revocation ────────────────────────────────────────────────────────────────

#[test]
fn test_revoke_freezes_the_schedule_at_the_vested_amount() {
    let f = setup();
    f.create_default_vesting();

    // Halfway through the linear window: 600_000 vested, 400_000 still to come.
    f.set_time(START + CLIFF + 200);
    f.client.revoke_creator_vesting(&f.admin, &f.campaign_id);

    let vesting = f.client.get_creator_vesting(&f.campaign_id);
    assert_eq!(vesting.status, CreatorVestingStatus::Revoked);
    assert_eq!(vesting.total_amount, 600_000);

    // Nothing accrues afterwards, however long we wait.
    f.set_time(START + DURATION * 10);
    assert_eq!(f.client.get_vested_amount(&f.campaign_id), 600_000);
}

#[test]
fn test_creator_keeps_what_had_vested_before_revocation() {
    let f = setup();
    f.create_default_vesting();

    f.set_time(START + CLIFF);
    f.client
        .release_creator_vesting(&f.merchant, &f.campaign_id);

    f.set_time(START + CLIFF + 200);
    f.client.revoke_creator_vesting(&f.admin, &f.campaign_id);

    // 600_000 had vested, 200_000 was already paid: the balance is still owed.
    assert_eq!(f.client.get_releasable_amount(&f.campaign_id), 400_000);
    assert_eq!(
        f.client
            .release_creator_vesting(&f.merchant, &f.campaign_id),
        400_000
    );
    assert_eq!(f.balance(&f.merchant), 600_000);

    // Draining a revoked schedule keeps the revoked status for auditors.
    let vesting = f.client.get_creator_vesting(&f.campaign_id);
    assert_eq!(vesting.status, CreatorVestingStatus::Revoked);
    assert!(f
        .client
        .try_release_creator_vesting(&f.merchant, &f.campaign_id)
        .is_err());
}

#[test]
fn test_revoke_emits_event() {
    let f = setup();
    f.create_default_vesting();
    f.set_time(START + CLIFF);
    f.client.revoke_creator_vesting(&f.admin, &f.campaign_id);

    let events = f.env.events().all();
    let last = events.last().unwrap();
    let topic = Symbol::from_val(&f.env, &last.1.get(0).unwrap());
    assert_eq!(topic, Symbol::new(&f.env, "creator_vesting_revoked_event"));
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")] // NotAuthorized
fn test_creator_cannot_revoke_their_own_schedule() {
    let f = setup();
    f.create_default_vesting();
    f.client.revoke_creator_vesting(&f.merchant, &f.campaign_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #267)")] // VestingRevoked
fn test_revoking_twice_reverts() {
    let f = setup();
    f.create_default_vesting();
    f.set_time(START + CLIFF);
    f.client.revoke_creator_vesting(&f.admin, &f.campaign_id);
    f.client.revoke_creator_vesting(&f.admin, &f.campaign_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #268)")] // VestingCompleted
fn test_revoking_a_completed_schedule_reverts() {
    let f = setup();
    f.create_default_vesting();
    f.set_time(START + DURATION);
    f.client
        .release_creator_vesting(&f.merchant, &f.campaign_id);
    f.client.revoke_creator_vesting(&f.admin, &f.campaign_id);
}

// ── Pause integration ─────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "Error(Contract, #9)")] // ContractPaused
fn test_release_blocked_while_paused() {
    let f = setup();
    f.create_default_vesting();
    f.set_time(START + CLIFF);
    f.client.pause(&f.admin);
    f.client
        .release_creator_vesting(&f.merchant, &f.campaign_id);
}

// ── Independent schedules ─────────────────────────────────────────────────────

#[test]
fn test_creator_can_vest_several_campaigns_independently() {
    let f = setup();
    f.create_default_vesting();

    let second = f.client.create_backer_campaign(
        &f.merchant,
        &String::from_str(&f.env, "Rev 3"),
        &f.token,
        &(START + 86_400),
    );
    let backer = Address::generate(&f.env);
    StellarAssetClient::new(&f.env, &f.token).mint(&backer, &400_000);
    f.client.pledge_to_campaign(&backer, &second, &400_000);

    f.client
        .create_creator_vesting(&f.merchant, &second, &400_000, &START, &0, &400, &0);

    // The first schedule is still inside its 100s cliff; the second has none.
    f.set_time(START + 50);
    assert_eq!(f.client.get_vested_amount(&f.campaign_id), 0);
    assert_eq!(f.client.get_vested_amount(&second), 50_000);

    let campaigns = f.client.get_creator_vesting_campaigns(&f.merchant);
    assert_eq!(campaigns.len(), 2);
    assert_eq!(campaigns.get(1).unwrap(), second);
}

#[test]
fn test_unknown_creator_has_no_schedules() {
    let f = setup();
    let stranger = Address::generate(&f.env);
    assert!(f.client.get_creator_vesting_campaigns(&stranger).is_empty());
}
