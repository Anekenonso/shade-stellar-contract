#![cfg(test)]
//! Tests for stretch-goal milestones on overfunded campaigns.
//!
//! Covers the full lifecycle (create → unlock → grant → claim), the ownership
//! and state guards, the strictly-increasing target ladder, event emission, and
//! that rejected calls leave no state behind.

extern crate std;

use shade::shade::{Shade, ShadeClient};
use crate::from_shade::types::{StretchGoalStatus};
use soroban_sdk::testutils::{Address as _, Events as _, Ledger as _};
use soroban_sdk::{Address, Env, String, Symbol, TryIntoVal, Vec};

const BASE_GOAL: i128 = 10_000;

struct Fixture<'a> {
    env: Env,
    client: ShadeClient<'a>,
    merchant: Address,
    backer: Address,
    campaign_id: u64,
}

fn setup() -> Fixture<'static> {
    let env = Env::default();
    env.mock_all_auths();
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

    let category_id = client.create_campaign_category(
        &admin,
        &String::from_str(&env, "Tech"),
        &String::from_str(&env, "Tech campaigns"),
    );
    let campaign_id = client.create_campaign(
        &merchant,
        &String::from_str(&env, "Widget"),
        &String::from_str(&env, "A widget"),
        &category_id,
        &Vec::new(&env),
        &BASE_GOAL,
        &token,
        &(env.ledger().timestamp() + 86_400),
    );

    let backer = Address::generate(&env);
    Fixture {
        env,
        client,
        merchant,
        backer,
        campaign_id,
    }
}

fn make_goal(f: &Fixture, target: i128) -> u64 {
    f.client.create_stretch_goal(
        &f.merchant,
        &f.campaign_id,
        &target,
        &String::from_str(&f.env, "Ship in blue"),
        &String::from_str(&f.env, "Blue edition for all backers"),
    )
}

/// Raise the campaign to `amount` so goals below it become unlockable.
fn raise_to(f: &Fixture, amount: i128) {
    let contributor = Address::generate(&f.env);
    f.client
        .record_campaign_contribution(&f.campaign_id, &contributor, &amount);
}

// ── Creation ──────────────────────────────────────────────────────────────────

#[test]
fn create_stretch_goal_stores_all_fields() {
    let f = setup();
    let id = make_goal(&f, 15_000);

    let goal = f.client.get_stretch_goal(&id);
    assert_eq!(goal.id, id);
    assert_eq!(goal.campaign_id, f.campaign_id);
    assert_eq!(goal.merchant, f.merchant);
    assert_eq!(goal.target_amount, 15_000);
    assert_eq!(goal.status, StretchGoalStatus::Pending);
    assert_eq!(goal.unlocked_at, 0);
    assert_eq!(goal.reward_count, 0);
    assert_eq!(goal.total_reward_amount, 0);
    assert_eq!(goal.description, String::from_str(&f.env, "Ship in blue"));
}

#[test]
fn create_stretch_goal_indexes_goal_under_its_campaign() {
    let f = setup();
    let a = make_goal(&f, 15_000);
    let b = make_goal(&f, 20_000);

    let ids = f.client.get_campaign_stretch_goals(&f.campaign_id);
    assert_eq!(ids.len(), 2);
    assert_eq!(ids.get(0).unwrap(), a);
    assert_eq!(ids.get(1).unwrap(), b);

    let details = f.client.get_campaign_stretch_goal_data(&f.campaign_id);
    assert_eq!(details.len(), 2);
    assert_eq!(details.get(0).unwrap().target_amount, 15_000);
    assert_eq!(details.get(1).unwrap().target_amount, 20_000);
}

#[test]
fn create_stretch_goal_assigns_sequential_ids() {
    let f = setup();
    assert_eq!(make_goal(&f, 15_000), 1);
    assert_eq!(make_goal(&f, 20_000), 2);
    assert_eq!(make_goal(&f, 25_000), 3);
}

#[test]
#[should_panic(expected = "Error(Contract, #246)")] // TargetBelowBaseGoal
fn create_stretch_goal_rejects_target_at_base_goal() {
    let f = setup();
    make_goal(&f, BASE_GOAL);
}

#[test]
#[should_panic(expected = "Error(Contract, #246)")] // TargetBelowBaseGoal
fn create_stretch_goal_rejects_target_below_base_goal() {
    let f = setup();
    make_goal(&f, BASE_GOAL - 1);
}

#[test]
#[should_panic(expected = "Error(Contract, #247)")] // TargetNotIncreasing
fn create_stretch_goal_rejects_non_increasing_target() {
    let f = setup();
    make_goal(&f, 20_000);
    make_goal(&f, 20_000);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")] // InvalidAmount
fn create_stretch_goal_rejects_zero_target() {
    let f = setup();
    make_goal(&f, 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #33)")] // InvalidDescription
fn create_stretch_goal_rejects_empty_description() {
    let f = setup();
    f.client.create_stretch_goal(
        &f.merchant,
        &f.campaign_id,
        &15_000,
        &String::from_str(&f.env, ""),
        &String::from_str(&f.env, "reward"),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #250)")] // NotGoalOwner
fn create_stretch_goal_rejects_non_owner() {
    let f = setup();
    let attacker = Address::generate(&f.env);
    f.client.create_stretch_goal(
        &attacker,
        &f.campaign_id,
        &15_000,
        &String::from_str(&f.env, "d"),
        &String::from_str(&f.env, "r"),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #200)")] // CampaignNotFound
fn create_stretch_goal_rejects_unknown_campaign() {
    let f = setup();
    f.client.create_stretch_goal(
        &f.merchant,
        &999,
        &15_000,
        &String::from_str(&f.env, "d"),
        &String::from_str(&f.env, "r"),
    );
}

// ── Unlocking ─────────────────────────────────────────────────────────────────

#[test]
fn unlock_stretch_goal_marks_goal_unlocked_at_target() {
    let f = setup();
    let id = make_goal(&f, 15_000);
    raise_to(&f, 15_000);

    f.env.ledger().with_mut(|l| l.timestamp = 5_000);
    f.client.unlock_stretch_goal(&f.merchant, &id);

    let goal = f.client.get_stretch_goal(&id);
    assert_eq!(goal.status, StretchGoalStatus::Unlocked);
    assert_eq!(goal.unlocked_at, 5_000);
}

#[test]
fn unlock_stretch_goal_succeeds_when_campaign_overshoots_target() {
    let f = setup();
    let id = make_goal(&f, 15_000);
    raise_to(&f, 50_000);
    f.client.unlock_stretch_goal(&f.merchant, &id);
    assert_eq!(
        f.client.get_stretch_goal(&id).status,
        StretchGoalStatus::Unlocked
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #248)")] // TargetNotReached
fn unlock_stretch_goal_rejects_when_target_not_reached() {
    let f = setup();
    let id = make_goal(&f, 15_000);
    raise_to(&f, 14_999);
    f.client.unlock_stretch_goal(&f.merchant, &id);
}

#[test]
#[should_panic(expected = "Error(Contract, #248)")] // TargetNotReached
fn unlock_stretch_goal_rejects_with_no_contributions() {
    let f = setup();
    let id = make_goal(&f, 15_000);
    f.client.unlock_stretch_goal(&f.merchant, &id);
}

#[test]
#[should_panic(expected = "Error(Contract, #242)")] // GoalAlreadyUnlocked
fn unlock_stretch_goal_is_not_repeatable() {
    let f = setup();
    let id = make_goal(&f, 15_000);
    raise_to(&f, 15_000);
    f.client.unlock_stretch_goal(&f.merchant, &id);
    f.client.unlock_stretch_goal(&f.merchant, &id);
}

#[test]
#[should_panic(expected = "Error(Contract, #250)")] // NotGoalOwner
fn unlock_stretch_goal_rejects_non_owner() {
    let f = setup();
    let id = make_goal(&f, 15_000);
    raise_to(&f, 15_000);
    let attacker = Address::generate(&f.env);
    f.client.unlock_stretch_goal(&attacker, &id);
}

#[test]
#[should_panic(expected = "Error(Contract, #240)")] // StretchGoalNotFound
fn unlock_stretch_goal_rejects_unknown_goal() {
    let f = setup();
    f.client.unlock_stretch_goal(&f.merchant, &999);
}

/// A goal whose target is not yet met must be left untouched by a failed unlock.
#[test]
fn failed_unlock_leaves_goal_pending() {
    let f = setup();
    let id = make_goal(&f, 15_000);
    raise_to(&f, 1_000);

    assert!(f.client.try_unlock_stretch_goal(&f.merchant, &id).is_err());

    let goal = f.client.get_stretch_goal(&id);
    assert_eq!(goal.status, StretchGoalStatus::Pending);
    assert_eq!(goal.unlocked_at, 0);
}

// ── Cancellation ──────────────────────────────────────────────────────────────

#[test]
fn cancel_stretch_goal_marks_goal_cancelled() {
    let f = setup();
    let id = make_goal(&f, 15_000);
    f.client.cancel_stretch_goal(&f.merchant, &id);
    assert_eq!(
        f.client.get_stretch_goal(&id).status,
        StretchGoalStatus::Cancelled
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #249)")] // GoalCancelled
fn cancelled_goal_cannot_be_unlocked() {
    let f = setup();
    let id = make_goal(&f, 15_000);
    f.client.cancel_stretch_goal(&f.merchant, &id);
    raise_to(&f, 15_000);
    f.client.unlock_stretch_goal(&f.merchant, &id);
}

#[test]
#[should_panic(expected = "Error(Contract, #242)")] // GoalAlreadyUnlocked
fn unlocked_goal_cannot_be_cancelled() {
    let f = setup();
    let id = make_goal(&f, 15_000);
    raise_to(&f, 15_000);
    f.client.unlock_stretch_goal(&f.merchant, &id);
    f.client.cancel_stretch_goal(&f.merchant, &id);
}

#[test]
#[should_panic(expected = "Error(Contract, #250)")] // NotGoalOwner
fn cancel_stretch_goal_rejects_non_owner() {
    let f = setup();
    let id = make_goal(&f, 15_000);
    let attacker = Address::generate(&f.env);
    f.client.cancel_stretch_goal(&attacker, &id);
}

// ── Rewards ───────────────────────────────────────────────────────────────────

fn unlocked_goal(f: &Fixture) -> u64 {
    let id = make_goal(f, 15_000);
    raise_to(f, 15_000);
    f.client.unlock_stretch_goal(&f.merchant, &id);
    id
}

#[test]
fn grant_reward_stores_reward_and_updates_goal_totals() {
    let f = setup();
    let id = unlocked_goal(&f);

    f.env.ledger().with_mut(|l| l.timestamp = 7_000);
    f.client
        .grant_stretch_goal_reward(&f.merchant, &id, &f.backer, &250);

    let reward = f
        .client
        .get_stretch_goal_reward(&id, &f.backer)
        .expect("reward should exist");
    assert_eq!(reward.goal_id, id);
    assert_eq!(reward.campaign_id, f.campaign_id);
    assert_eq!(reward.backer, f.backer);
    assert_eq!(reward.reward_amount, 250);
    assert!(!reward.claimed);
    assert_eq!(reward.granted_at, 7_000);
    assert_eq!(reward.claimed_at, 0);

    let goal = f.client.get_stretch_goal(&id);
    assert_eq!(goal.reward_count, 1);
    assert_eq!(goal.total_reward_amount, 250);
}

/// One goal must be able to reward many backers — the reward key is
/// `(goal_id, backer)`, not `goal_id` alone.
#[test]
fn grant_reward_supports_many_backers_per_goal() {
    let f = setup();
    let id = unlocked_goal(&f);
    let b2 = Address::generate(&f.env);
    let b3 = Address::generate(&f.env);

    f.client
        .grant_stretch_goal_reward(&f.merchant, &id, &f.backer, &100);
    f.client
        .grant_stretch_goal_reward(&f.merchant, &id, &b2, &200);
    f.client
        .grant_stretch_goal_reward(&f.merchant, &id, &b3, &300);

    assert_eq!(
        f.client
            .get_stretch_goal_reward(&id, &f.backer)
            .unwrap()
            .reward_amount,
        100
    );
    assert_eq!(
        f.client
            .get_stretch_goal_reward(&id, &b2)
            .unwrap()
            .reward_amount,
        200
    );
    assert_eq!(
        f.client
            .get_stretch_goal_reward(&id, &b3)
            .unwrap()
            .reward_amount,
        300
    );

    let goal = f.client.get_stretch_goal(&id);
    assert_eq!(goal.reward_count, 3);
    assert_eq!(goal.total_reward_amount, 600);
}

#[test]
#[should_panic(expected = "Error(Contract, #245)")] // RewardAlreadyGranted
fn grant_reward_rejects_duplicate_grant_to_same_backer() {
    let f = setup();
    let id = unlocked_goal(&f);
    f.client
        .grant_stretch_goal_reward(&f.merchant, &id, &f.backer, &100);
    f.client
        .grant_stretch_goal_reward(&f.merchant, &id, &f.backer, &100);
}

#[test]
#[should_panic(expected = "Error(Contract, #241)")] // StretchGoalNotUnlocked
fn grant_reward_rejects_pending_goal() {
    let f = setup();
    let id = make_goal(&f, 15_000);
    f.client
        .grant_stretch_goal_reward(&f.merchant, &id, &f.backer, &100);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")] // InvalidAmount
fn grant_reward_rejects_zero_amount() {
    let f = setup();
    let id = unlocked_goal(&f);
    f.client
        .grant_stretch_goal_reward(&f.merchant, &id, &f.backer, &0);
}

#[test]
#[should_panic(expected = "Error(Contract, #250)")] // NotGoalOwner
fn grant_reward_rejects_non_owner() {
    let f = setup();
    let id = unlocked_goal(&f);
    let attacker = Address::generate(&f.env);
    f.client
        .grant_stretch_goal_reward(&attacker, &id, &f.backer, &100);
}

#[test]
fn get_reward_returns_none_when_never_granted() {
    let f = setup();
    let id = unlocked_goal(&f);
    assert!(f.client.get_stretch_goal_reward(&id, &f.backer).is_none());
}

// ── Claiming ──────────────────────────────────────────────────────────────────

#[test]
fn claim_reward_marks_reward_claimed() {
    let f = setup();
    let id = unlocked_goal(&f);
    f.client
        .grant_stretch_goal_reward(&f.merchant, &id, &f.backer, &250);

    f.env.ledger().with_mut(|l| l.timestamp = 9_000);
    f.client.claim_stretch_goal_reward(&f.backer, &id);

    let reward = f.client.get_stretch_goal_reward(&id, &f.backer).unwrap();
    assert!(reward.claimed);
    assert_eq!(reward.claimed_at, 9_000);
    assert_eq!(reward.reward_amount, 250);
}

#[test]
#[should_panic(expected = "Error(Contract, #243)")] // RewardAlreadyClaimed
fn claim_reward_is_not_repeatable() {
    let f = setup();
    let id = unlocked_goal(&f);
    f.client
        .grant_stretch_goal_reward(&f.merchant, &id, &f.backer, &250);
    f.client.claim_stretch_goal_reward(&f.backer, &id);
    f.client.claim_stretch_goal_reward(&f.backer, &id);
}

#[test]
#[should_panic(expected = "Error(Contract, #244)")] // RewardNotFound
fn claim_reward_rejects_backer_without_a_grant() {
    let f = setup();
    let id = unlocked_goal(&f);
    let stranger = Address::generate(&f.env);
    f.client.claim_stretch_goal_reward(&stranger, &id);
}

/// A backer must not be able to claim another backer's reward: the reward is
/// keyed by the caller's own address.
#[test]
#[should_panic(expected = "Error(Contract, #244)")] // RewardNotFound
fn claim_reward_rejects_other_backers_reward() {
    let f = setup();
    let id = unlocked_goal(&f);
    let other = Address::generate(&f.env);
    f.client
        .grant_stretch_goal_reward(&f.merchant, &id, &f.backer, &250);
    f.client.claim_stretch_goal_reward(&other, &id);
}

#[test]
fn failed_claim_leaves_reward_unclaimed() {
    let f = setup();
    let id = unlocked_goal(&f);
    f.client
        .grant_stretch_goal_reward(&f.merchant, &id, &f.backer, &250);

    let other = Address::generate(&f.env);
    assert!(f.client.try_claim_stretch_goal_reward(&other, &id).is_err());

    let reward = f.client.get_stretch_goal_reward(&id, &f.backer).unwrap();
    assert!(!reward.claimed);
    assert_eq!(reward.claimed_at, 0);
}

// ── Next-milestone query ──────────────────────────────────────────────────────

#[test]
fn next_stretch_goal_returns_lowest_pending_goal() {
    let f = setup();
    let first = make_goal(&f, 15_000);
    make_goal(&f, 20_000);

    assert_eq!(
        f.client.get_next_stretch_goal(&f.campaign_id).unwrap().id,
        first
    );

    raise_to(&f, 15_000);
    f.client.unlock_stretch_goal(&f.merchant, &first);

    let next = f.client.get_next_stretch_goal(&f.campaign_id).unwrap();
    assert_eq!(next.target_amount, 20_000);
}

#[test]
fn next_stretch_goal_is_none_when_all_resolved() {
    let f = setup();
    let id = make_goal(&f, 15_000);
    raise_to(&f, 15_000);
    f.client.unlock_stretch_goal(&f.merchant, &id);
    assert!(f.client.get_next_stretch_goal(&f.campaign_id).is_none());
}

#[test]
fn next_stretch_goal_is_none_for_campaign_without_goals() {
    let f = setup();
    assert!(f.client.get_next_stretch_goal(&f.campaign_id).is_none());
}

// ── Events ────────────────────────────────────────────────────────────────────

/// `env.events().all()` only holds the most recent invocation's events, so each
/// assertion runs immediately after the call under test.
fn last_event_topic(env: &Env) -> Symbol {
    let events = env.events().all();
    assert!(!events.is_empty(), "no events emitted");
    let (_c, topics, _d) = events.get(events.len() - 1).unwrap();
    topics.get(0).unwrap().try_into_val(env).unwrap()
}

#[test]
fn create_emits_stretch_goal_created_event() {
    let f = setup();
    make_goal(&f, 15_000);
    assert_eq!(
        last_event_topic(&f.env),
        Symbol::new(&f.env, "stretch_goal_created_event")
    );
}

#[test]
fn unlock_emits_stretch_goal_unlocked_event() {
    let f = setup();
    let id = make_goal(&f, 15_000);
    raise_to(&f, 15_000);
    f.client.unlock_stretch_goal(&f.merchant, &id);
    assert_eq!(
        last_event_topic(&f.env),
        Symbol::new(&f.env, "stretch_goal_unlocked_event")
    );
}

#[test]
fn cancel_emits_stretch_goal_cancelled_event() {
    let f = setup();
    let id = make_goal(&f, 15_000);
    f.client.cancel_stretch_goal(&f.merchant, &id);
    assert_eq!(
        last_event_topic(&f.env),
        Symbol::new(&f.env, "stretch_goal_cancelled_event")
    );
}

#[test]
fn grant_emits_stretch_reward_granted_event() {
    let f = setup();
    let id = unlocked_goal(&f);
    f.client
        .grant_stretch_goal_reward(&f.merchant, &id, &f.backer, &250);
    assert_eq!(
        last_event_topic(&f.env),
        Symbol::new(&f.env, "stretch_reward_granted_event")
    );
}

#[test]
fn claim_emits_stretch_reward_claimed_event() {
    let f = setup();
    let id = unlocked_goal(&f);
    f.client
        .grant_stretch_goal_reward(&f.merchant, &id, &f.backer, &250);
    f.client.claim_stretch_goal_reward(&f.backer, &id);
    assert_eq!(
        last_event_topic(&f.env),
        Symbol::new(&f.env, "stretch_reward_claimed_event")
    );
}

// ── Multi-campaign isolation ──────────────────────────────────────────────────

/// Goals and their targets are scoped per campaign: a second campaign starts
/// its own ladder and does not see the first campaign's goals.
#[test]
fn goals_are_scoped_per_campaign() {
    let f = setup();
    let a = make_goal(&f, 15_000);

    let admin = f.client.get_admin();
    let category_id = f.client.create_campaign_category(
        &admin,
        &String::from_str(&f.env, "Art"),
        &String::from_str(&f.env, "Art campaigns"),
    );
    let token = f.client.get_campaign(&f.campaign_id).token;
    let other_campaign = f.client.create_campaign(
        &f.merchant,
        &String::from_str(&f.env, "Canvas"),
        &String::from_str(&f.env, "A canvas"),
        &category_id,
        &Vec::new(&f.env),
        &BASE_GOAL,
        &token,
        &(f.env.ledger().timestamp() + 86_400),
    );

    // 12_000 is below campaign A's existing 15_000 goal but is still valid here.
    let b = f.client.create_stretch_goal(
        &f.merchant,
        &other_campaign,
        &12_000,
        &String::from_str(&f.env, "d"),
        &String::from_str(&f.env, "r"),
    );

    let a_ids = f.client.get_campaign_stretch_goals(&f.campaign_id);
    let b_ids = f.client.get_campaign_stretch_goals(&other_campaign);
    assert_eq!(a_ids.len(), 1);
    assert_eq!(b_ids.len(), 1);
    assert_eq!(a_ids.get(0).unwrap(), a);
    assert_eq!(b_ids.get(0).unwrap(), b);
}

/// Raising campaign A must not unlock campaign B's goals.
#[test]
#[should_panic(expected = "Error(Contract, #248)")] // TargetNotReached
fn unlock_reads_the_goals_own_campaign_raise() {
    let f = setup();

    let admin = f.client.get_admin();
    let category_id = f.client.create_campaign_category(
        &admin,
        &String::from_str(&f.env, "Art"),
        &String::from_str(&f.env, "Art campaigns"),
    );
    let token = f.client.get_campaign(&f.campaign_id).token;
    let other_campaign = f.client.create_campaign(
        &f.merchant,
        &String::from_str(&f.env, "Canvas"),
        &String::from_str(&f.env, "A canvas"),
        &category_id,
        &Vec::new(&f.env),
        &BASE_GOAL,
        &token,
        &(f.env.ledger().timestamp() + 86_400),
    );
    let goal_b = f.client.create_stretch_goal(
        &f.merchant,
        &other_campaign,
        &15_000,
        &String::from_str(&f.env, "d"),
        &String::from_str(&f.env, "r"),
    );

    // Fund campaign A well past the target, then try to unlock B's goal.
    raise_to(&f, 100_000);
    f.client.unlock_stretch_goal(&f.merchant, &goal_b);
}
