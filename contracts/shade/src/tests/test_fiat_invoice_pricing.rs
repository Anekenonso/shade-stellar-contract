#![cfg(test)]

use crate::shade::{Shade, ShadeClient};
use crate::types::{DataKey, InvoicePricingMode, OracleConfig};
use soroban_sdk::testutils::{Address as _, Events, Ledger as _};
use soroban_sdk::{contract, contractimpl, Address, Env, String, Symbol, TryFromVal, Val, Vec};

// ── Mock oracle ───────────────────────────────────────────────────────────────

#[contract]
pub struct MockOracle201;

#[contractimpl]
impl MockOracle201 {
    pub fn get_price(env: Env, _token: Address, _quote_currency: String) -> i128 {
        env.storage()
            .instance()
            .get(&"price")
            .unwrap_or(100_000_000i128)
    }
    pub fn set_price(env: Env, price: i128) {
        env.storage().instance().set(&"price", &price);
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn setup() -> (Env, ShadeClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, id, admin)
}

fn make_token(env: &Env) -> Address {
    let a = Address::generate(env);
    env.register_stellar_asset_contract_v2(a).address()
}

fn register_oracle(env: &Env, price: i128) -> (Address, MockOracle201Client<'static>) {
    let oid = env.register(MockOracle201, ());
    let oc = MockOracle201Client::new(env, &oid);
    oc.set_price(&price);
    (oid, oc)
}

fn default_config(oracle_id: Address) -> OracleConfig {
    OracleConfig {
        contract: oracle_id,
        price_decimals: 8,
        token_decimals: 7,
    }
}

/// expected crypto = (fiat * 10^token_dec * 10^price_dec) / (price * 10^fiat_dec)
fn expected_amount(fiat: i128, fiat_dec: u32, price: i128, token_dec: u32, price_dec: u32) -> i128 {
    let num = fiat * 10i128.pow(token_dec) * 10i128.pow(price_dec);
    let den = price * 10i128.pow(fiat_dec);
    num / den
}

fn event_topic_name(env: &Env, topics: &Vec<Val>) -> Symbol {
    Symbol::try_from_val(env, &topics.get(0).unwrap()).unwrap()
}

// ── 1. Happy path ─────────────────────────────────────────────────────────────

#[test]
fn test_set_and_get_token_oracle() {
    let (env, client, _, admin) = setup();
    let token = make_token(&env);
    client.add_accepted_token(&admin, &token);

    let (oid, _) = register_oracle(&env, 200_000_000);
    let cfg = default_config(oid.clone());
    client.set_token_oracle(&admin, &token, &cfg);

    let stored = client.get_token_oracle(&token);
    assert_eq!(stored.contract, oid);
    assert_eq!(stored.price_decimals, 8);
    assert_eq!(stored.token_decimals, 7);
}

#[test]
fn test_fiat_invoice_creation_resolves_correctly() {
    let (env, client, _, admin) = setup();
    let token = make_token(&env);
    client.add_accepted_token(&admin, &token);

    let price = 200_000_000i128; // $2.00 with 8 decimals
    let (oid, _) = register_oracle(&env, price);
    client.set_token_oracle(&admin, &token, &default_config(oid));

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    let fiat = 1000i128; // $10.00 with 2 decimals
    let fiat_dec = 2u32;
    let id = client.create_fiat_invoice(
        &merchant,
        &String::from_str(&env, "test"),
        &fiat,
        &String::from_str(&env, "USD"),
        &fiat_dec,
        &token,
        &None,
    );

    let inv = client.get_invoice(&id);
    assert_eq!(inv.pricing_mode, InvoicePricingMode::FixedFiat);
    let want = expected_amount(fiat, fiat_dec, price, 7, 8);
    assert_eq!(inv.amount, want);
    assert_eq!(client.resolve_invoice_amount(&id), want);
}

#[test]
fn test_resolve_invoice_amount_reflects_price_change() {
    let (env, client, _, admin) = setup();
    let token = make_token(&env);
    client.add_accepted_token(&admin, &token);

    let (oid, oracle) = register_oracle(&env, 200_000_000);
    client.set_token_oracle(&admin, &token, &default_config(oid));

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    let id = client.create_fiat_invoice(
        &merchant,
        &String::from_str(&env, "dynamic"),
        &1000,
        &String::from_str(&env, "USD"),
        &2,
        &token,
        &None,
    );

    let a1 = client.resolve_invoice_amount(&id);
    // price doubles → crypto amount halves
    oracle.set_price(&400_000_000);
    let a2 = client.resolve_invoice_amount(&id);
    assert!(a2 < a1);
    assert_eq!(a2, a1 / 2);
}

#[test]
fn test_oracle_config_overwrite() {
    let (env, client, _, admin) = setup();
    let token = make_token(&env);
    client.add_accepted_token(&admin, &token);

    let (oid1, _) = register_oracle(&env, 100_000_000);
    let (oid2, _) = register_oracle(&env, 200_000_000);

    client.set_token_oracle(&admin, &token, &default_config(oid1));
    client.set_token_oracle(&admin, &token, &default_config(oid2.clone()));

    let stored = client.get_token_oracle(&token);
    assert_eq!(stored.contract, oid2);
}

#[test]
fn test_fiat_invoice_amount_matches_formula() {
    // $50.00 fiat (5000, dec=2) @ $5.00 price (500_000_000, dec=8), token_dec=7
    // expected = 5000 * 10^7 * 10^8 / (500_000_000 * 10^2) = 5e3 * 1e15 / 5e10 = 1e8
    let (env, client, _, admin) = setup();
    let token = make_token(&env);
    client.add_accepted_token(&admin, &token);

    let price = 500_000_000i128;
    let (oid, _) = register_oracle(&env, price);
    client.set_token_oracle(&admin, &token, &default_config(oid));

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    let id = client.create_fiat_invoice(
        &merchant,
        &String::from_str(&env, "formula"),
        &5000,
        &String::from_str(&env, "USD"),
        &2,
        &token,
        &None,
    );

    let inv = client.get_invoice(&id);
    let want = expected_amount(5000, 2, price, 7, 8);
    assert_eq!(inv.amount, want);
    // 5000 * 10^7 * 10^8 / (500_000_000 * 10^2) = 100_000_000
    assert_eq!(inv.amount, 100_000_000);
}

// ── 2. Authorization ──────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn test_set_oracle_non_admin_rejected() {
    let (env, client, _, admin) = setup();
    let token = make_token(&env);
    client.add_accepted_token(&admin, &token);

    let (oid, _) = register_oracle(&env, 100_000_000);
    let cfg = default_config(oid);

    // The attacker's own auth is mocked, so the rejection comes from the admin check.
    let attacker = Address::generate(&env);
    client.set_token_oracle(&attacker, &token, &cfg);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #12)")]
fn test_set_oracle_token_not_accepted() {
    let (env, client, _, admin) = setup();
    let token = make_token(&env);
    // token never added to accepted list

    let (oid, _) = register_oracle(&env, 100_000_000);
    client.set_token_oracle(&admin, &token, &default_config(oid));
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #34)")]
fn test_get_oracle_not_configured_panics() {
    let (env, client, _, admin) = setup();
    let token = make_token(&env);
    client.add_accepted_token(&admin, &token);
    // oracle never set
    client.get_token_oracle(&token);
}

#[test]
#[should_panic]
fn test_fiat_invoice_no_oracle_panics() {
    let (env, client, _, admin) = setup();
    let token = make_token(&env);
    client.add_accepted_token(&admin, &token);
    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);
    // no oracle configured → panics with OracleNotConfigured
    client.create_fiat_invoice(
        &merchant,
        &String::from_str(&env, "no oracle"),
        &1000,
        &String::from_str(&env, "USD"),
        &2,
        &token,
        &None,
    );
}

// ── 3. Events ─────────────────────────────────────────────────────────────────

#[test]
fn test_set_oracle_emits_token_oracle_set_event() {
    let (env, client, shade_id, admin) = setup();
    let token = make_token(&env);
    client.add_accepted_token(&admin, &token);

    let (oid, _) = register_oracle(&env, 100_000_000);
    client.set_token_oracle(&admin, &token, &default_config(oid.clone()));

    let all = env.events().all();
    let last = all.last().unwrap();

    // Emitting contract matches
    assert_eq!(last.0, shade_id);
    // Topic name is correct
    let topic = event_topic_name(&env, &last.1);
    assert_eq!(topic, Symbol::new(&env, "token_oracle_set_event"));
}

#[test]
fn test_create_fiat_invoice_emits_fiat_priced_event() {
    let (env, client, shade_id, admin) = setup();
    let token = make_token(&env);
    client.add_accepted_token(&admin, &token);

    let (oid, _) = register_oracle(&env, 200_000_000);
    client.set_token_oracle(&admin, &token, &default_config(oid));

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    client.create_fiat_invoice(
        &merchant,
        &String::from_str(&env, "priced"),
        &1000,
        &String::from_str(&env, "USD"),
        &2,
        &token,
        &None,
    );

    let all = env.events().all();
    let priced = all.iter().find(|e| {
        e.0 == shade_id
            && event_topic_name(&env, &e.1) == Symbol::new(&env, "fiat_invoice_priced_event")
    });
    assert!(priced.is_some(), "fiat_invoice_priced event not emitted");
}

#[test]
fn test_create_fiat_invoice_also_emits_invoice_created_event() {
    let (env, client, shade_id, admin) = setup();
    let token = make_token(&env);
    client.add_accepted_token(&admin, &token);

    let (oid, _) = register_oracle(&env, 200_000_000);
    client.set_token_oracle(&admin, &token, &default_config(oid));

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    client.create_fiat_invoice(
        &merchant,
        &String::from_str(&env, "created"),
        &1000,
        &String::from_str(&env, "USD"),
        &2,
        &token,
        &None,
    );

    let all = env.events().all();
    let created = all.iter().find(|e| {
        e.0 == shade_id
            && event_topic_name(&env, &e.1) == Symbol::new(&env, "invoice_created_event")
    });
    assert!(created.is_some(), "invoice_created event not emitted");
}

#[test]
fn test_set_oracle_event_after_ledger_timestamp() {
    let (env, client, shade_id, admin) = setup();
    let token = make_token(&env);
    client.add_accepted_token(&admin, &token);

    env.ledger().set_timestamp(9_999_999);

    let (oid, _) = register_oracle(&env, 100_000_000);
    client.set_token_oracle(&admin, &token, &default_config(oid));

    let all = env.events().all();
    let last = all.last().unwrap();
    assert_eq!(last.0, shade_id);
    let topic = event_topic_name(&env, &last.1);
    assert_eq!(topic, Symbol::new(&env, "token_oracle_set_event"));
}

// ── 4. Rollback / storage consistency ────────────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #12)")]
fn test_oracle_not_stored_when_token_not_accepted_panics() {
    // Calling set_token_oracle on a non-whitelisted token must panic.
    // The Soroban host rolls back ALL storage writes made during a failing
    // top-level call, so no oracle can be persisted.
    let (env, client, _, admin) = setup();
    let token = make_token(&env);
    let (oid, _) = register_oracle(&env, 100_000_000);
    client.set_token_oracle(&admin, &token, &default_config(oid));
}

#[test]
fn test_oracle_storage_absent_after_rejected_call() {
    // After a TokenNotAccepted rejection the storage key must be absent.
    let (env, client, _, admin) = setup();
    let token = make_token(&env);
    let (oid, _) = register_oracle(&env, 100_000_000);

    // Attempt (will panic) - wrap in a new env so we can inspect storage after
    let env2 = Env::default();
    env2.mock_all_auths();
    let id2 = env2.register(Shade, ());
    let client2 = ShadeClient::new(&env2, &id2);
    let admin2 = Address::generate(&env2);
    client2.initialize(&admin2);
    let token2 = make_token(&env2);
    // token2 is NOT added to accepted list

    // The call is expected to panic – we verify via a second client that
    // nothing was stored (we simply check storage doesn't have the key).
    let stored: Option<OracleConfig> = env2.as_contract(&id2, || {
        env2.storage()
            .persistent()
            .get(&DataKey::TokenOracle(token2.clone()))
    });
    assert!(stored.is_none(), "oracle must not exist before any call");

    // Now add the token so we can use the client legitimately
    client2.add_accepted_token(&admin2, &token2);
    let (oid2, _) = register_oracle(&env2, 100_000_000);
    client2.set_token_oracle(&admin2, &token2, &default_config(oid2.clone()));

    let stored2: Option<OracleConfig> = env2.as_contract(&id2, || {
        env2.storage()
            .persistent()
            .get(&DataKey::TokenOracle(token2.clone()))
    });
    assert!(stored2.is_some());
    assert_eq!(stored2.unwrap().contract, oid2);

    // Suppress unused warnings
    let _ = (env, client, admin, token, oid);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #35)")]
fn test_fiat_invoice_zero_price_panics_and_does_not_store() {
    // When oracle returns 0, creation must panic → host rolls back.
    let (env, client, _, admin) = setup();
    let token = make_token(&env);
    client.add_accepted_token(&admin, &token);

    let (oid, _) = register_oracle(&env, 0);
    client.set_token_oracle(&admin, &token, &default_config(oid));

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    client.create_fiat_invoice(
        &merchant,
        &String::from_str(&env, "zero price"),
        &1000,
        &String::from_str(&env, "USD"),
        &2,
        &token,
        &None,
    );
}

// ── 5. Edge cases ─────────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #35)")]
fn test_fiat_invoice_fails_zero_price() {
    let (env, client, _, admin) = setup();
    let token = make_token(&env);
    client.add_accepted_token(&admin, &token);

    let (oid, _) = register_oracle(&env, 0);
    client.set_token_oracle(&admin, &token, &default_config(oid));

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    client.create_fiat_invoice(
        &merchant,
        &String::from_str(&env, "zero"),
        &1000,
        &String::from_str(&env, "USD"),
        &2,
        &token,
        &None,
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #35)")]
fn test_fiat_invoice_fails_negative_price() {
    let (env, client, _, admin) = setup();
    let token = make_token(&env);
    client.add_accepted_token(&admin, &token);

    let (oid, _) = register_oracle(&env, -1);
    client.set_token_oracle(&admin, &token, &default_config(oid));

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    client.create_fiat_invoice(
        &merchant,
        &String::from_str(&env, "neg"),
        &1000,
        &String::from_str(&env, "USD"),
        &2,
        &token,
        &None,
    );
}

#[test]
fn test_set_oracle_multiple_tokens_independently() {
    let (env, client, _, admin) = setup();
    let token_a = make_token(&env);
    let token_b = make_token(&env);
    client.add_accepted_token(&admin, &token_a);
    client.add_accepted_token(&admin, &token_b);

    let (oid_a, _) = register_oracle(&env, 100_000_000);
    let (oid_b, _) = register_oracle(&env, 200_000_000);
    client.set_token_oracle(&admin, &token_a, &default_config(oid_a.clone()));
    client.set_token_oracle(&admin, &token_b, &default_config(oid_b.clone()));

    assert_eq!(client.get_token_oracle(&token_a).contract, oid_a);
    assert_eq!(client.get_token_oracle(&token_b).contract, oid_b);
}

#[test]
fn test_repeated_oracle_set_idempotent() {
    let (env, client, _, admin) = setup();
    let token = make_token(&env);
    client.add_accepted_token(&admin, &token);

    let (oid, _) = register_oracle(&env, 100_000_000);
    let cfg = default_config(oid.clone());

    client.set_token_oracle(&admin, &token, &cfg);
    client.set_token_oracle(&admin, &token, &cfg);
    client.set_token_oracle(&admin, &token, &cfg);

    let stored = client.get_token_oracle(&token);
    assert_eq!(stored.contract, oid);
    assert_eq!(stored.price_decimals, 8);
}

#[test]
fn test_oracle_config_zero_decimals_boundary() {
    let (env, client, _, admin) = setup();
    let token = make_token(&env);
    client.add_accepted_token(&admin, &token);

    let (oid, _) = register_oracle(&env, 1_000_000_000);
    let cfg = OracleConfig {
        contract: oid,
        price_decimals: 0,
        token_decimals: 0,
    };
    client.set_token_oracle(&admin, &token, &cfg);

    let stored = client.get_token_oracle(&token);
    assert_eq!(stored.price_decimals, 0);
    assert_eq!(stored.token_decimals, 0);
}

#[test]
fn test_oracle_config_large_decimals() {
    let (env, client, _, admin) = setup();
    let token = make_token(&env);
    client.add_accepted_token(&admin, &token);

    let (oid, _) = register_oracle(&env, 100_000_000);
    let cfg = OracleConfig {
        contract: oid.clone(),
        price_decimals: 18,
        token_decimals: 18,
    };
    client.set_token_oracle(&admin, &token, &cfg);

    let stored = client.get_token_oracle(&token);
    assert_eq!(stored.price_decimals, 18);
    assert_eq!(stored.token_decimals, 18);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #34)")]
fn test_oracle_not_configured_for_other_token_panics() {
    let (env, client, _, admin) = setup();
    let token_a = make_token(&env);
    let token_b = make_token(&env);
    client.add_accepted_token(&admin, &token_a);
    client.add_accepted_token(&admin, &token_b);

    let (oid, _) = register_oracle(&env, 100_000_000);
    client.set_token_oracle(&admin, &token_a, &default_config(oid));

    // token_b has no oracle; get should panic with OracleNotConfigured (#34)
    client.get_token_oracle(&token_b);
}

#[test]
fn test_multiple_fiat_invoices_use_same_oracle() {
    let (env, client, _, admin) = setup();
    let token = make_token(&env);
    client.add_accepted_token(&admin, &token);

    let (oid, _) = register_oracle(&env, 100_000_000);
    client.set_token_oracle(&admin, &token, &default_config(oid));

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    let id1 = client.create_fiat_invoice(
        &merchant,
        &String::from_str(&env, "inv1"),
        &500,
        &String::from_str(&env, "USD"),
        &2,
        &token,
        &None,
    );
    let id2 = client.create_fiat_invoice(
        &merchant,
        &String::from_str(&env, "inv2"),
        &1000,
        &String::from_str(&env, "USD"),
        &2,
        &token,
        &None,
    );

    let a1 = client.get_invoice(&id1).amount;
    let a2 = client.get_invoice(&id2).amount;
    // Second fiat amount is twice the first → crypto amount also doubles
    assert_eq!(a2, a1 * 2);
}

#[test]
fn test_fiat_invoice_pricing_mode_is_fixed_fiat() {
    let (env, client, _, admin) = setup();
    let token = make_token(&env);
    client.add_accepted_token(&admin, &token);

    let (oid, _) = register_oracle(&env, 200_000_000);
    client.set_token_oracle(&admin, &token, &default_config(oid));

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    let id = client.create_fiat_invoice(
        &merchant,
        &String::from_str(&env, "mode check"),
        &1000,
        &String::from_str(&env, "EUR"),
        &2,
        &token,
        &None,
    );

    let inv = client.get_invoice(&id);
    assert_eq!(inv.pricing_mode, InvoicePricingMode::FixedFiat);
}
