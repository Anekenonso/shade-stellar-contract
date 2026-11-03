#![cfg(test)]

use crate::errors::ContractError;
use crate::shade::{Shade, ShadeClient};
use crate::tests::support::LinkMerchantAccount as _;
use crate::types::InvoiceStatus;
use soroban_sdk::testutils::{Address as _, Events as _, Ledger as _};
use soroban_sdk::{token, Address, Env, Map, String, Symbol, TryIntoVal, Val};

const FEE_BPS_5_PERCENT: i128 = 500;

struct Fixture<'a> {
    env: Env,
    client: ShadeClient<'a>,
    contract_id: Address,
    admin: Address,
    token: Address,
}

fn setup() -> Fixture<'static> {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    client.add_accepted_token(&admin, &token);

    Fixture {
        env,
        client,
        contract_id,
        admin,
        token,
    }
}

fn setup_with_fee(fee_bps: i128) -> Fixture<'static> {
    let f = setup();
    f.client.set_fee(&f.admin, &f.token, &fee_bps);
    f
}

fn register_merchant(f: &Fixture) -> (Address, Address) {
    let merchant = Address::generate(&f.env);
    let merchant_account = Address::generate(&f.env);
    f.client.register_merchant(&merchant);
    f.client.link_merchant_account(&merchant, &merchant_account);
    (merchant, merchant_account)
}

fn mint(f: &Fixture, to: &Address, amount: i128) {
    token::StellarAssetClient::new(&f.env, &f.token).mint(to, &amount);
}

fn assert_platform_account_set_event(
    env: &Env,
    contract_id: &Address,
    expected_admin: &Address,
    expected_account: &Address,
    expected_timestamp: u64,
) {
    let events = env.events().all();
    assert!(!events.is_empty());

    let (event_contract_id, topics, data) = events.get(events.len() - 1).unwrap();
    assert_eq!(event_contract_id, contract_id.clone());
    assert_eq!(topics.len(), 1);

    let event_name: Symbol = topics.get(0).unwrap().try_into_val(env).unwrap();
    assert_eq!(event_name, Symbol::new(env, "platform_account_set_event"));

    let data_map: Map<Symbol, Val> = data.try_into_val(env).unwrap();
    let admin_val = data_map.get(Symbol::new(env, "admin")).unwrap();
    let account_val = data_map.get(Symbol::new(env, "account")).unwrap();
    let timestamp_val = data_map.get(Symbol::new(env, "timestamp")).unwrap();

    let admin_in_event: Address = admin_val.try_into_val(env).unwrap();
    let account_in_event: Address = account_val.try_into_val(env).unwrap();
    let timestamp_in_event: u64 = timestamp_val.try_into_val(env).unwrap();

    assert_eq!(admin_in_event, expected_admin.clone());
    assert_eq!(account_in_event, expected_account.clone());
    assert_eq!(timestamp_in_event, expected_timestamp);
}

#[allow(clippy::too_many_arguments)]
fn assert_payment_split_routed_event(
    env: &Env,
    contract_id: &Address,
    expected_invoice_id: u64,
    expected_merchant_account: &Address,
    expected_platform_account: &Address,
    expected_merchant_amount: i128,
    expected_platform_amount: i128,
    expected_token: &Address,
    expected_timestamp: u64,
) {
    let events = env.events().all();
    let mut found = false;

    for i in 0..events.len() {
        let (event_contract_id, topics, data) = events.get(i).unwrap();
        if event_contract_id != contract_id.clone() {
            continue;
        }
        let event_name: Symbol = topics.get(0).unwrap().try_into_val(env).unwrap();
        if event_name != Symbol::new(env, "payment_split_routed_event") {
            continue;
        }

        let data_map: Map<Symbol, Val> = data.try_into_val(env).unwrap();
        let invoice_id: u64 = data_map
            .get(Symbol::new(env, "invoice_id"))
            .unwrap()
            .try_into_val(env)
            .unwrap();
        let merchant_account: Address = data_map
            .get(Symbol::new(env, "merchant_account"))
            .unwrap()
            .try_into_val(env)
            .unwrap();
        let platform_account: Address = data_map
            .get(Symbol::new(env, "platform_account"))
            .unwrap()
            .try_into_val(env)
            .unwrap();
        let merchant_amount: i128 = data_map
            .get(Symbol::new(env, "merchant_amount"))
            .unwrap()
            .try_into_val(env)
            .unwrap();
        let platform_amount: i128 = data_map
            .get(Symbol::new(env, "platform_amount"))
            .unwrap()
            .try_into_val(env)
            .unwrap();
        let token: Address = data_map
            .get(Symbol::new(env, "token"))
            .unwrap()
            .try_into_val(env)
            .unwrap();
        let timestamp: u64 = data_map
            .get(Symbol::new(env, "timestamp"))
            .unwrap()
            .try_into_val(env)
            .unwrap();

        assert_eq!(invoice_id, expected_invoice_id);
        assert_eq!(merchant_account, expected_merchant_account.clone());
        assert_eq!(platform_account, expected_platform_account.clone());
        assert_eq!(merchant_amount, expected_merchant_amount);
        assert_eq!(platform_amount, expected_platform_amount);
        assert_eq!(token, expected_token.clone());
        assert_eq!(timestamp, expected_timestamp);
        found = true;
        break;
    }

    assert!(found, "payment_split_routed_event not found");
}

// ── Platform account configuration ───────────────────────────────────────────

#[test]
fn test_get_platform_account_defaults_to_admin() {
    let f = setup();
    assert_eq!(f.client.get_platform_account(), f.admin);
}

#[test]
fn test_set_platform_account_success_and_event() {
    let f = setup();
    let platform = Address::generate(&f.env);
    let expected_timestamp = f.env.ledger().timestamp();

    f.client.set_platform_account(&f.admin, &platform);

    assert_platform_account_set_event(
        &f.env,
        &f.contract_id,
        &f.admin,
        &platform,
        expected_timestamp,
    );
    assert_eq!(f.client.get_platform_account(), platform);
}

#[test]
fn test_set_platform_account_unauthorized() {
    let f = setup();
    let platform = Address::generate(&f.env);
    let attacker = Address::generate(&f.env);

    let expected_error =
        soroban_sdk::Error::from_contract_error(ContractError::NotAuthorized as u32);

    let result = f.client.try_set_platform_account(&attacker, &platform);
    assert!(matches!(result, Err(Ok(err)) if err == expected_error));
    assert_eq!(f.client.get_platform_account(), f.admin);
}

#[test]
fn test_update_platform_account() {
    let f = setup();
    let platform_a = Address::generate(&f.env);
    let platform_b = Address::generate(&f.env);

    f.client.set_platform_account(&f.admin, &platform_a);
    assert_eq!(f.client.get_platform_account(), platform_a);

    f.client.set_platform_account(&f.admin, &platform_b);
    assert_eq!(f.client.get_platform_account(), platform_b);
}

// ── Invoice payment fee routing (happy path) ─────────────────────────────────

#[test]
fn test_invoice_payment_routes_fee_to_platform_account() {
    let f = setup_with_fee(FEE_BPS_5_PERCENT);
    let platform = Address::generate(&f.env);
    f.client.set_platform_account(&f.admin, &platform);

    let (merchant, merchant_account) = register_merchant(&f);
    let amount: i128 = 10_000;
    let fee = amount * FEE_BPS_5_PERCENT / 10_000;
    let merchant_amount = amount - fee;

    let invoice_id = f.client.create_invoice(
        &merchant,
        &String::from_str(&f.env, "Platform fee invoice"),
        &amount,
        &f.token,
        &None,
    );

    let payer = Address::generate(&f.env);
    mint(&f, &payer, amount);

    let timestamp = f.env.ledger().timestamp();
    f.client.pay_invoice(&payer, &invoice_id);

    assert_payment_split_routed_event(
        &f.env,
        &f.contract_id,
        invoice_id,
        &merchant_account,
        &platform,
        merchant_amount,
        fee,
        &f.token,
        timestamp,
    );

    let tok = token::TokenClient::new(&f.env, &f.token);
    assert_eq!(tok.balance(&platform), fee);
    assert_eq!(tok.balance(&merchant_account), merchant_amount);
    assert_eq!(tok.balance(&payer), 0);
    assert_eq!(tok.balance(&f.contract_id), 0);

    let invoice = f.client.get_invoice(&invoice_id);
    assert_eq!(invoice.status, InvoiceStatus::Paid);
}

#[test]
fn test_invoice_payment_zero_fee_skips_platform_transfer() {
    let f = setup_with_fee(0);
    let platform = Address::generate(&f.env);
    f.client.set_platform_account(&f.admin, &platform);

    let (merchant, merchant_account) = register_merchant(&f);
    let amount: i128 = 5_000;

    let invoice_id = f.client.create_invoice(
        &merchant,
        &String::from_str(&f.env, "Zero fee invoice"),
        &amount,
        &f.token,
        &None,
    );

    let payer = Address::generate(&f.env);
    mint(&f, &payer, amount);
    f.client.pay_invoice(&payer, &invoice_id);

    let tok = token::TokenClient::new(&f.env, &f.token);
    assert_eq!(tok.balance(&platform), 0);
    assert_eq!(tok.balance(&merchant_account), amount);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn test_invoice_rejected_when_fee_consumes_whole_amount() {
    let f = setup_with_fee(10_000);
    let (merchant, _merchant_account) = register_merchant(&f);

    f.client.create_invoice(
        &merchant,
        &String::from_str(&f.env, "Max fee invoice"),
        &1_000,
        &f.token,
        &None,
    );
}

#[test]
fn test_partial_payment_routes_fee_per_tranche() {
    let f = setup_with_fee(FEE_BPS_5_PERCENT);
    let platform = Address::generate(&f.env);
    f.client.set_platform_account(&f.admin, &platform);

    let (merchant, merchant_account) = register_merchant(&f);
    let total: i128 = 1_000;
    let first_payment: i128 = 400;
    let second_payment: i128 = 600;

    let invoice_id = f.client.create_invoice(
        &merchant,
        &String::from_str(&f.env, "Partial platform fee invoice"),
        &total,
        &f.token,
        &None,
    );

    let payer = Address::generate(&f.env);
    mint(&f, &payer, total);

    f.client
        .pay_invoice_partial(&payer, &invoice_id, &first_payment);
    f.client
        .pay_invoice_partial(&payer, &invoice_id, &second_payment);

    let total_fee = total * FEE_BPS_5_PERCENT / 10_000;
    let tok = token::TokenClient::new(&f.env, &f.token);
    assert_eq!(tok.balance(&platform), total_fee);
    assert_eq!(tok.balance(&merchant_account), total - total_fee);

    let invoice = f.client.get_invoice(&invoice_id);
    assert_eq!(invoice.status, InvoiceStatus::Paid);
}

// ── Boundary values ──────────────────────────────────────────────────────────

#[test]
fn test_platform_fee_single_unit_amount() {
    let f = setup_with_fee(1);
    let platform = Address::generate(&f.env);
    f.client.set_platform_account(&f.admin, &platform);

    let (merchant, merchant_account) = register_merchant(&f);
    let amount: i128 = 1;
    let fee = 0;

    let invoice_id = f.client.create_invoice(
        &merchant,
        &String::from_str(&f.env, "Single unit"),
        &amount,
        &f.token,
        &None,
    );

    let payer = Address::generate(&f.env);
    mint(&f, &payer, amount);
    f.client.pay_invoice(&payer, &invoice_id);

    let tok = token::TokenClient::new(&f.env, &f.token);
    assert_eq!(tok.balance(&platform), fee);
    assert_eq!(tok.balance(&merchant_account), amount - fee);
}

#[test]
fn test_platform_fee_boundary_at_whole_unit() {
    let f = setup_with_fee(1);
    let platform = Address::generate(&f.env);
    f.client.set_platform_account(&f.admin, &platform);

    let (merchant, merchant_account) = register_merchant(&f);
    let amount: i128 = 10_000;
    let fee = 1;

    let invoice_id = f.client.create_invoice(
        &merchant,
        &String::from_str(&f.env, "Boundary fee"),
        &amount,
        &f.token,
        &None,
    );

    let payer = Address::generate(&f.env);
    mint(&f, &payer, amount);
    f.client.pay_invoice(&payer, &invoice_id);

    let tok = token::TokenClient::new(&f.env, &f.token);
    assert_eq!(tok.balance(&platform), fee);
    assert_eq!(tok.balance(&merchant_account), amount - fee);
}

#[test]
fn test_platform_fee_large_amount_no_overflow() {
    let f = setup_with_fee(250);
    let platform = Address::generate(&f.env);
    f.client.set_platform_account(&f.admin, &platform);

    let (merchant, merchant_account) = register_merchant(&f);
    let amount: i128 = 1_000_000_000;
    let fee = amount * 250 / 10_000;

    let invoice_id = f.client.create_invoice(
        &merchant,
        &String::from_str(&f.env, "Large payment"),
        &amount,
        &f.token,
        &None,
    );

    let payer = Address::generate(&f.env);
    mint(&f, &payer, amount);
    f.client.pay_invoice(&payer, &invoice_id);

    let tok = token::TokenClient::new(&f.env, &f.token);
    assert_eq!(tok.balance(&platform), fee);
    assert_eq!(tok.balance(&merchant_account), amount - fee);
}

// ── Unauthorized access & storage rollback ───────────────────────────────────

#[test]
fn test_pay_invoice_insufficient_funds_rolls_back_storage() {
    let f = setup_with_fee(FEE_BPS_5_PERCENT);
    let platform = Address::generate(&f.env);
    f.client.set_platform_account(&f.admin, &platform);

    let (merchant, merchant_account) = register_merchant(&f);
    let amount: i128 = 1_000;

    let invoice_id = f.client.create_invoice(
        &merchant,
        &String::from_str(&f.env, "Rollback test"),
        &amount,
        &f.token,
        &None,
    );

    let payer = Address::generate(&f.env);
    mint(&f, &payer, 500);

    let result = f.client.try_pay_invoice(&payer, &invoice_id);
    assert!(result.is_err());

    let invoice = f.client.get_invoice(&invoice_id);
    assert_eq!(invoice.status, InvoiceStatus::Pending);
    assert_eq!(invoice.amount_paid, 0);
    assert!(invoice.payer.is_none());

    let tok = token::TokenClient::new(&f.env, &f.token);
    assert_eq!(tok.balance(&payer), 500);
    assert_eq!(tok.balance(&platform), 0);
    assert_eq!(tok.balance(&merchant_account), 0);
}

#[test]
fn test_pay_invoice_expired_rolls_back_storage() {
    let f = setup_with_fee(FEE_BPS_5_PERCENT);
    let platform = Address::generate(&f.env);
    f.client.set_platform_account(&f.admin, &platform);

    let (merchant, merchant_account) = register_merchant(&f);
    let amount: i128 = 1_000;
    let expires_at = 1_000_u64;

    let invoice_id = f.client.create_invoice(
        &merchant,
        &String::from_str(&f.env, "Expired rollback"),
        &amount,
        &f.token,
        &Some(expires_at),
    );

    let payer = Address::generate(&f.env);
    mint(&f, &payer, amount);
    f.env.ledger().set_timestamp(expires_at);

    let result = f.client.try_pay_invoice(&payer, &invoice_id);
    assert!(result.is_err());

    let invoice = f.client.get_invoice(&invoice_id);
    assert_eq!(invoice.status, InvoiceStatus::Pending);

    let tok = token::TokenClient::new(&f.env, &f.token);
    assert_eq!(tok.balance(&payer), amount);
    assert_eq!(tok.balance(&platform), 0);
    assert_eq!(tok.balance(&merchant_account), 0);
}

#[test]
fn test_custom_platform_account_receives_subsequent_payments() {
    let f = setup_with_fee(FEE_BPS_5_PERCENT);
    let platform = Address::generate(&f.env);
    f.client.set_platform_account(&f.admin, &platform);

    let (merchant, merchant_account) = register_merchant(&f);
    let amount: i128 = 2_000;
    let fee = amount * FEE_BPS_5_PERCENT / 10_000;

    let invoice_a = f.client.create_invoice(
        &merchant,
        &String::from_str(&f.env, "Invoice A"),
        &amount,
        &f.token,
        &None,
    );
    let invoice_b = f.client.create_invoice(
        &merchant,
        &String::from_str(&f.env, "Invoice B"),
        &amount,
        &f.token,
        &None,
    );

    let payer = Address::generate(&f.env);
    mint(&f, &payer, amount * 2);

    f.client.pay_invoice(&payer, &invoice_a);
    f.client.pay_invoice(&payer, &invoice_b);

    let tok = token::TokenClient::new(&f.env, &f.token);
    assert_eq!(tok.balance(&platform), fee * 2);
    assert_eq!(tok.balance(&merchant_account), (amount - fee) * 2);
    assert_eq!(tok.balance(&f.admin), 0);
}

#[test]
fn test_merchant_analytics_records_platform_fees() {
    let f = setup_with_fee(FEE_BPS_5_PERCENT);
    let platform = Address::generate(&f.env);
    f.client.set_platform_account(&f.admin, &platform);

    let (merchant, _merchant_account) = register_merchant(&f);
    let amount: i128 = 20_000;
    let fee = amount * FEE_BPS_5_PERCENT / 10_000;

    let invoice_id = f.client.create_invoice(
        &merchant,
        &String::from_str(&f.env, "Analytics fee"),
        &amount,
        &f.token,
        &None,
    );

    let payer = Address::generate(&f.env);
    mint(&f, &payer, amount);
    f.client.pay_invoice(&payer, &invoice_id);

    let analytics = f.client.get_merchant_analytics(&merchant, &f.token);
    assert_eq!(analytics.total_volume, amount);
    assert_eq!(analytics.total_fees, fee);
    assert_eq!(analytics.transaction_count, 1);
}
