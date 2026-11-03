use crate::from_shade::events;
use shade::types::{Invoice, InvoiceFilter, InvoicePage, Merchant, MerchantFilter, MerchantPage, PageInfo};
use crate::from_shade::types::{DataKey, Event, EventFilter, EventKey, MultiSigKey, Subscription, SubscriptionFilter, SubscriptionPlan, SubscriptionPlanFilter, WithdrawalProposal, WithdrawalProposalFilter};
use soroban_sdk::{Address, Env, Vec};

/// Return all subscription plans matching `filter`.
pub fn search_subscription_plans(
    env: &Env,
    caller: &Address,
    filter: SubscriptionPlanFilter,
) -> Vec<SubscriptionPlan> {
    let plan_count: u64 = env
        .storage()
        .persistent()
        .get(&DataKey::PlanCount)
        .unwrap_or(0);

    let mut results: Vec<SubscriptionPlan> = Vec::new(env);

    let merchant_id_opt: Option<u64> = filter.merchant.as_ref().and_then(|addr| {
        env.storage()
            .persistent()
            .get::<_, u64>(&shade::types::DataKey::MerchantId(addr.clone()))
    });

    for i in 1..=plan_count {
        if let Some(plan) = env
            .storage()
            .persistent()
            .get::<_, SubscriptionPlan>(&DataKey::SubscriptionPlan(i))
        {
            let mut ok = true;
            if let Some(mid) = merchant_id_opt {
                if plan.merchant_id != mid {
                    ok = false;
                }
            }
            if ok {
                if let Some(active) = filter.active {
                    if plan.active != active {
                        ok = false;
                    }
                }
            }
            if ok {
                if let Some(ref tok) = filter.token {
                    if plan.token != *tok {
                        ok = false;
                    }
                }
            }
            if ok {
                results.push_back(plan);
            }
        }
    }

    let count = results.len();
    events::publish_subscription_plan_search_event(
        env,
        caller.clone(),
        count,
        env.ledger().timestamp(),
    );

    results
}

/// Return all subscriptions matching `filter`.
pub fn search_subscriptions(env: &Env, filter: SubscriptionFilter) -> Vec<Subscription> {
    let sub_count: u64 = env
        .storage()
        .persistent()
        .get(&DataKey::SubscriptionCount)
        .unwrap_or(0);

    let mut results: Vec<Subscription> = Vec::new(env);

    for i in 1..=sub_count {
        if let Some(sub) = env
            .storage()
            .persistent()
            .get::<_, Subscription>(&DataKey::Subscription(i))
        {
            let mut ok = true;
            if let Some(plan_id) = filter.plan_id {
                if sub.plan_id != plan_id {
                    ok = false;
                }
            }
            if ok {
                if let Some(ref customer) = filter.customer {
                    if sub.customer != *customer {
                        ok = false;
                    }
                }
            }
            if ok {
                if let Some(status_u32) = filter.status {
                    let sub_status_u32 = sub.status as u32;
                    if sub_status_u32 != status_u32 {
                        ok = false;
                    }
                }
            }
            if ok {
                results.push_back(sub);
            }
        }
    }

    results
}
