use crate::from_shade::events;
use shade::types::{Invoice, InvoiceFilter, InvoicePage, Merchant, MerchantFilter, MerchantPage, PageInfo};
use crate::from_shade::types::{DataKey, Event, EventFilter, EventKey, MultiSigKey, Subscription, SubscriptionFilter, SubscriptionPlan, SubscriptionPlanFilter, WithdrawalProposal, WithdrawalProposalFilter};
use soroban_sdk::{Address, Env, Vec};

/// Return all withdrawal proposals matching `filter`.
pub fn search_withdrawal_proposals(
    env: &Env,
    caller: &Address,
    filter: WithdrawalProposalFilter,
) -> Vec<WithdrawalProposal> {
    let proposal_count: u64 = env
        .storage()
        .persistent()
        .get(&MultiSigKey::WithdrawalProposalCount)
        .unwrap_or(0);

    let mut results: Vec<WithdrawalProposal> = Vec::new(env);

    for i in 1..=proposal_count {
        if let Some(p) = env
            .storage()
            .persistent()
            .get::<_, WithdrawalProposal>(&MultiSigKey::WithdrawalProposal(i))
        {
            let mut ok = true;
            if let Some(ref merchant) = filter.merchant {
                if p.merchant != *merchant {
                    ok = false;
                }
            }
            if ok {
                if let Some(status_u32) = filter.status {
                    let p_status_u32 = p.status as u32;
                    if p_status_u32 != status_u32 {
                        ok = false;
                    }
                }
            }
            if ok {
                if let Some(ref tok) = filter.token {
                    if p.token != *tok {
                        ok = false;
                    }
                }
            }
            if ok {
                if let Some(after) = filter.created_after {
                    if p.created_at < after {
                        ok = false;
                    }
                }
            }
            if ok {
                results.push_back(p);
            }
        }
    }

    let count = results.len();
    events::publish_withdrawal_proposal_search_event(
        env,
        caller.clone(),
        count,
        env.ledger().timestamp(),
    );

    results
}
