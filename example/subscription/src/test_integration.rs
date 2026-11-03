/// End-to-end integration tests for cyclical billing (#297).
///
/// These tests simulate real scheduler behaviour: time advances in
/// discrete steps, the billing bot calls `process_billing_cycle` each
/// period, and we assert the full lifecycle — from subscription through
/// multiple paid cycles, grace recovery, and eventual termination.
use super::*;
use crate::types::ChargeOutcome;
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::token::{StellarAssetClient, TokenClient};
use soroban_sdk::{Address, Env, String, Vec};

const MONTHLY: u64 = 2_592_000; // 30 days in seconds
const PLAN_AMOUNT: i128 = 1_000;

// ── Helpers ───────────────────────────────────────────────────────────────────

struct Fixture<'a> {
    env: Env,
    contract: Address,
    client: SubscriptionContractClient<'a>,
    merchant: Address,
    customer: Address,
    token: Address,
    plan_id: u64,
}

fn setup() -> Fixture<'static> {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = 1_000_000);

    let contract = env.register(SubscriptionContract, ());
    let client = SubscriptionContractClient::new(&env, &contract);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    client.add_accepted_token(&token);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    let plan_id = client.create_plan(
        &merchant,
        &String::from_str(&env, "Monthly Plan"),
        &token,
        &PLAN_AMOUNT,
        &MONTHLY,
        &None,
        &0,
    );

    Fixture {
        env,
        contract,
        client,
        merchant,
        customer,
        token,
        plan_id,
    }
}

fn fund(env: &Env, token: &Address, to: &Address, amount: i128) {
    StellarAssetClient::new(env, token).mint(to, &amount);
}

fn approve(env: &Env, token: &Address, owner: &Address, spender: &Address, amount: i128) {
    let expiry = env.ledger().sequence() + 1_000_000;
    TokenClient::new(env, token).approve(owner, spender, &amount, &expiry);
}

fn balance(env: &Env, token: &Address, who: &Address) -> i128 {
    TokenClient::new(env, token).balance(who)
}

fn advance(env: &Env, seconds: u64) {
    env.ledger().with_mut(|l| l.timestamp += seconds);
}

fn batch(env: &Env, sub_ids: &[u64]) -> Vec<u64> {
    let mut v: Vec<u64> = Vec::new(env);
    for id in sub_ids {
        v.push_back(*id);
    }
    v
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Simulate 6 consecutive monthly billing cycles for a single subscriber.
/// The scheduler fires once per interval; the merchant should accumulate
/// 6× the plan amount by the end.
#[test]
fn test_six_monthly_cycles_end_to_end() {
    let f = setup();
    fund(&f.env, &f.token, &f.customer, PLAN_AMOUNT * 6);
    approve(&f.env, &f.token, &f.customer, &f.contract, PLAN_AMOUNT * 6);

    let sub_id = f.client.subscribe(&f.customer, &f.plan_id);
    let ids = batch(&f.env, &[sub_id]);

    for cycle in 1..=6u64 {
        let outcomes = f.client.process_billing_cycle(&ids);
        assert_eq!(
            outcomes.get(0).unwrap(),
            ChargeOutcome::Charged,
            "cycle {cycle}"
        );
        advance(&f.env, MONTHLY);
    }

    assert_eq!(balance(&f.env, &f.token, &f.merchant), PLAN_AMOUNT * 6);
    assert_eq!(balance(&f.env, &f.token, &f.customer), 0);
}

/// Scheduler fires every interval for multiple subscribers simultaneously.
/// Each subscriber pays independently; merchant accumulates all funds.
#[test]
fn test_multi_subscriber_cyclical_billing() {
    let f = setup();
    let customer_a = Address::generate(&f.env);
    let customer_b = Address::generate(&f.env);
    let customer_c = Address::generate(&f.env);
    let all_customers = [&customer_a, &customer_b, &customer_c];

    let mut sub_ids = Vec::new(&f.env);
    for c in all_customers.iter() {
        fund(&f.env, &f.token, c, PLAN_AMOUNT * 3);
        approve(&f.env, &f.token, c, &f.contract, PLAN_AMOUNT * 3);
        sub_ids.push_back(f.client.subscribe(c, &f.plan_id));
    }

    for _ in 0..3 {
        let outcomes = f.client.process_billing_cycle(&sub_ids);
        for outcome in outcomes.iter() {
            assert_eq!(outcome, ChargeOutcome::Charged);
        }
        advance(&f.env, MONTHLY);
    }

    assert_eq!(balance(&f.env, &f.token, &f.merchant), PLAN_AMOUNT * 9);
}

/// A subscriber misses one cycle (no allowance), enters grace, then
/// recovers in the next scheduler pass after topping up.
#[test]
fn test_grace_recovery_within_cyclical_billing() {
    let f = setup();
    // Set a 7-day grace period.
    f.client
        .set_plan_grace_period(&f.merchant, &f.plan_id, &(7 * 86_400));

    let sub_id = f.client.subscribe(&f.customer, &f.plan_id);
    let ids = batch(&f.env, &[sub_id]);

    // Cycle 1: funded → Charged.
    fund(&f.env, &f.token, &f.customer, PLAN_AMOUNT);
    approve(&f.env, &f.token, &f.customer, &f.contract, PLAN_AMOUNT);
    assert_eq!(
        f.client.process_billing_cycle(&ids).get(0).unwrap(),
        ChargeOutcome::Charged
    );
    advance(&f.env, MONTHLY);

    // Cycle 2: no allowance → EnteredGrace.
    assert_eq!(
        f.client.process_billing_cycle(&ids).get(0).unwrap(),
        ChargeOutcome::EnteredGrace
    );

    // Customer tops up within grace window.
    advance(&f.env, 86_400); // 1 day into grace
    fund(&f.env, &f.token, &f.customer, PLAN_AMOUNT);
    approve(&f.env, &f.token, &f.customer, &f.contract, PLAN_AMOUNT);

    // Cycle 2 retry: Recovered.
    assert_eq!(
        f.client.process_billing_cycle(&ids).get(0).unwrap(),
        ChargeOutcome::Recovered
    );
    assert_eq!(
        f.client.get_subscription(&sub_id).status,
        SubscriptionStatus::Active
    );
    assert_eq!(balance(&f.env, &f.token, &f.merchant), PLAN_AMOUNT * 2);
}

/// Grace window expires without recovery → subscription terminates.
/// Subsequent scheduler passes return Skipped.
#[test]
fn test_grace_expiry_terminates_subscription_in_cycle() {
    let f = setup();
    f.client
        .set_plan_grace_period(&f.merchant, &f.plan_id, &86_400);

    let sub_id = f.client.subscribe(&f.customer, &f.plan_id);
    let ids = batch(&f.env, &[sub_id]);

    // No allowance → EnteredGrace.
    assert_eq!(
        f.client.process_billing_cycle(&ids).get(0).unwrap(),
        ChargeOutcome::EnteredGrace
    );

    // Advance past grace window without recovery.
    advance(&f.env, 86_401);

    // Scheduler fires again → Terminated.
    assert_eq!(
        f.client.process_billing_cycle(&ids).get(0).unwrap(),
        ChargeOutcome::Terminated
    );

    // All subsequent passes → Skipped.
    advance(&f.env, MONTHLY);
    assert_eq!(
        f.client.process_billing_cycle(&ids).get(0).unwrap(),
        ChargeOutcome::Skipped
    );
}

/// Mixed batch: some subscribers active, one cancelled, one past-due.
/// Verifies the batch never aborts and returns correct per-entry outcomes.
#[test]
fn test_mixed_batch_cyclical_sweep() {
    let f = setup();
    f.client
        .set_plan_grace_period(&f.merchant, &f.plan_id, &86_400);

    let customer_a = Address::generate(&f.env); // will charge
    let customer_b = Address::generate(&f.env); // will be cancelled
    let customer_c = Address::generate(&f.env); // no allowance → grace

    fund(&f.env, &f.token, &customer_a, PLAN_AMOUNT);
    approve(&f.env, &f.token, &customer_a, &f.contract, PLAN_AMOUNT);

    let sub_a = f.client.subscribe(&customer_a, &f.plan_id);
    let sub_b = f.client.subscribe(&customer_b, &f.plan_id);
    let sub_c = f.client.subscribe(&customer_c, &f.plan_id);

    f.client.cancel_subscription(&customer_b, &sub_b);

    let ids = batch(&f.env, &[sub_a, sub_b, sub_c]);
    let outcomes = f.client.process_billing_cycle(&ids);

    assert_eq!(outcomes.get(0).unwrap(), ChargeOutcome::Charged);
    assert_eq!(outcomes.get(1).unwrap(), ChargeOutcome::Skipped);
    assert_eq!(outcomes.get(2).unwrap(), ChargeOutcome::EnteredGrace);
}

/// Verify that `subscribe_with_token` (multi-token, #295) integrates
/// correctly with the cyclical billing loop.
#[test]
fn test_multi_token_subscriber_in_cyclical_billing() {
    let f = setup();

    // Register a second accepted token.
    let token2_admin = Address::generate(&f.env);
    let token2 = f
        .env
        .register_stellar_asset_contract_v2(token2_admin)
        .address();
    f.client.add_accepted_token(&token2);

    // Customer subscribes using token2 as preferred payment token.
    fund(&f.env, &token2, &f.customer, PLAN_AMOUNT * 3);
    approve(&f.env, &token2, &f.customer, &f.contract, PLAN_AMOUNT * 3);

    let sub_id = f
        .client
        .subscribe_with_token(&f.customer, &f.plan_id, &token2);
    let ids = batch(&f.env, &[sub_id]);

    for cycle in 1..=3u64 {
        let outcomes = f.client.process_billing_cycle(&ids);
        assert_eq!(
            outcomes.get(0).unwrap(),
            ChargeOutcome::Charged,
            "cycle {cycle}"
        );
        advance(&f.env, MONTHLY);
    }

    // Merchant received token2, not the plan's default token.
    assert_eq!(balance(&f.env, &token2, &f.merchant), PLAN_AMOUNT * 3);
    assert_eq!(balance(&f.env, &f.token, &f.merchant), 0);
}
