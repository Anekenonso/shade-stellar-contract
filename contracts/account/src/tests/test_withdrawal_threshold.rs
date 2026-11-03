#![cfg(test)]

use crate::account::{MerchantAccount, MerchantAccountClient};
use crate::errors::ContractError;
use crate::types::WithdrawalStatus;
use soroban_sdk::testutils::{Address as _, Events as _};
use soroban_sdk::{token, Address, Env, Symbol, TryIntoVal};

struct Fixture<'a> {
    env: Env,
    contract_id: Address,
    client: MerchantAccountClient<'a>,
    merchant: Address,
    manager: Address,
    token: token::Client<'a>,
    recipient: Address,
}

fn setup<'a>() -> Fixture<'a> {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(MerchantAccount, ());
    let client = MerchantAccountClient::new(&env, &contract_id);
    let merchant = Address::generate(&env);
    let manager = Address::generate(&env);
    client.initialize(&merchant, &manager, &1u64);

    let token_id = env
        .register_stellar_asset_contract_v2(Address::generate(&env))
        .address();
    token::StellarAssetClient::new(&env, &token_id).mint(&contract_id, &10_000);
    let token = token::Client::new(&env, &token_id);
    let recipient = Address::generate(&env);

    Fixture {
        env,
        contract_id,
        client,
        merchant,
        manager,
        token,
        recipient,
    }
}

fn emitted(f: &Fixture, name: &str) -> bool {
    let wanted = Symbol::new(&f.env, name);
    f.env.events().all().iter().any(|(contract, topics, _)| {
        let topic: Option<Symbol> = topics.get(0).and_then(|t| t.try_into_val(&f.env).ok());
        contract == f.contract_id && topic == Some(wanted.clone())
    })
}

#[test]
fn test_identity_getters() {
    let f = setup();
    assert_eq!(f.client.get_manager(), f.manager);
    let info = f.client.get_account_info();
    assert_eq!(info.merchant, f.merchant);
    assert_eq!(info.manager, f.manager);
    assert_eq!(info.merchant_id, 1);
}

#[test]
fn test_set_threshold_emits_event() {
    let f = setup();
    f.client.set_withdrawal_threshold(&500);
    assert!(emitted(&f, "withdrawal_threshold_set_event"));
    assert_eq!(f.client.get_withdrawal_threshold(), 500);
}

#[test]
fn test_negative_threshold_is_rejected() {
    let f = setup();
    let result = f.client.try_set_withdrawal_threshold(&-1);
    assert_eq!(result, Err(Ok(ContractError::InvalidThreshold.into())));
}

#[test]
fn test_withdrawal_at_threshold_executes_immediately() {
    let f = setup();
    f.client.set_withdrawal_threshold(&500);
    f.client.withdraw_to(&f.token.address, &500, &f.recipient);
    assert_eq!(f.token.balance(&f.recipient), 500);
}

#[test]
fn test_over_threshold_creates_visible_pending_request() {
    let f = setup();
    f.client.set_withdrawal_threshold(&500);

    f.client.withdraw_to(&f.token.address, &501, &f.recipient);
    assert!(emitted(&f, "pending_withdrawal_created_event"));
    assert_eq!(f.token.balance(&f.recipient), 0);

    let request = f.client.get_withdrawal_request(&1);
    assert_eq!(request.status, WithdrawalStatus::Pending);
    assert_eq!(request.amount, 501);
    assert_eq!(request.approvals.len(), 1);
}

#[test]
fn test_manager_approval_executes_once() {
    let f = setup();
    f.client.set_withdrawal_threshold(&500);
    f.client.withdraw_to(&f.token.address, &501, &f.recipient);

    f.client.approve_withdrawal(&1);
    assert!(emitted(&f, "withdrawal_approved_event"));
    assert!(emitted(&f, "withdrawal_executed_event"));
    assert_eq!(f.token.balance(&f.recipient), 501);
    assert_eq!(
        f.client.get_withdrawal_request(&1).status,
        WithdrawalStatus::Executed
    );

    let again = f.client.try_approve_withdrawal(&1);
    assert_eq!(
        again,
        Err(Ok(ContractError::InvalidWithdrawalStatus.into()))
    );
    assert_eq!(f.token.balance(&f.recipient), 501);
}

#[test]
fn test_approval_is_blocked_while_restricted() {
    let f = setup();
    f.client.set_withdrawal_threshold(&500);
    f.client.withdraw_to(&f.token.address, &501, &f.recipient);

    f.client.restrict_account(&true);
    let result = f.client.try_approve_withdrawal(&1);
    assert_eq!(result, Err(Ok(ContractError::AccountRestricted.into())));
    assert_eq!(f.token.balance(&f.recipient), 0);

    f.client.restrict_account(&false);
    f.client.approve_withdrawal(&1);
    assert_eq!(f.token.balance(&f.recipient), 501);
}

#[test]
fn test_unknown_request_returns_typed_error() {
    let f = setup();
    assert_eq!(
        f.client.try_get_withdrawal_request(&99),
        Err(Ok(ContractError::WithdrawalRequestNotFound.into()))
    );
    assert_eq!(
        f.client.try_approve_withdrawal(&99),
        Err(Ok(ContractError::WithdrawalRequestNotFound.into()))
    );
}
