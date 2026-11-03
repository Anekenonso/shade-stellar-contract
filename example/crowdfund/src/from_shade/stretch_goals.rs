//! Stretch-goal milestones for overfunded campaigns.
//!
//! A stretch goal is a funding milestone *beyond* a campaign's base goal. The
//! owning merchant defines goals up front; each unlocks once the campaign's
//! cumulative raise reaches its `target_amount`. Once unlocked, the merchant
//! grants per-backer rewards which backers then claim.
//!
//! # Invariants
//!
//! - Only the campaign's owning merchant may create, unlock, cancel or grant
//!   rewards for its goals. Backers may only claim their own reward.
//! - `target_amount` must exceed the campaign's base `goal_amount` (a milestone
//!   at or below the base goal is not a *stretch* goal) and must be strictly
//!   greater than every existing goal on the campaign, so a campaign's goals
//!   form a strictly increasing ladder.
//! - A goal can only leave `Pending` once, so concurrent `unlock_stretch_goal`
//!   calls cannot double-emit or double-count.
//! - Rewards are keyed by `(goal_id, backer)`, so one goal serves any number of
//!   backers, and a backer can be granted at most one reward per goal.
//!
//! # Storage
//!
//! Keys live in [`StretchKey`], a dedicated key enum, so this feature adds no
//! pressure to the core `DataKey` enum (Soroban caps every enum at 50 cases).
//! The reverse index [`StretchKey::CampaignStretchGoals`] holds only `u64` IDs
//! rather than whole records, keeping the per-campaign entry small and cheap to
//! extend as goals are added.

use crate::from_shade::campaigns;
use crate::from_shade::errors::{ContractError, StretchGoalError};
use crate::from_shade::events;
use crate::from_shade::types::{StretchGoal, StretchGoalReward, StretchGoalStatus, StretchKey};
use soroban_sdk::{panic_with_error, Address, Env, String, Vec};

/// Upper bound on goals per campaign. Bounds the reverse-index entry so reads
/// and writes stay within a predictable rent/CPU budget.
const MAX_GOALS_PER_CAMPAIGN: u32 = 32;

/// Maximum length accepted for the free-text description fields.
const MAX_TEXT_LEN: u32 = 300;

fn get_stretch_goal_count(env: &Env) -> u64 {
    env.storage()
        .persistent()
        .get(&StretchKey::StretchGoalCount)
        .unwrap_or(0)
}

fn set_stretch_goal_count(env: &Env, count: u64) {
    env.storage()
        .persistent()
        .set(&StretchKey::StretchGoalCount, &count);
}

fn save_goal(env: &Env, goal: &StretchGoal) {
    env.storage()
        .persistent()
        .set(&StretchKey::StretchGoal(goal.id), goal);
}

/// Loads a goal, panicking with [`StretchGoalError::StretchGoalNotFound`] if it
/// does not exist.
pub fn get_stretch_goal(env: &Env, goal_id: u64) -> StretchGoal {
    env.storage()
        .persistent()
        .get(&StretchKey::StretchGoal(goal_id))
        .unwrap_or_else(|| panic_with_error!(env, StretchGoalError::StretchGoalNotFound))
}

/// Ordered list of a campaign's stretch goal IDs, lowest target first.
pub fn get_campaign_stretch_goals(env: &Env, campaign_id: u64) -> Vec<u64> {
    env.storage()
        .persistent()
        .get(&StretchKey::CampaignStretchGoals(campaign_id))
        .unwrap_or_else(|| Vec::new(env))
}

/// Asserts `caller` is the merchant that owns `campaign_id`, and returns the
/// campaign's base goal amount.
fn assert_campaign_owner(env: &Env, campaign_id: u64, caller: &Address) -> i128 {
    let campaign = campaigns::get_campaign(env, campaign_id);
    if campaign.merchant != *caller {
        panic_with_error!(env, StretchGoalError::NotGoalOwner);
    }
    campaign.goal_amount
}

/// Creates a new stretch goal on a campaign.
///
/// `target_amount` is the cumulative campaign raise that unlocks the goal. It
/// must be above the campaign's base goal and above every existing goal on the
/// campaign, keeping each campaign's goals strictly increasing.
pub fn create_stretch_goal(
    env: &Env,
    merchant: Address,
    campaign_id: u64,
    target_amount: i128,
    description: String,
    reward_description: String,
) -> u64 {
    merchant.require_auth();

    let base_goal = assert_campaign_owner(env, campaign_id, &merchant);

    if target_amount <= 0 {
        panic_with_error!(env, shade::errors::ContractError::InvalidAmount);
    }
    if target_amount <= base_goal {
        panic_with_error!(env, StretchGoalError::TargetBelowBaseGoal);
    }
    if description.is_empty()
        || reward_description.is_empty()
        || description.len() > MAX_TEXT_LEN
        || reward_description.len() > MAX_TEXT_LEN
    {
        panic_with_error!(env, shade::errors::ContractError::InvalidDescription);
    }

    let mut campaign_goals = get_campaign_stretch_goals(env, campaign_id);
    if campaign_goals.len() >= MAX_GOALS_PER_CAMPAIGN {
        panic_with_error!(env, StretchGoalError::TooManyStretchGoals);
    }
    // Targets must strictly increase; the list is append-ordered so the last
    // entry carries the current highest target.
    if let Some(last_id) = campaign_goals.last() {
        let last_goal = get_stretch_goal(env, last_id);
        if target_amount <= last_goal.target_amount {
            panic_with_error!(env, StretchGoalError::TargetNotIncreasing);
        }
    }

    let goal_id = get_stretch_goal_count(env) + 1;
    set_stretch_goal_count(env, goal_id);

    let now = env.ledger().timestamp();
    let goal = StretchGoal {
        id: goal_id,
        campaign_id,
        merchant: merchant.clone(),
        target_amount,
        description,
        reward_description,
        status: StretchGoalStatus::Pending,
        created_at: now,
        unlocked_at: 0,
        reward_count: 0,
        total_reward_amount: 0,
    };
    save_goal(env, &goal);

    campaign_goals.push_back(goal_id);
    env.storage().persistent().set(
        &StretchKey::CampaignStretchGoals(campaign_id),
        &campaign_goals,
    );

    events::publish_stretch_goal_created_event(
        env,
        goal_id,
        campaign_id,
        merchant,
        target_amount,
        base_goal,
        campaign_goals.len(),
        now,
    );

    goal_id
}

/// Unlocks a goal once the campaign has raised at least its `target_amount`.
///
/// The raised amount is read from the campaign record rather than supplied by
/// the caller, so a merchant cannot unlock a goal the campaign has not actually
/// reached. Callable by the owning merchant.
pub fn unlock_stretch_goal(env: &Env, merchant: Address, goal_id: u64) {
    merchant.require_auth();

    let mut goal = get_stretch_goal(env, goal_id);
    if goal.merchant != merchant {
        panic_with_error!(env, StretchGoalError::NotGoalOwner);
    }

    match goal.status {
        StretchGoalStatus::Unlocked => {
            panic_with_error!(env, StretchGoalError::GoalAlreadyUnlocked)
        }
        StretchGoalStatus::Cancelled => panic_with_error!(env, StretchGoalError::GoalCancelled),
        StretchGoalStatus::Pending => {}
    }

    let campaign = campaigns::get_campaign(env, goal.campaign_id);
    if campaign.raised_amount < goal.target_amount {
        panic_with_error!(env, StretchGoalError::TargetNotReached);
    }

    let now = env.ledger().timestamp();
    goal.status = StretchGoalStatus::Unlocked;
    goal.unlocked_at = now;
    save_goal(env, &goal);

    events::publish_stretch_goal_unlocked_event(
        env,
        goal_id,
        goal.campaign_id,
        goal.merchant.clone(),
        goal.target_amount,
        campaign.raised_amount,
        now,
    );
}

/// Retires a goal that has not been unlocked. Callable by the owning merchant.
pub fn cancel_stretch_goal(env: &Env, merchant: Address, goal_id: u64) {
    merchant.require_auth();

    let mut goal = get_stretch_goal(env, goal_id);
    if goal.merchant != merchant {
        panic_with_error!(env, StretchGoalError::NotGoalOwner);
    }
    match goal.status {
        StretchGoalStatus::Unlocked => {
            panic_with_error!(env, StretchGoalError::GoalAlreadyUnlocked)
        }
        StretchGoalStatus::Cancelled => panic_with_error!(env, StretchGoalError::GoalCancelled),
        StretchGoalStatus::Pending => {}
    }

    let now = env.ledger().timestamp();
    goal.status = StretchGoalStatus::Cancelled;
    save_goal(env, &goal);

    events::publish_stretch_goal_cancelled_event(env, goal_id, goal.campaign_id, merchant, now);
}

/// Grants a reward to one backer for an unlocked goal.
///
/// Rewards are keyed by `(goal_id, backer)`, so a goal can reward any number of
/// backers but each backer at most once. Callable by the owning merchant.
pub fn grant_stretch_goal_reward(
    env: &Env,
    merchant: Address,
    goal_id: u64,
    backer: Address,
    reward_amount: i128,
) {
    merchant.require_auth();

    let mut goal = get_stretch_goal(env, goal_id);
    if goal.merchant != merchant {
        panic_with_error!(env, StretchGoalError::NotGoalOwner);
    }
    if goal.status != StretchGoalStatus::Unlocked {
        panic_with_error!(env, StretchGoalError::StretchGoalNotUnlocked);
    }
    if reward_amount <= 0 {
        panic_with_error!(env, shade::errors::ContractError::InvalidAmount);
    }

    let reward_key = StretchKey::StretchGoalReward(goal_id, backer.clone());
    if env.storage().persistent().has(&reward_key) {
        panic_with_error!(env, StretchGoalError::RewardAlreadyGranted);
    }

    let now = env.ledger().timestamp();
    let reward = StretchGoalReward {
        goal_id,
        campaign_id: goal.campaign_id,
        backer: backer.clone(),
        reward_amount,
        claimed: false,
        granted_at: now,
        claimed_at: 0,
    };
    env.storage().persistent().set(&reward_key, &reward);

    goal.reward_count = goal.reward_count.saturating_add(1);
    goal.total_reward_amount = goal.total_reward_amount.saturating_add(reward_amount);
    save_goal(env, &goal);

    events::publish_stretch_reward_granted_event(
        env,
        goal_id,
        goal.campaign_id,
        backer,
        reward_amount,
        goal.reward_count,
        goal.total_reward_amount,
        now,
    );
}

/// Marks the caller's reward for a goal as claimed. Callable by the backer only.
pub fn claim_stretch_goal_reward(env: &Env, backer: Address, goal_id: u64) {
    backer.require_auth();

    let reward_key = StretchKey::StretchGoalReward(goal_id, backer.clone());
    let mut reward: StretchGoalReward = env
        .storage()
        .persistent()
        .get(&reward_key)
        .unwrap_or_else(|| panic_with_error!(env, StretchGoalError::RewardNotFound));

    if reward.claimed {
        panic_with_error!(env, StretchGoalError::RewardAlreadyClaimed);
    }

    // A goal cancelled after a grant must not pay out.
    let goal = get_stretch_goal(env, goal_id);
    if goal.status == StretchGoalStatus::Cancelled {
        panic_with_error!(env, StretchGoalError::GoalCancelled);
    }

    let now = env.ledger().timestamp();
    reward.claimed = true;
    reward.claimed_at = now;
    env.storage().persistent().set(&reward_key, &reward);

    events::publish_stretch_reward_claimed_event(
        env,
        goal_id,
        reward.campaign_id,
        backer,
        reward.reward_amount,
        now,
    );
}

/// Returns a backer's reward for a goal, or `None` if none was granted.
pub fn get_stretch_goal_reward(
    env: &Env,
    goal_id: u64,
    backer: Address,
) -> Option<StretchGoalReward> {
    env.storage()
        .persistent()
        .get(&StretchKey::StretchGoalReward(goal_id, backer))
}

/// Every goal record for a campaign, in creation (ascending target) order.
pub fn get_campaign_stretch_goal_data(env: &Env, campaign_id: u64) -> Vec<StretchGoal> {
    let ids = get_campaign_stretch_goals(env, campaign_id);
    let mut goals = Vec::new(env);
    for id in ids.iter() {
        if let Some(goal) = env
            .storage()
            .persistent()
            .get::<_, StretchGoal>(&StretchKey::StretchGoal(id))
        {
            goals.push_back(goal);
        }
    }
    goals
}

/// The next goal a campaign has yet to unlock, if any. Useful for UIs showing
/// "next milestone" progress without fetching every goal.
pub fn get_next_stretch_goal(env: &Env, campaign_id: u64) -> Option<StretchGoal> {
    let ids = get_campaign_stretch_goals(env, campaign_id);
    for id in ids.iter() {
        let goal = get_stretch_goal(env, id);
        if goal.status == StretchGoalStatus::Pending {
            return Some(goal);
        }
    }
    None
}
