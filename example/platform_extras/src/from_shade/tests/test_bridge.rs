#![cfg(test)]

use shade::shade::{Shade, ShadeClient};
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{Address, BytesN, Env, String};

struct BridgeCtx<'a> {
    env: Env,
    client: ShadeClient<'a>,
    admin: Address,
    token: Address,
}

fn setup() -> BridgeCtx<'static> {
    let env = Env::default();
    env.mock_all_auths();

    let shade_id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &shade_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    client.add_accepted_token(&admin, &token);

    BridgeCtx {
        env,
        client,
        admin,
        token,
    }
}

fn tx_hash(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

// ---------------------------------------------------------------------------
// Listener registration
// ---------------------------------------------------------------------------

#[test]
fn test_admin_registers_and_removes_listener() {
    let ctx = setup();
    let listener = Address::generate(&ctx.env);

    assert!(!ctx.client.is_bridge_listener(&listener));
    ctx.client.register_bridge_listener(&ctx.admin, &listener);
    assert!(ctx.client.is_bridge_listener(&listener));

    ctx.client.remove_bridge_listener(&ctx.admin, &listener);
    assert!(!ctx.client.is_bridge_listener(&listener));
}

/// Registration and removal are idempotent — repeated calls converge on the
/// same state without panicking (concurrency-friendly admin ops).
#[test]
fn test_register_remove_are_idempotent() {
    let ctx = setup();
    let listener = Address::generate(&ctx.env);

    ctx.client.register_bridge_listener(&ctx.admin, &listener);
    ctx.client.register_bridge_listener(&ctx.admin, &listener);
    assert!(ctx.client.is_bridge_listener(&listener));

    ctx.client.remove_bridge_listener(&ctx.admin, &listener);
    ctx.client.remove_bridge_listener(&ctx.admin, &listener);
    assert!(!ctx.client.is_bridge_listener(&listener));
}

/// A non-admin caller cannot register a listener (NotAuthorized, #1).
#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn test_non_admin_cannot_register_listener() {
    let ctx = setup();
    let not_admin = Address::generate(&ctx.env);
    let listener = Address::generate(&ctx.env);
    ctx.client.register_bridge_listener(&not_admin, &listener);
}

// ---------------------------------------------------------------------------
// Recording deposits
// ---------------------------------------------------------------------------

#[test]
fn test_registered_listener_records_deposit() {
    let ctx = setup();
    ctx.env.ledger().set_timestamp(1_700_000_000);

    let listener = Address::generate(&ctx.env);
    let recipient = Address::generate(&ctx.env);
    ctx.client.register_bridge_listener(&ctx.admin, &listener);

    let chain = String::from_str(&ctx.env, "ethereum");
    let id = ctx.client.record_bridge_deposit(
        &listener,
        &chain,
        &tx_hash(&ctx.env, 1),
        &ctx.token,
        &5_000_i128,
        &recipient,
    );

    assert_eq!(id, 1);
    assert_eq!(ctx.client.get_bridge_deposit_count(), 1);

    let deposit = ctx.client.get_bridge_deposit(&id).unwrap();
    assert_eq!(deposit.id, 1);
    assert_eq!(deposit.source_chain, chain);
    assert_eq!(deposit.listener, listener);
    assert_eq!(deposit.token, ctx.token);
    assert_eq!(deposit.amount, 5_000);
    assert_eq!(deposit.recipient, recipient);
    assert_eq!(deposit.timestamp, 1_700_000_000);

    assert!(ctx
        .client
        .is_bridge_deposit_processed(&tx_hash(&ctx.env, 1)));
    assert_eq!(ctx.client.get_bridge_credit(&recipient, &ctx.token), 5_000);
}

/// Credit totals accumulate across multiple distinct deposits to the same
/// recipient/token, while deposit ids increment monotonically.
#[test]
fn test_credit_accumulates_across_deposits() {
    let ctx = setup();
    let listener = Address::generate(&ctx.env);
    let recipient = Address::generate(&ctx.env);
    ctx.client.register_bridge_listener(&ctx.admin, &listener);

    let chain = String::from_str(&ctx.env, "polygon");
    let id1 = ctx.client.record_bridge_deposit(
        &listener,
        &chain,
        &tx_hash(&ctx.env, 1),
        &ctx.token,
        &1_000_i128,
        &recipient,
    );
    let id2 = ctx.client.record_bridge_deposit(
        &listener,
        &chain,
        &tx_hash(&ctx.env, 2),
        &ctx.token,
        &2_500_i128,
        &recipient,
    );

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(ctx.client.get_bridge_credit(&recipient, &ctx.token), 3_500);
}

/// The same origin-chain tx hash can never be credited twice (#55).
#[test]
#[should_panic(expected = "HostError: Error(Contract, #49)")]
fn test_replay_is_rejected() {
    let ctx = setup();
    let listener = Address::generate(&ctx.env);
    let recipient = Address::generate(&ctx.env);
    ctx.client.register_bridge_listener(&ctx.admin, &listener);

    let chain = String::from_str(&ctx.env, "ethereum");
    let hash = tx_hash(&ctx.env, 7);
    ctx.client.record_bridge_deposit(
        &listener,
        &chain,
        &hash,
        &ctx.token,
        &1_000_i128,
        &recipient,
    );
    // Replay with the same source_tx_id.
    ctx.client.record_bridge_deposit(
        &listener,
        &chain,
        &hash,
        &ctx.token,
        &1_000_i128,
        &recipient,
    );
}

/// A caller that is not a registered listener cannot record deposits (#1).
#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn test_unregistered_caller_cannot_record() {
    let ctx = setup();
    let rogue = Address::generate(&ctx.env);
    let recipient = Address::generate(&ctx.env);
    ctx.client.record_bridge_deposit(
        &rogue,
        &String::from_str(&ctx.env, "ethereum"),
        &tx_hash(&ctx.env, 1),
        &ctx.token,
        &1_000_i128,
        &recipient,
    );
}

/// A revoked listener loses the ability to record (#1).
#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn test_revoked_listener_cannot_record() {
    let ctx = setup();
    let listener = Address::generate(&ctx.env);
    let recipient = Address::generate(&ctx.env);
    ctx.client.register_bridge_listener(&ctx.admin, &listener);
    ctx.client.remove_bridge_listener(&ctx.admin, &listener);

    ctx.client.record_bridge_deposit(
        &listener,
        &String::from_str(&ctx.env, "ethereum"),
        &tx_hash(&ctx.env, 1),
        &ctx.token,
        &1_000_i128,
        &recipient,
    );
}

/// Deposits in a non-accepted token are rejected (TokenNotAccepted, #12).
#[test]
#[should_panic(expected = "HostError: Error(Contract, #12)")]
fn test_unaccepted_token_rejected() {
    let ctx = setup();
    let listener = Address::generate(&ctx.env);
    let recipient = Address::generate(&ctx.env);
    ctx.client.register_bridge_listener(&ctx.admin, &listener);

    let other_token = env_token(&ctx.env);
    ctx.client.record_bridge_deposit(
        &listener,
        &String::from_str(&ctx.env, "ethereum"),
        &tx_hash(&ctx.env, 1),
        &other_token,
        &1_000_i128,
        &recipient,
    );
}

/// Non-positive amounts are rejected (InvalidAmount, #7).
#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn test_non_positive_amount_rejected() {
    let ctx = setup();
    let listener = Address::generate(&ctx.env);
    let recipient = Address::generate(&ctx.env);
    ctx.client.register_bridge_listener(&ctx.admin, &listener);

    ctx.client.record_bridge_deposit(
        &listener,
        &String::from_str(&ctx.env, "ethereum"),
        &tx_hash(&ctx.env, 1),
        &ctx.token,
        &0_i128,
        &recipient,
    );
}

/// Fetching an unknown deposit id returns None rather than panicking.
#[test]
fn test_get_unknown_deposit_returns_none() {
    let ctx = setup();
    assert!(ctx.client.get_bridge_deposit(&999_u64).is_none());
}

fn env_token(env: &Env) -> Address {
    let token_admin = Address::generate(env);
    env.register_stellar_asset_contract_v2(token_admin)
        .address()
}
