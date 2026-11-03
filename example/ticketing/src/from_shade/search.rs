use crate::from_shade::events;
use shade::types::{Invoice, InvoiceFilter, InvoicePage, Merchant, MerchantFilter, MerchantPage, PageInfo};
use crate::from_shade::types::{DataKey, Event, EventFilter, EventKey, MultiSigKey, Subscription, SubscriptionFilter, SubscriptionPlan, SubscriptionPlanFilter, WithdrawalProposal, WithdrawalProposalFilter};
use soroban_sdk::{Address, Env, Vec};

/// Return all on-chain events matching `filter`.
pub fn search_events(env: &Env, caller: &Address, filter: EventFilter) -> Vec<Event> {
    let event_count: u64 = env
        .storage()
        .persistent()
        .get(&EventKey::EventCount)
        .unwrap_or(0);

    let mut results: Vec<Event> = Vec::new(env);

    let merchant_id_opt: Option<u64> = filter.merchant.as_ref().and_then(|addr| {
        env.storage()
            .persistent()
            .get::<_, u64>(&shade::types::DataKey::MerchantId(addr.clone()))
    });

    for i in 1..=event_count {
        if let Some(evt) = env
            .storage()
            .persistent()
            .get::<_, Event>(&EventKey::Event(i))
        {
            let mut ok = true;
            if let Some(mid) = merchant_id_opt {
                if evt.merchant_id != mid {
                    ok = false;
                }
            }
            if ok {
                if let Some(cancelled) = filter.cancelled {
                    if evt.cancelled != cancelled {
                        ok = false;
                    }
                }
            }
            if ok {
                if let Some(start) = filter.start_date {
                    if evt.event_date < start {
                        ok = false;
                    }
                }
            }
            if ok {
                if let Some(end) = filter.end_date {
                    if evt.event_date > end {
                        ok = false;
                    }
                }
            }
            if ok {
                if let Some(min_avail) = filter.min_available {
                    let available = evt.capacity.saturating_sub(evt.sold);
                    if available < min_avail {
                        ok = false;
                    }
                }
            }
            if ok {
                results.push_back(evt);
            }
        }
    }

    let count = results.len();
    events::publish_event_search_executed_event(
        env,
        caller.clone(),
        count,
        env.ledger().timestamp(),
    );

    results
}
