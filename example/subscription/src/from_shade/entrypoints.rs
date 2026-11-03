//! The entry points these modules exposed on the Shade hub, kept for reference.

use super::types::*;
use shade::types::*;
use soroban_sdk::{Address, BytesN, Env, String, Vec};

pub struct HubEntrypoints;

impl HubEntrypoints {
    pub fn create_subscription_plan(
        env: Env,
        merchant: Address,
        description: String,
        token: Address,
        amount: i128,
        interval: u64,
    ) -> u64 {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::subscription::create_subscription_plan(
            &env,
            merchant,
            description,
            token,
            amount,
            interval,
        )
    }

    pub fn get_subscription_plan(env: Env, plan_id: u64) -> SubscriptionPlan {
        crate::from_shade::subscription::get_subscription_plan(&env, plan_id)
    }

    pub fn subscribe(env: Env, customer: Address, plan_id: u64) -> u64 {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::subscription::subscribe(&env, customer, plan_id)
    }

    pub fn get_subscription(env: Env, subscription_id: u64) -> Subscription {
        crate::from_shade::subscription::get_subscription(&env, subscription_id)
    }

    pub fn charge_subscription(env: Env, subscription_id: u64) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::subscription::charge_subscription(&env, subscription_id);
    }

    pub fn cancel_subscription(env: Env, caller: Address, subscription_id: u64) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::subscription::cancel_subscription(&env, caller, subscription_id);
    }

    pub fn deactivate_plan(env: Env, caller: Address, plan_id: u64) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::subscription::deactivate_plan(&env, caller, plan_id);
    }

    pub fn search_subscription_plans(
        env: Env,
        caller: Address,
        filter: SubscriptionPlanFilter,
    ) -> Vec<SubscriptionPlan> {
        crate::from_shade::search::search_subscription_plans(&env, &caller, filter)
    }

    pub fn search_subscriptions(env: Env, filter: SubscriptionFilter) -> Vec<Subscription> {
        crate::from_shade::search::search_subscriptions(&env, filter)
    }
}
