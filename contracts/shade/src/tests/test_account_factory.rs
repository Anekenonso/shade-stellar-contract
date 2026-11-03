#![cfg(test)]

use crate::shade::{Shade, ShadeClient};
use crate::types::{InvoiceStatus, Role};
use account::account::MerchantAccountClient;
use account::types::WithdrawalStatus;
use soroban_sdk::testutils::{Address as _, Events as _};
use soroban_sdk::{token, Address, BytesN, Env, String, Symbol, TryIntoVal};

/// The real account contract. Rebuild with `make fixtures` after changing it.
const ACCOUNT_WASM: &[u8] = include_bytes!("fixtures/account.wasm");

struct Fixture<'a> {
    env: Env,
    client: ShadeClient<'a>,
    contract_id: Address,
    admin: Address,
    merchant: Address,
    token: token::Client<'a>,
    minter: token::StellarAssetClient<'a>,
}

fn setup_without_wasm<'a>() -> Fixture<'a> {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_id = env
        .register_stellar_asset_contract_v2(Address::generate(&env))
        .address();
    client.add_accepted_token(&admin, &token_id);
    let token = token::Client::new(&env, &token_id);
    let minter = token::StellarAssetClient::new(&env, &token_id);

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    Fixture {
        env,
        client,
        contract_id,
        admin,
        merchant,
        token,
        minter,
    }
}

fn setup<'a>() -> Fixture<'a> {
    let f = setup_without_wasm();
    let wasm_hash: BytesN<32> = f.env.deployer().upload_contract_wasm(ACCOUNT_WASM);
    f.client.set_account_wasm_hash(&f.admin, &wasm_hash);
    f
}

fn emitted(f: &Fixture, name: &str) -> bool {
    let wanted = Symbol::new(&f.env, name);
    f.env.events().all().iter().any(|(contract, topics, _)| {
        let topic: Option<Symbol> = topics.get(0).and_then(|t| t.try_into_val(&f.env).ok());
        contract == f.contract_id && topic == Some(wanted.clone())
    })
}

fn pay_new_invoice(f: &Fixture, amount: i128) -> u64 {
    let invoice_id = f.client.create_invoice(
        &f.merchant,
        &String::from_str(&f.env, "Order"),
        &amount,
        &f.token.address,
        &None,
    );
    let payer = Address::generate(&f.env);
    f.minter.mint(&payer, &amount);
    f.client.pay_invoice(&payer, &invoice_id);
    invoice_id
}

#[test]
fn test_deploy_links_an_initialized_account_managed_by_the_hub() {
    let f = setup();
    let account = f.client.deploy_merchant_account(&f.merchant);
    assert!(emitted(&f, "merchant_account_deployed_event"));

    assert_eq!(f.client.get_merchant_account(&1u64), account);

    let vault = MerchantAccountClient::new(&f.env, &account);
    assert_eq!(vault.get_merchant(), f.merchant);
    assert_eq!(vault.get_manager(), f.contract_id);
    assert_eq!(vault.get_account_info().merchant_id, 1);
}

#[test]
fn test_each_merchant_gets_its_own_account() {
    let f = setup();
    let other = Address::generate(&f.env);
    f.client.register_merchant(&other);

    let first = f.client.deploy_merchant_account(&f.merchant);
    let second = f.client.deploy_merchant_account(&other);

    assert_ne!(first, second);
    assert_eq!(
        MerchantAccountClient::new(&f.env, &second).get_merchant(),
        other
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #52)")]
fn test_second_deploy_is_rejected() {
    let f = setup();
    f.client.deploy_merchant_account(&f.merchant);
    f.client.deploy_merchant_account(&f.merchant);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #18)")]
fn test_deploy_requires_a_wasm_hash() {
    let f = setup_without_wasm();
    f.client.deploy_merchant_account(&f.merchant);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #6)")]
fn test_deploy_requires_a_registered_merchant() {
    let f = setup();
    let stranger = Address::generate(&f.env);
    f.client.deploy_merchant_account(&stranger);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #32)")]
fn test_deploy_requires_an_active_merchant() {
    let f = setup();
    f.client.set_merchant_status(&f.admin, &1u64, &false);
    f.client.deploy_merchant_account(&f.merchant);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #20)")]
fn test_restrict_requires_a_linked_account() {
    let f = setup();
    f.client
        .restrict_merchant_account(&f.admin, &f.merchant, &true);
}

#[test]
fn test_payment_and_refund_flow_through_the_deployed_account() {
    let f = setup();
    let account = f.client.deploy_merchant_account(&f.merchant);

    let invoice_id = pay_new_invoice(&f, 1_000);
    assert_eq!(f.token.balance(&account), 1_000);

    f.client.refund_invoice(&f.merchant, &invoice_id);
    assert_eq!(f.token.balance(&account), 0);
    assert_eq!(
        f.client.get_invoice(&invoice_id).status,
        InvoiceStatus::Refunded
    );
}

#[test]
fn test_auto_withdrawal_sweeps_from_the_deployed_account() {
    let f = setup();
    let account = f.client.deploy_merchant_account(&f.merchant);
    let recipient = Address::generate(&f.env);
    f.client
        .set_auto_withdrawal_threshold(&f.merchant, &f.token.address, &500);
    f.client
        .set_auto_withdrawal_recipient(&f.merchant, &recipient);

    pay_new_invoice(&f, 1_000);

    assert_eq!(f.token.balance(&recipient), 1_000);
    assert_eq!(f.token.balance(&account), 0);
}

#[test]
fn test_hub_operates_the_accounts_manager_actions() {
    let f = setup();
    let account = f.client.deploy_merchant_account(&f.merchant);
    let vault = MerchantAccountClient::new(&f.env, &account);

    f.client.verify_merchant_account(&f.admin, &1u64);
    assert!(vault.is_verified_account());

    f.client.add_account_token(&f.merchant, &f.token.address);
    assert!(vault.has_token(&f.token.address));

    pay_new_invoice(&f, 1_000);
    assert_eq!(vault.get_balances().get(0).unwrap().balance, 1_000);

    let operator = Address::generate(&f.env);
    f.client.grant_role(&f.admin, &operator, &Role::Manager);
    f.client
        .set_account_withdraw_threshold(&operator, &1u64, &100);
    assert_eq!(vault.get_withdrawal_threshold(), 100);

    let recipient = Address::generate(&f.env);
    vault.withdraw_to(&f.token.address, &600, &recipient);
    assert_eq!(f.token.balance(&recipient), 0);

    f.client
        .restrict_merchant_account(&operator, &f.merchant, &true);
    assert!(f
        .client
        .try_approve_account_withdrawal(&operator, &1u64, &1u64)
        .is_err());

    f.client
        .restrict_merchant_account(&operator, &f.merchant, &false);
    f.client.approve_account_withdrawal(&operator, &1u64, &1u64);
    assert_eq!(f.token.balance(&recipient), 600);
    assert_eq!(
        vault.get_withdrawal_request(&1u64).status,
        WithdrawalStatus::Executed
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn test_threshold_requires_an_operator_role() {
    let f = setup();
    f.client.deploy_merchant_account(&f.merchant);
    let stranger = Address::generate(&f.env);
    f.client
        .set_account_withdraw_threshold(&stranger, &1u64, &100);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn test_merchants_cannot_approve_their_own_withdrawals() {
    let f = setup();
    f.client.deploy_merchant_account(&f.merchant);
    f.client
        .approve_account_withdrawal(&f.merchant, &1u64, &1u64);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #12)")]
fn test_account_token_must_be_whitelisted() {
    let f = setup();
    f.client.deploy_merchant_account(&f.merchant);
    let unknown = f
        .env
        .register_stellar_asset_contract_v2(Address::generate(&f.env))
        .address();
    f.client.add_account_token(&f.merchant, &unknown);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #9)")]
fn test_deploy_is_blocked_while_paused() {
    let f = setup();
    f.client.pause(&f.admin);
    f.client.deploy_merchant_account(&f.merchant);
}
