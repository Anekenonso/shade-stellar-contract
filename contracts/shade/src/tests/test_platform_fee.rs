extern crate std;
use crate::errors::ContractError;
use crate::shade::{Shade, ShadeClient};
use crate::tests::support::LinkMerchantAccount as _;
use crate::types::Role;
use soroban_sdk::testutils::{Address as _, Events as _};
use soroban_sdk::{token, Address, Env, String, Symbol, TryIntoVal};

fn setup() -> (Env, ShadeClient<'static>, Address, Address, Address) {
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

    (env, client, admin, token, contract_id)
}

fn register_merchant_with_account(
    env: &Env,
    client: &ShadeClient,
    token: &Address,
) -> (Address, Address) {
    let merchant = Address::generate(env);
    let merchant_account = merchant.clone();
    client.register_merchant(&merchant);
    client.link_merchant_account(&merchant, &merchant_account);
    client.set_merchant_accepted_tokens(
        &merchant,
        &soroban_sdk::Vec::from_array(env, [token.clone()]),
    );
    (merchant, merchant_account)
}

#[test]
fn compute_platform_fee_split_uses_token_default_fee() {
    let (_env, client, admin, token, _contract_id) = setup();
    let (merchant, _) = register_merchant_with_account(&_env, &client, &token);

    client.set_fee(&admin, &token, &500i128);

    let split = client.compute_platform_fee_split(&merchant, &token, &10_000i128);
    assert_eq!(split.gross_amount, 10_000);
    assert_eq!(split.platform_fee, 500);
    assert_eq!(split.merchant_amount, 9_500);
    assert_eq!(split.fee_bps_applied, 500);
}

#[test]
fn merchant_platform_fee_override_takes_precedence() {
    let (_env, client, admin, token, _contract_id) = setup();
    let (merchant, _) = register_merchant_with_account(&_env, &client, &token);

    client.set_fee(&admin, &token, &1_000i128);
    client.set_merchant_platform_fee(&admin, &1u64, &token, &250i128);

    let split = client.compute_platform_fee_split(&merchant, &token, &10_000i128);
    assert_eq!(split.platform_fee, 250);
    assert_eq!(split.fee_bps_applied, 250);

    let stored = client.get_merchant_platform_fee(&1u64, &token);
    assert_eq!(stored, Some(250));
}

#[test]
fn clear_merchant_platform_fee_reverts_to_token_default() {
    let (_env, client, admin, token, _contract_id) = setup();
    let (merchant, _) = register_merchant_with_account(&_env, &client, &token);

    client.set_fee(&admin, &token, &800i128);
    client.set_merchant_platform_fee(&admin, &1u64, &token, &200i128);
    client.clear_merchant_platform_fee(&admin, &1u64, &token);

    assert_eq!(client.get_merchant_platform_fee(&1u64, &token), None);
    let split = client.compute_platform_fee_split(&merchant, &token, &10_000i128);
    assert_eq!(split.platform_fee, 800);
}

#[test]
fn invoice_payment_routes_platform_fee_via_abstraction() {
    let (env, client, admin, token, contract_id) = setup();
    let (merchant, merchant_account) = register_merchant_with_account(&env, &client, &token);

    client.set_fee(&admin, &token, &1_000i128);

    let invoice_id = client.create_invoice(
        &merchant,
        &String::from_str(&env, "Campaign pledge"),
        &1_000i128,
        &token,
        &None,
    );

    let payer = Address::generate(&env);
    let token_client = token::StellarAssetClient::new(&env, &token);
    token_client.mint(&payer, &5_000i128);

    client.pay_invoice(&payer, &invoice_id);
    // `env.events().all()` only holds the most recent invocation's events, so
    // snapshot them before any further contract calls (balance/getter queries).
    let events = env.events().all();

    let platform = client.get_platform_account();
    assert_eq!(token_client.balance(&merchant_account), 900);
    assert_eq!(token_client.balance(&platform), 100);

    let mut found = false;
    for i in 0..events.len() {
        let (_cid, topics, _data) = events.get(i).unwrap();
        if !topics.is_empty() {
            let name: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
            if name == Symbol::new(&env, "platform_fee_routed_event") {
                found = true;
                break;
            }
        }
    }
    assert!(found);
    let _ = contract_id;
}

#[test]
fn manager_can_set_merchant_platform_fee() {
    let (_env, client, admin, token, _contract_id) = setup();
    let (merchant, _) = register_merchant_with_account(&_env, &client, &token);
    let manager = Address::generate(&_env);
    client.grant_role(&admin, &manager, &Role::Manager);

    client.set_merchant_platform_fee(&manager, &1u64, &token, &300i128);

    let split = client.compute_platform_fee_split(&merchant, &token, &10_000i128);
    assert_eq!(split.platform_fee, 300);
}

#[test]
fn unauthorized_caller_cannot_set_merchant_platform_fee() {
    let (_env, client, _admin, token, _contract_id) = setup();
    let stranger = Address::generate(&_env);

    let expected_error =
        soroban_sdk::Error::from_contract_error(ContractError::NotAuthorized as u32);
    let result = client.try_set_merchant_platform_fee(&stranger, &1u64, &token, &100i128);
    assert!(matches!(result, Err(Ok(err)) if err == expected_error));
}

#[test]
fn batch_invoice_payments_route_fees_concurrently() {
    let (env, client, admin, token, _contract_id) = setup();
    let (merchant, merchant_account) = register_merchant_with_account(&env, &client, &token);
    client.set_fee(&admin, &token, &500i128);

    let payer = Address::generate(&env);
    let token_client = token::StellarAssetClient::new(&env, &token);
    token_client.mint(&payer, &10_000i128);

    let mut invoice_ids = soroban_sdk::Vec::new(&env);
    for _ in 0..3 {
        let id = client.create_invoice(
            &merchant,
            &String::from_str(&env, "Pledge"),
            &1_000i128,
            &token,
            &None,
        );
        invoice_ids.push_back(id);
    }

    client.pay_invoices_batch(&payer, &invoice_ids);

    let platform = client.get_platform_account();
    assert_eq!(token_client.balance(&merchant_account), 2_850);
    assert_eq!(token_client.balance(&platform), 150);
}
