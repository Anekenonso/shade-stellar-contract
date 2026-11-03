use soroban_sdk::{contractevent, Address, BytesN, Env, String, Vec};
use super::types::*;
use shade::types::*;

#[contractevent]
pub struct SubscriptionPlanCreatedEvent {
    pub plan_id: u64,
    pub merchant: Address,
    pub token: Address,
    pub amount: i128,
    pub interval: u64,
    pub timestamp: u64,
}

pub fn publish_subscription_plan_created_event(
    env: &Env,
    plan_id: u64,
    merchant: Address,
    token: Address,
    amount: i128,
    interval: u64,
    timestamp: u64,
) {
    SubscriptionPlanCreatedEvent {
        plan_id,
        merchant,
        token,
        amount,
        interval,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct SubscribedEvent {
    pub subscription_id: u64,
    pub plan_id: u64,
    pub customer: Address,
    pub timestamp: u64,
}

pub fn publish_subscribed_event(
    env: &Env,
    subscription_id: u64,
    plan_id: u64,
    customer: Address,
    timestamp: u64,
) {
    SubscribedEvent {
        subscription_id,
        plan_id,
        customer,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct SubscriptionChargedEvent {
    pub subscription_id: u64,
    pub plan_id: u64,
    pub customer: Address,
    pub merchant: Address,
    pub amount: i128,
    pub fee: i128,
    pub token: Address,
    pub timestamp: u64,
}

#[allow(clippy::too_many_arguments)]
pub fn publish_subscription_charged_event(
    env: &Env,
    subscription_id: u64,
    plan_id: u64,
    customer: Address,
    merchant: Address,
    amount: i128,
    fee: i128,
    token: Address,
    timestamp: u64,
) {
    SubscriptionChargedEvent {
        subscription_id,
        plan_id,
        customer,
        merchant,
        amount,
        fee,
        token,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct SubscriptionCancelledEvent {
    pub subscription_id: u64,
    pub caller: Address,
    pub timestamp: u64,
}

pub fn publish_subscription_cancelled_event(
    env: &Env,
    subscription_id: u64,
    caller: Address,
    timestamp: u64,
) {
    SubscriptionCancelledEvent {
        subscription_id,
        caller,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct PlanDeactivatedEvent {
    pub plan_id: u64,
    pub merchant: Address,
    pub timestamp: u64,
}

pub fn publish_plan_deactivated_event(env: &Env, plan_id: u64, merchant: Address, timestamp: u64) {
    PlanDeactivatedEvent {
        plan_id,
        merchant,
        timestamp,
    }
    .publish(env);
}

/// Emitted when a subscription plan query is executed.
#[contractevent]
pub struct PlanSearchExecutedEvent {
    pub caller: Address,
    pub result_count: u32,
    pub timestamp: u64,
}

pub fn publish_subscription_plan_search_event(
    env: &Env,
    caller: Address,
    result_count: u32,
    timestamp: u64,
) {
    PlanSearchExecutedEvent {
        caller,
        result_count,
        timestamp,
    }
    .publish(env);
}
