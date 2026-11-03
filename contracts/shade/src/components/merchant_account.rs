//! The hub's side of each merchant's `account` contract. The hub deploys the
//! account and is its manager, so the actions an account reserves for its manager
//! can only be reached through the entry points here.

use crate::components::core as core_component;
use crate::components::{access_control, account_factory, admin, merchant};
use crate::errors::ContractError;
use crate::events;
use crate::types::Role;
use soroban_sdk::{contractclient, panic_with_error, Address, Env};

#[contractclient(name = "MerchantAccountClient")]
pub trait MerchantAccountContract {
    fn restrict_account(env: Env, status: bool);
    fn verify_account(env: Env);
    fn add_token(env: Env, token: Address);
    fn set_withdrawal_threshold(env: Env, threshold: i128);
    fn approve_withdrawal(env: Env, request_id: u64);
}

fn assert_account_operator(env: &Env, caller: &Address) {
    caller.require_auth();
    if !access_control::has_role(env, caller, Role::Admin)
        && !access_control::has_role(env, caller, Role::Manager)
    {
        panic_with_error!(env, ContractError::NotAuthorized);
    }
}

fn account_client<'a>(env: &'a Env, merchant_id: u64) -> MerchantAccountClient<'a> {
    MerchantAccountClient::new(env, &merchant::get_merchant_account(env, merchant_id))
}

pub fn deploy_merchant_account(env: &Env, merchant_address: &Address) -> Address {
    merchant_address.require_auth();

    let merchant_id = merchant::get_merchant_id(env, merchant_address);
    if !merchant::is_merchant_active(env, merchant_id) {
        panic_with_error!(env, ContractError::MerchantNotActive);
    }
    if merchant::has_merchant_account(env, merchant_id) {
        panic_with_error!(env, ContractError::MerchantAccountAlreadySet);
    }

    let account = account_factory::deploy_account(env, merchant_address.clone(), merchant_id);
    merchant::link_account(env, merchant_id, &account);
    account
}

pub fn restrict_merchant_account(
    env: &Env,
    caller: &Address,
    merchant_address: &Address,
    status: bool,
) {
    assert_account_operator(env, caller);

    let merchant_id = merchant::get_merchant_id(env, merchant_address);
    account_client(env, merchant_id).restrict_account(&status);

    events::publish_account_restricted_event(
        env,
        merchant_address.clone(),
        status,
        caller.clone(),
        env.ledger().timestamp(),
    );
}

pub fn verify_merchant_account(env: &Env, admin_address: &Address, merchant_id: u64) {
    core_component::assert_admin(env, admin_address);
    account_client(env, merchant_id).verify_account();
}

pub fn add_account_token(env: &Env, merchant_address: &Address, token: &Address) {
    merchant_address.require_auth();

    if !admin::is_accepted_token(env, token) {
        panic_with_error!(env, ContractError::TokenNotAccepted);
    }

    let merchant_id = merchant::get_merchant_id(env, merchant_address);
    account_client(env, merchant_id).add_token(token);
}

pub fn set_account_withdraw_threshold(
    env: &Env,
    caller: &Address,
    merchant_id: u64,
    threshold: i128,
) {
    assert_account_operator(env, caller);
    account_client(env, merchant_id).set_withdrawal_threshold(&threshold);
}

pub fn approve_account_withdrawal(env: &Env, caller: &Address, merchant_id: u64, request_id: u64) {
    assert_account_operator(env, caller);
    account_client(env, merchant_id).approve_withdrawal(&request_id);
}
