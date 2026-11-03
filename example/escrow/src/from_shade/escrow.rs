use shade::components::admin;
use shade::components::merchant;
use crate::from_shade::errors::ContractError;
use crate::from_shade::events;
use crate::from_shade::types::{DataKey, Escrow, EscrowStatus};
use soroban_sdk::{panic_with_error, token, Address, Env};

pub fn create_escrow(
    env: &Env,
    seller: &Address,
    buyer: &Address,
    token: &Address,
    amount: i128,
    invoice_id: Option<u64>,
) -> u64 {
    seller.require_auth();

    if amount <= 0 {
        panic_with_error!(env, shade::errors::ContractError::InvalidAmount);
    }

    if !admin::is_accepted_token(env, token) {
        panic_with_error!(env, shade::errors::ContractError::TokenNotAccepted);
    }

    if !merchant::is_merchant(env, seller) {
        panic_with_error!(env, shade::errors::ContractError::MerchantNotFound);
    }

    let id = env
        .storage()
        .persistent()
        .get(&DataKey::EscrowCount)
        .unwrap_or(0u64)
        + 1;

    let escrow = Escrow {
        id,
        buyer: buyer.clone(),
        seller: seller.clone(),
        token: token.clone(),
        amount,
        status: EscrowStatus::Created,
        invoice_id,
        date_created: env.ledger().timestamp(),
        date_funded: Option::None,
        date_released: Option::None,
    };

    env.storage()
        .persistent()
        .set(&DataKey::Escrow(id), &escrow);
    env.storage().persistent().set(&DataKey::EscrowCount, &id);

    events::publish_escrow_created_event(
        env,
        id,
        seller.clone(),
        buyer.clone(),
        token.clone(),
        amount,
        invoice_id,
        env.ledger().timestamp(),
    );

    id
}

pub fn get_escrow(env: &Env, escrow_id: u64) -> Escrow {
    env.storage()
        .persistent()
        .get(&DataKey::Escrow(escrow_id))
        .unwrap_or_else(|| panic_with_error!(env, ContractError::EscrowNotFound))
}

pub fn fund_escrow(env: &Env, buyer: &Address, escrow_id: u64) {
    buyer.require_auth();

    let mut escrow = get_escrow(env, escrow_id);

    if escrow.status != EscrowStatus::Created {
        panic_with_error!(env, ContractError::InvalidEscrowStatus);
    }

    if escrow.buyer != *buyer {
        panic_with_error!(env, shade::errors::ContractError::NotAuthorized);
    }

    let token_client = token::TokenClient::new(env, &escrow.token);
    let contract_address = env.current_contract_address();
    token_client.transfer(buyer, &contract_address, &escrow.amount);

    escrow.status = EscrowStatus::Funded;
    escrow.date_funded = Option::Some(env.ledger().timestamp());

    env.storage()
        .persistent()
        .set(&DataKey::Escrow(escrow_id), &escrow);

    events::publish_escrow_funded_event(
        env,
        escrow_id,
        buyer.clone(),
        escrow.seller.clone(),
        escrow.token.clone(),
        escrow.amount,
        env.ledger().timestamp(),
    );
}

pub fn release_escrow(env: &Env, buyer: &Address, escrow_id: u64) {
    buyer.require_auth();

    let mut escrow = get_escrow(env, escrow_id);

    if escrow.status != EscrowStatus::Funded {
        panic_with_error!(env, ContractError::InvalidEscrowStatus);
    }

    if escrow.buyer != *buyer {
        panic_with_error!(env, shade::errors::ContractError::NotAuthorized);
    }

    let merchant_account =
        merchant::get_merchant_account(env, merchant::get_merchant_id(env, &escrow.seller));
    let token_client = token::TokenClient::new(env, &escrow.token);
    let contract_address = env.current_contract_address();

    let amount = escrow.amount;
    let fee = admin::calculate_fee(env, &escrow.seller, &escrow.token, amount);
    let merchant_amount = amount - fee;

    if merchant_amount > 0 {
        token_client.transfer(&contract_address, &merchant_account, &merchant_amount);
    }

    if fee > 0 {
        let platform_account = admin::get_platform_account(env);
        token_client.transfer(&contract_address, &platform_account, &fee);
    }

    admin::record_merchant_payment(env, &escrow.seller, &escrow.token, amount, fee);

    escrow.status = EscrowStatus::Released;
    escrow.date_released = Option::Some(env.ledger().timestamp());

    env.storage()
        .persistent()
        .set(&DataKey::Escrow(escrow_id), &escrow);

    events::publish_escrow_released_event(
        env,
        escrow_id,
        buyer.clone(),
        escrow.seller.clone(),
        escrow.token.clone(),
        merchant_amount,
        fee,
        env.ledger().timestamp(),
    );
}

pub fn refund_escrow(env: &Env, seller: &Address, escrow_id: u64) {
    seller.require_auth();

    let mut escrow = get_escrow(env, escrow_id);

    if escrow.status != EscrowStatus::Funded {
        panic_with_error!(env, ContractError::InvalidEscrowStatus);
    }

    if escrow.seller != *seller {
        panic_with_error!(env, shade::errors::ContractError::NotAuthorized);
    }

    let token_client = token::TokenClient::new(env, &escrow.token);
    let contract_address = env.current_contract_address();
    token_client.transfer(&contract_address, &escrow.buyer, &escrow.amount);

    escrow.status = EscrowStatus::Refunded;

    env.storage()
        .persistent()
        .set(&DataKey::Escrow(escrow_id), &escrow);

    events::publish_escrow_refunded_event(
        env,
        escrow_id,
        seller.clone(),
        escrow.buyer.clone(),
        escrow.token.clone(),
        escrow.amount,
        env.ledger().timestamp(),
    );
}
