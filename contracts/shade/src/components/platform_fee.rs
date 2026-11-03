use crate::components::{access_control, admin, analytics, core, merchant, reentrancy};
use crate::errors::ContractError;
use crate::events;
use crate::types::{DataKey, PendingFee, PlatformFeeRouteKind, PlatformFeeSplit, Role};
use soroban_sdk::{panic_with_error, token, Address, Env};

const MAX_FEE_BPS: i128 = 10_000;
pub const FEE_UPDATE_DELAY: u64 = 172_800; // 48 hours in seconds

fn assert_fee_operator(env: &Env, caller: &Address) {
    caller.require_auth();
    if !access_control::has_role(env, caller, Role::Admin)
        && !access_control::has_role(env, caller, Role::Manager)
    {
        panic_with_error!(env, ContractError::NotAuthorized);
    }
}

fn apply_volume_discount(fee_bps: i128, volume: i128) -> i128 {
    let discount_percentage = if volume >= 200_000 {
        50
    } else if volume >= 50_000 {
        25
    } else if volume >= 10_000 {
        10
    } else {
        0
    };

    if discount_percentage == 0 {
        fee_bps
    } else {
        (fee_bps * (100 - discount_percentage)) / 100
    }
}

pub fn effective_fee_bps(env: &Env, merchant_id: u64, merchant: &Address, token: &Address) -> i128 {
    let base_bps =
        get_merchant_platform_fee(env, merchant_id, token).unwrap_or_else(|| get_fee(env, token));
    apply_volume_discount(
        base_bps,
        analytics::get_merchant_volume(env, merchant, token),
    )
}

pub fn compute_split(
    env: &Env,
    merchant: &Address,
    token: &Address,
    amount: i128,
) -> PlatformFeeSplit {
    if amount <= 0 {
        panic_with_error!(env, ContractError::InvalidAmount);
    }

    let merchant_id = merchant::get_merchant_id(env, merchant);
    let fee_bps = effective_fee_bps(env, merchant_id, merchant, token);
    let platform_fee = if fee_bps == 0 {
        0
    } else {
        (amount * fee_bps) / MAX_FEE_BPS
    };

    if platform_fee >= amount {
        panic_with_error!(env, ContractError::InvalidAmount);
    }

    PlatformFeeSplit {
        gross_amount: amount,
        platform_fee,
        merchant_amount: amount - platform_fee,
        fee_bps_applied: fee_bps,
    }
}

pub fn route_from_payer(
    env: &Env,
    payer: &Address,
    merchant: &Address,
    merchant_account: &Address,
    token: &Address,
    amount: i128,
    route_kind: PlatformFeeRouteKind,
    ref_id: u64,
    merchant_id: u64,
) -> PlatformFeeSplit {
    let split = compute_split(env, merchant, token, amount);
    execute_split_transfers(env, payer, merchant_account, token, &split, false);
    finalize_route(
        env,
        merchant,
        token,
        amount,
        &split,
        route_kind,
        ref_id,
        merchant_id,
        payer,
    );
    split
}

pub fn route_from_allowance(
    env: &Env,
    customer: &Address,
    merchant: &Address,
    merchant_account: &Address,
    token: &Address,
    amount: i128,
    route_kind: PlatformFeeRouteKind,
    ref_id: u64,
    merchant_id: u64,
) -> PlatformFeeSplit {
    let split = compute_split(env, merchant, token, amount);
    execute_split_transfers(env, customer, merchant_account, token, &split, true);
    finalize_route(
        env,
        merchant,
        token,
        amount,
        &split,
        route_kind,
        ref_id,
        merchant_id,
        customer,
    );
    split
}

fn execute_split_transfers(
    env: &Env,
    source: &Address,
    merchant_account: &Address,
    token: &Address,
    split: &PlatformFeeSplit,
    from_allowance: bool,
) {
    let platform_account = admin::get_platform_account(env);
    let token_client = token::TokenClient::new(env, token);

    if from_allowance {
        let spender = env.current_contract_address();
        token_client.transfer_from(&spender, source, merchant_account, &split.merchant_amount);
        if split.platform_fee > 0 {
            token_client.transfer_from(&spender, source, &platform_account, &split.platform_fee);
        }
    } else {
        token_client.transfer(source, merchant_account, &split.merchant_amount);
        if split.platform_fee > 0 {
            token_client.transfer(source, &platform_account, &split.platform_fee);
        }
    }
}

fn finalize_route(
    env: &Env,
    merchant: &Address,
    token: &Address,
    gross_amount: i128,
    split: &PlatformFeeSplit,
    route_kind: PlatformFeeRouteKind,
    ref_id: u64,
    merchant_id: u64,
    payer: &Address,
) {
    analytics::record_merchant_payment(env, merchant, token, gross_amount, split.platform_fee);

    let platform_account = admin::get_platform_account(env);
    let merchant_account = merchant::get_merchant_account(env, merchant_id);
    let timestamp = env.ledger().timestamp();

    events::publish_platform_fee_routed_event(
        env,
        route_kind,
        ref_id,
        merchant_id,
        merchant.clone(),
        merchant_account,
        platform_account,
        payer.clone(),
        split.gross_amount,
        split.platform_fee,
        split.merchant_amount,
        token.clone(),
        split.fee_bps_applied,
        timestamp,
    );
}

pub fn set_merchant_platform_fee(
    env: &Env,
    caller: &Address,
    merchant_id: u64,
    token: &Address,
    fee_bps: i128,
) {
    reentrancy::enter(env);
    assert_fee_operator(env, caller);

    if !(0..=MAX_FEE_BPS).contains(&fee_bps) {
        panic_with_error!(env, ContractError::InvalidAmount);
    }
    if !admin::is_accepted_token(env, token) {
        panic_with_error!(env, ContractError::TokenNotAccepted);
    }

    let _ = merchant::get_merchant(env, merchant_id);

    env.storage().persistent().set(
        &DataKey::MerchantPlatformFee(merchant_id, token.clone()),
        &fee_bps,
    );

    events::publish_merchant_platform_fee_set_event(
        env,
        caller.clone(),
        merchant_id,
        token.clone(),
        fee_bps,
        env.ledger().timestamp(),
    );
    reentrancy::exit(env);
}

pub fn get_merchant_platform_fee(env: &Env, merchant_id: u64, token: &Address) -> Option<i128> {
    env.storage()
        .persistent()
        .get(&DataKey::MerchantPlatformFee(merchant_id, token.clone()))
}

pub fn clear_merchant_platform_fee(env: &Env, caller: &Address, merchant_id: u64, token: &Address) {
    reentrancy::enter(env);
    assert_fee_operator(env, caller);

    let _ = merchant::get_merchant(env, merchant_id);

    env.storage()
        .persistent()
        .remove(&DataKey::MerchantPlatformFee(merchant_id, token.clone()));

    events::publish_merchant_platform_fee_cleared_event(
        env,
        caller.clone(),
        merchant_id,
        token.clone(),
        env.ledger().timestamp(),
    );
    reentrancy::exit(env);
}

pub fn set_fee(env: &Env, admin: &Address, token: &Address, fee: i128) {
    reentrancy::enter(env);
    core::assert_admin(env, admin);

    if !admin::is_accepted_token(env, token) {
        panic_with_error!(env, ContractError::TokenNotAccepted);
    }

    env.storage()
        .persistent()
        .set(&DataKey::TokenFee(token.clone()), &fee);

    events::publish_fee_set_event(
        env,
        admin.clone(),
        token.clone(),
        fee,
        env.ledger().timestamp(),
    );
    reentrancy::exit(env);
}

pub fn get_fee(env: &Env, token: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::TokenFee(token.clone()))
        .unwrap_or(0)
}

/// The platform fee that *would* apply to `amount` for this merchant/token.
///
/// This is a pure query: unlike `platform_fee::compute_split` (the payment
/// path) it never panics. A non-positive amount yields 0, and an address that
/// is not a registered merchant is priced at the token's default fee rather
/// than rejected.
pub fn calculate_fee(env: &Env, merchant: &Address, token: &Address, amount: i128) -> i128 {
    if amount <= 0 {
        return 0;
    }
    let merchant_id = merchant::find_merchant_id(env, merchant).unwrap_or(0);
    let fee_bps = effective_fee_bps(env, merchant_id, merchant, token);
    if fee_bps == 0 {
        0
    } else {
        (amount * fee_bps) / 10_000i128
    }
}

pub fn propose_fee(env: &Env, admin: &Address, token: &Address, fee: i128) {
    reentrancy::enter(env);
    core::assert_admin(env, admin);

    if !admin::is_accepted_token(env, token) {
        panic_with_error!(env, ContractError::TokenNotAccepted);
    }

    let pending = PendingFee {
        token: token.clone(),
        fee,
        proposed_at: env.ledger().timestamp(),
    };

    env.storage()
        .persistent()
        .set(&DataKey::PendingTokenFee(token.clone()), &pending);

    events::publish_fee_proposed_event(
        env,
        admin.clone(),
        token.clone(),
        fee,
        env.ledger().timestamp(),
    );
    reentrancy::exit(env);
}

pub fn execute_fee(env: &Env, admin: &Address, token: &Address) {
    reentrancy::enter(env);
    core::assert_admin(env, admin);

    let pending: PendingFee = env
        .storage()
        .persistent()
        .get(&DataKey::PendingTokenFee(token.clone()))
        .unwrap_or_else(|| panic_with_error!(env, ContractError::NoPendingFeeUpdate));

    let elapsed = env.ledger().timestamp() - pending.proposed_at;
    if elapsed < FEE_UPDATE_DELAY {
        panic_with_error!(env, ContractError::FeeUpdateTooEarly);
    }

    env.storage()
        .persistent()
        .set(&DataKey::TokenFee(token.clone()), &pending.fee);

    env.storage()
        .persistent()
        .remove(&DataKey::PendingTokenFee(token.clone()));

    events::publish_fee_set_event(
        env,
        admin.clone(),
        token.clone(),
        pending.fee,
        env.ledger().timestamp(),
    );
    reentrancy::exit(env);
}

pub fn get_pending_fee(env: &Env, token: &Address) -> PendingFee {
    env.storage()
        .persistent()
        .get(&DataKey::PendingTokenFee(token.clone()))
        .unwrap_or_else(|| panic_with_error!(env, ContractError::NoPendingFeeUpdate))
}
