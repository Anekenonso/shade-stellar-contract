use crate::from_shade::errors::ContractError;
use shade::shade::{Shade, ShadeClient};
use crate::from_shade::types::{CrossChainBridgePayload};
use soroban_sdk::testutils::{Address as _, Events as _, Ledger as _};
use soroban_sdk::{Address, Env, Map, String, Symbol, TryIntoVal, Val};

fn setup() -> (Env, ShadeClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    (env, client, contract_id, admin)
}

fn sample_payload(
    env: &Env,
    invoice_id: u64,
    merchant: &Address,
    payer: Option<Address>,
    token: &Address,
    amount: i128,
    memo: Option<String>,
) -> CrossChainBridgePayload {
    CrossChainBridgePayload {
        invoice_id,
        merchant: merchant.clone(),
        payer,
        source_chain: String::from_str(env, "ethereum"),
        destination_chain: String::from_str(env, "stellar"),
        token: token.clone(),
        amount,
        destination_recipient: String::from_str(env, "GABC1234567890"),
        memo,
    }
}

fn assert_bridge_placeholder_event(
    env: &Env,
    contract_id: &Address,
    expected_caller: &Address,
    expected_payload: &CrossChainBridgePayload,
    expected_timestamp: u64,
) {
    let events = env.events().all();
    assert!(!events.is_empty(), "expected bridge placeholder event");

    let mut found = false;
    for i in (0..events.len()).rev() {
        let (event_contract_id, topics, data) = events.get(i).unwrap();
        if event_contract_id != contract_id.clone() || topics.len() != 1 {
            continue;
        }

        let event_name: Symbol = topics.get(0).unwrap().try_into_val(env).unwrap();
        if event_name != Symbol::new(env, "bridge_placeholder_event") {
            continue;
        }

        let data_map: Map<Symbol, Val> = data.try_into_val(env).unwrap();
        let caller_val = data_map.get(Symbol::new(env, "caller")).unwrap();
        let payload_val = data_map.get(Symbol::new(env, "payload")).unwrap();
        let timestamp_val = data_map.get(Symbol::new(env, "timestamp")).unwrap();

        let caller_in_event: Address = caller_val.try_into_val(env).unwrap();
        let payload_in_event: CrossChainBridgePayload = payload_val.try_into_val(env).unwrap();
        let timestamp_in_event: u64 = timestamp_val.try_into_val(env).unwrap();

        assert_eq!(caller_in_event, expected_caller.clone());
        assert_eq!(payload_in_event, expected_payload.clone());
        assert_eq!(timestamp_in_event, expected_timestamp);

        found = true;
        break;
    }

    assert!(found, "bridge_placeholder_event not found");
}

fn count_bridge_placeholder_events(env: &Env, contract_id: &Address) -> u32 {
    let events = env.events().all();
    let mut count = 0_u32;
    for i in 0..events.len() {
        let (event_contract_id, topics, _) = events.get(i).unwrap();
        if event_contract_id != contract_id.clone() || topics.len() != 1 {
            continue;
        }
        let event_name: Symbol = topics.get(0).unwrap().try_into_val(env).unwrap();
        if event_name == Symbol::new(env, "bridge_placeholder_event") {
            count += 1;
        }
    }
    count
}

#[test]
fn test_emit_bridge_placeholder_happy_path_with_full_payload() {
    let (env, client, contract_id, _admin) = setup();
    let caller = Address::generate(&env);
    let merchant = Address::generate(&env);
    let payer = Address::generate(&env);
    let token = Address::generate(&env);
    let memo = String::from_str(&env, "cross-chain settlement");

    let payload = sample_payload(
        &env,
        42,
        &merchant,
        Some(payer.clone()),
        &token,
        5_000,
        Some(memo),
    );

    let timestamp = env.ledger().timestamp();
    client.emit_bridge_placeholder(&caller, &payload);

    assert_bridge_placeholder_event(&env, &contract_id, &caller, &payload, timestamp);
}

#[test]
fn test_emit_bridge_placeholder_happy_path_with_optional_fields_none() {
    let (env, client, contract_id, _admin) = setup();
    let caller = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    let payload = sample_payload(&env, 1, &merchant, None, &token, 100, None);

    let timestamp = env.ledger().timestamp();
    client.emit_bridge_placeholder(&caller, &payload);

    assert_bridge_placeholder_event(&env, &contract_id, &caller, &payload, timestamp);
}

#[test]
fn test_emit_bridge_placeholder_multiple_emissions_increment_events() {
    let (env, client, contract_id, _admin) = setup();
    let caller = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    let payload_a = sample_payload(&env, 10, &merchant, None, &token, 250, None);
    let payload_b = sample_payload(&env, 11, &merchant, None, &token, 750, None);

    // `env.events().all()` only holds the most recent invocation's events, so
    // count immediately after each emit rather than at the end.
    client.emit_bridge_placeholder(&caller, &payload_a);
    assert_eq!(count_bridge_placeholder_events(&env, &contract_id), 1);

    env.ledger().with_mut(|l| l.timestamp += 60);
    client.emit_bridge_placeholder(&caller, &payload_b);
    assert_eq!(count_bridge_placeholder_events(&env, &contract_id), 1);
}

#[test]
fn test_emit_bridge_placeholder_rejects_unauthorized_caller() {
    let (env, client, _contract_id, _admin) = setup();
    let caller = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let payload = sample_payload(&env, 7, &merchant, None, &token, 500, None);

    let admin_before = client.get_admin();

    env.set_auths(&[]);
    let result = client.try_emit_bridge_placeholder(&caller, &payload);

    assert!(result.is_err());
    assert_eq!(client.get_admin(), admin_before);
    // `env.events().all()` returns only the events of the most recent
    // invocation, so assert on that call's output rather than a running total.
    assert!(env.events().all().is_empty());
}

#[test]
fn test_emit_bridge_placeholder_blocked_when_paused() {
    let (env, client, contract_id, admin) = setup();
    client.pause(&admin);

    let caller = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let payload = sample_payload(&env, 99, &merchant, None, &token, 1_000, None);

    let expected_error =
        soroban_sdk::Error::from_contract_error(shade::errors::ContractError::ContractPaused as u32);

    let result = client.try_emit_bridge_placeholder(&caller, &payload);

    assert!(matches!(result, Err(Ok(err)) if err == expected_error));
    assert!(client.is_paused());
    assert_eq!(count_bridge_placeholder_events(&env, &contract_id), 0);
    // `env.events().all()` returns only the events of the most recent
    // invocation, so assert on that call's output rather than a running total.
    assert!(env.events().all().is_empty());
}

#[test]
fn test_emit_bridge_placeholder_succeeds_after_unpause() {
    let (env, client, contract_id, admin) = setup();
    client.pause(&admin);
    client.unpause(&admin);

    let caller = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let payload = sample_payload(&env, 3, &merchant, None, &token, 300, None);

    let timestamp = env.ledger().timestamp();
    client.emit_bridge_placeholder(&caller, &payload);

    assert_bridge_placeholder_event(&env, &contract_id, &caller, &payload, timestamp);
}

#[test]
fn test_emit_bridge_placeholder_accepts_max_i128_amount() {
    let (env, client, contract_id, _admin) = setup();
    let caller = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    let payload = sample_payload(&env, u64::MAX, &merchant, None, &token, i128::MAX, None);

    let timestamp = env.ledger().timestamp();
    client.emit_bridge_placeholder(&caller, &payload);

    assert_bridge_placeholder_event(&env, &contract_id, &caller, &payload, timestamp);
}

#[test]
fn test_emit_bridge_placeholder_accepts_zero_amount() {
    let (env, client, contract_id, _admin) = setup();
    let caller = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    let payload = sample_payload(&env, 0, &merchant, None, &token, 0, None);

    let timestamp = env.ledger().timestamp();
    client.emit_bridge_placeholder(&caller, &payload);

    assert_bridge_placeholder_event(&env, &contract_id, &caller, &payload, timestamp);
}

#[test]
fn test_emit_bridge_placeholder_accepts_negative_amount_in_payload() {
    let (env, client, contract_id, _admin) = setup();
    let caller = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    let payload = sample_payload(&env, 5, &merchant, None, &token, -1, None);

    let timestamp = env.ledger().timestamp();
    client.emit_bridge_placeholder(&caller, &payload);

    assert_bridge_placeholder_event(&env, &contract_id, &caller, &payload, timestamp);
}

#[test]
fn test_failed_emit_does_not_mutate_contract_state() {
    let (env, client, contract_id, admin) = setup();
    let caller = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let payload = sample_payload(&env, 20, &merchant, None, &token, 400, None);

    let admin_before = client.get_admin();
    let events_before = count_bridge_placeholder_events(&env, &contract_id);

    client.pause(&admin);
    let _ = client.try_emit_bridge_placeholder(&caller, &payload);

    assert_eq!(client.get_admin(), admin_before);
    assert!(client.is_paused());
    assert_eq!(
        count_bridge_placeholder_events(&env, &contract_id),
        events_before
    );
}

#[test]
fn test_successful_emit_leaves_persistent_storage_unchanged() {
    let (env, client, contract_id, _admin) = setup();
    let caller = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let payload = sample_payload(&env, 88, &merchant, None, &token, 900, None);

    let admin_before = client.get_admin();
    let paused_before = client.is_paused();

    client.emit_bridge_placeholder(&caller, &payload);
    // `env.events().all()` only holds the most recent invocation's events, so
    // count immediately after each emit rather than at the end.
    assert_eq!(count_bridge_placeholder_events(&env, &contract_id), 1);

    assert_eq!(client.get_admin(), admin_before);
    assert_eq!(client.is_paused(), paused_before);
}
