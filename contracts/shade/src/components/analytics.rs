//! Merchant and token payment analytics, updated on every routed payment.

use crate::components::admin;
use crate::types::{DataKey, MerchantAnalytics, MerchantAnalyticsSummary, TokenAnalytics};
use soroban_sdk::{Address, Env, Vec};

pub fn get_merchant_volume(env: &Env, merchant: &Address, token: &Address) -> i128 {
    get_merchant_analytics(env, merchant, token).total_volume
}

pub fn get_merchant_analytics(env: &Env, merchant: &Address, token: &Address) -> MerchantAnalytics {
    env.storage()
        .persistent()
        .get(&DataKey::MerchantAnalytics(merchant.clone(), token.clone()))
        .unwrap_or(MerchantAnalytics {
            merchant: merchant.clone(),
            token: token.clone(),
            total_volume: env
                .storage()
                .persistent()
                .get(&DataKey::MerchantVolume(merchant.clone(), token.clone()))
                .unwrap_or(0),
            total_fees: 0,
            transaction_count: 0,
            last_updated: 0,
        })
}

pub fn get_merchant_analytics_summary(env: &Env, merchant: &Address) -> MerchantAnalyticsSummary {
    env.storage()
        .persistent()
        .get(&DataKey::MerchantAnalyticsSummary(merchant.clone()))
        .unwrap_or(MerchantAnalyticsSummary {
            merchant: merchant.clone(),
            total_volume: 0,
            total_fees: 0,
            transaction_count: 0,
            last_updated: 0,
        })
}

pub fn record_merchant_payment(
    env: &Env,
    merchant: &Address,
    token: &Address,
    volume_amount: i128,
    fee_amount: i128,
) {
    let mut analytics = get_merchant_analytics(env, merchant, token);
    analytics.total_volume += volume_amount;
    analytics.total_fees += fee_amount;
    analytics.transaction_count += 1;
    analytics.last_updated = env.ledger().timestamp();

    env.storage().persistent().set(
        &DataKey::MerchantAnalytics(merchant.clone(), token.clone()),
        &analytics,
    );
    env.storage().persistent().set(
        &DataKey::MerchantVolume(merchant.clone(), token.clone()),
        &analytics.total_volume,
    );

    let mut summary = get_merchant_analytics_summary(env, merchant);
    summary.total_volume += volume_amount;
    summary.total_fees += fee_amount;
    summary.transaction_count += 1;
    summary.last_updated = analytics.last_updated;

    env.storage().persistent().set(
        &DataKey::MerchantAnalyticsSummary(merchant.clone()),
        &summary,
    );

    // Update global token analytics
    record_token_payment(env, token, volume_amount, fee_amount);
}

pub fn get_token_analytics(env: &Env, token: &Address) -> TokenAnalytics {
    env.storage()
        .persistent()
        .get(&DataKey::TokenAnalytics(token.clone()))
        .unwrap_or(TokenAnalytics {
            token: token.clone(),
            total_volume: 0,
            total_fees: 0,
            transaction_count: 0,
            unique_merchants: 0,
            last_updated: 0,
        })
}

pub fn get_token_volume(env: &Env, token: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::TokenVolume(token.clone()))
        .unwrap_or(0)
}

fn record_token_payment(env: &Env, token: &Address, volume_amount: i128, fee_amount: i128) {
    let mut analytics = get_token_analytics(env, token);

    // Check if this is a new merchant for this token
    let current_volume = get_token_volume(env, token);
    let is_new_merchant = current_volume == 0;

    analytics.total_volume += volume_amount;
    analytics.total_fees += fee_amount;
    analytics.transaction_count += 1;
    if is_new_merchant {
        analytics.unique_merchants += 1;
    }
    analytics.last_updated = env.ledger().timestamp();

    env.storage()
        .persistent()
        .set(&DataKey::TokenAnalytics(token.clone()), &analytics);

    env.storage().persistent().set(
        &DataKey::TokenVolume(token.clone()),
        &analytics.total_volume,
    );
}

pub fn get_token_dominance_metrics(env: &Env, tokens: &Vec<Address>) -> Vec<(Address, i128)> {
    // Build the (token, volume) list in a soroban Vec. We avoid `std::vec::Vec`
    // because the contract is `#![no_std]`.
    let mut result: Vec<(Address, i128)> = Vec::new(env);
    for token in tokens.iter() {
        let volume = get_token_volume(env, &token);
        result.push_back((token, volume));
    }

    // Insertion sort by volume descending. n is small (one entry per accepted
    // token) so quadratic cost is fine and avoids allocator dependencies.
    let n = result.len();
    let mut i: u32 = 1;
    while i < n {
        let mut j: u32 = i;
        while j > 0 {
            let prev = result.get_unchecked(j - 1);
            let curr = result.get_unchecked(j);
            if curr.1 > prev.1 {
                result.set(j - 1, curr);
                result.set(j, prev);
                j -= 1;
            } else {
                break;
            }
        }
        i += 1;
    }

    result
}

pub fn get_top_tokens_by_volume(env: &Env, limit: u32) -> Vec<(Address, i128)> {
    let accepted_tokens = admin::get_accepted_tokens(env);
    let mut all_metrics = get_token_dominance_metrics(env, &accepted_tokens);

    // Truncate to specified limit
    while all_metrics.len() > limit {
        all_metrics.pop_back();
    }

    all_metrics
}

pub fn get_token_market_share(env: &Env, token: &Address) -> i128 {
    let token_volume = get_token_volume(env, token);
    if token_volume == 0 {
        return 0;
    }

    let accepted_tokens = admin::get_accepted_tokens(env);
    let mut total_volume: i128 = 0;

    for t in accepted_tokens.iter() {
        total_volume += get_token_volume(env, &t);
    }

    if total_volume == 0 {
        return 0;
    }

    // Return market share as basis points (10000 = 100%)
    (token_volume * 10000) / total_volume
}
