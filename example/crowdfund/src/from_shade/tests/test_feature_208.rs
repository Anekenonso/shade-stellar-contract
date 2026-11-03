#![cfg(test)]

//! Comprehensive test suite for the Creator Vesting feature (#208).
//!
//! Covers:
//!   1. Happy-path flows for all six vesting functions.
//!   2. Malicious-actor / unauthorized-access attempts.
//!   3. Event emission with exact argument verification.
//!   4. Storage rollback when functions panic.
//!   5. Boundary values and uninitialized-state edge cases.
//!   6. State transitions (e.g., cannot release twice, cannot release early).

use crate::from_shade::errors::ContractError;
use shade::shade::{Shade, ShadeClient};
use crate::from_shade::types::{CrowdfundVestingConfig, VestingSchedule, VestingTimeline};
use soroban_sdk::testutils::{Address as _, Events as _, Ledger as _};
use soroban_sdk::{Address, Env, Map, String, Symbol, TryIntoVal, Val};

// ── Constants ─────────────────────────────────────────────────────────────────

const CLIFF: u64 = 30 * 24 * 3600; // 30 days in seconds
const DURATION: u64 = 365 * 24 * 3600; // 1 year in seconds
const UNLOCK_BPS: i128 = 2500; // 25 %
const TOTAL_AMOUNT: i128 = 1_000_000;

// ── Test fixture ──────────────────────────────────────────────────────────────

struct Fixture<'a> {
    env: Env,
    client: ShadeClient<'a>,
    contract_id: Address,
    admin: Address,
}

fn setup() -> Fixture<'static> {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    Fixture {
        env,
        client,
        contract_id,
        admin,
    }
}

// ── Helper: create a timeline and return its ID ───────────────────────────────

fn create_default_timeline(f: &Fixture) -> u64 {
    f.client.create_vesting_timeline(
        &f.admin,
        &String::from_str(&f.env, "Founder Vest"),
        &CLIFF,
        &DURATION,
        &UNLOCK_BPS,
    )
}

// ── Helper: add a schedule tranche to an existing timeline ───────────────────

fn add_schedule(f: &Fixture, timeline_id: u64, tranche: u64, amount: i128, unlock_at: u64) {
    f.client.add_vesting_schedule(
        &f.admin,
        &timeline_id,
        &tranche,
        &amount,
        &unlock_at,
    );
}

// ── Event assertion helpers ───────────────────────────────────────────────────

/// Scan events from newest to oldest, return the first (most-recent) whose
/// topic symbol matches `event_name`.
fn last_event_map(env: &Env, contract_id: &Address, event_name: &str) -> Map<Symbol, Val> {
    let events = env.events().all();
    for i in (0..events.len()).rev() {
        let (cid, topics, data) = events.get(i).unwrap();
        if cid != contract_id.clone() || topics.len() != 1 {
            continue;
        }
        let name: Symbol = topics.get(0).unwrap().try_into_val(env).unwrap();
        if name == Symbol::new(env, event_name) {
            let m: Map<Symbol, Val> = data.try_into_val(env).unwrap();
            return m;
        }
    }
    panic!("event '{}' not found", event_name);
}

fn count_events(env: &Env, contract_id: &Address, event_name: &str) -> u32 {
    let events = env.events().all();
    let mut n = 0u32;
    for i in 0..events.len() {
        let (cid, topics, _) = events.get(i).unwrap();
        if cid != contract_id.clone() || topics.len() != 1 {
            continue;
        }
        let name: Symbol = topics.get(0).unwrap().try_into_val(env).unwrap();
        if name == Symbol::new(env, event_name) {
            n += 1;
        }
    }
    n
}

fn assert_vesting_timeline_created_event(
    env: &Env,
    contract_id: &Address,
    expected_id: u64,
    expected_cliff: u64,
    expected_duration: u64,
    expected_admin: &Address,
) {
    let m = last_event_map(env, contract_id, "vesting_timeline_created_event");
    let id: u64 = m.get(Symbol::new(env, "timeline_id")).unwrap().try_into_val(env).unwrap();
    let cliff: u64 = m.get(Symbol::new(env, "cliff_duration")).unwrap().try_into_val(env).unwrap();
    let dur: u64 = m.get(Symbol::new(env, "vesting_duration")).unwrap().try_into_val(env).unwrap();
    let admin: Address = m.get(Symbol::new(env, "admin")).unwrap().try_into_val(env).unwrap();
    assert_eq!(id, expected_id);
    assert_eq!(cliff, expected_cliff);
    assert_eq!(dur, expected_duration);
    assert_eq!(admin, expected_admin.clone());
}

fn assert_vesting_timeline_updated_event(
    env: &Env,
    contract_id: &Address,
    expected_id: u64,
    expected_cliff: u64,
    expected_duration: u64,
) {
    let m = last_event_map(env, contract_id, "vesting_timeline_updated_event");
    let id: u64 = m.get(Symbol::new(env, "timeline_id")).unwrap().try_into_val(env).unwrap();
    let cliff: u64 = m.get(Symbol::new(env, "cliff_duration")).unwrap().try_into_val(env).unwrap();
    let dur: u64 = m.get(Symbol::new(env, "vesting_duration")).unwrap().try_into_val(env).unwrap();
    assert_eq!(id, expected_id);
    assert_eq!(cliff, expected_cliff);
    assert_eq!(dur, expected_duration);
}

fn assert_crowdfund_vesting_configured_event(
    env: &Env,
    contract_id: &Address,
    expected_crowdfund_id: u64,
    expected_timeline_id: u64,
    expected_amount: i128,
) {
    let m = last_event_map(env, contract_id, "crowdfund_vesting_configured_event");
    let cid: u64 = m.get(Symbol::new(env, "crowdfund_id")).unwrap().try_into_val(env).unwrap();
    let tid: u64 = m.get(Symbol::new(env, "timeline_id")).unwrap().try_into_val(env).unwrap();
    let amt: i128 = m.get(Symbol::new(env, "total_vesting_amount")).unwrap().try_into_val(env).unwrap();
    assert_eq!(cid, expected_crowdfund_id);
    assert_eq!(tid, expected_timeline_id);
    assert_eq!(amt, expected_amount);
}

fn assert_vesting_schedule_released_event(
    env: &Env,
    contract_id: &Address,
    expected_timeline_id: u64,
    expected_tranche: u64,
    expected_amount: i128,
) {
    let m = last_event_map(env, contract_id, "vesting_schedule_released_event");
    let tid: u64 = m.get(Symbol::new(env, "timeline_id")).unwrap().try_into_val(env).unwrap();
    let tr: u64 = m.get(Symbol::new(env, "tranche_index")).unwrap().try_into_val(env).unwrap();
    let amt: i128 = m.get(Symbol::new(env, "unlock_amount")).unwrap().try_into_val(env).unwrap();
    assert_eq!(tid, expected_timeline_id);
    assert_eq!(tr, expected_tranche);
    assert_eq!(amt, expected_amount);
}

// ══════════════════════════════════════════════════════════════════════════════
// 1. create_vesting_timeline – happy path
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn create_timeline_returns_sequential_ids() {
    let f = setup();
    let id1 = create_default_timeline(&f);
    let id2 = create_default_timeline(&f);
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
}

#[test]
fn create_timeline_stores_correct_fields() {
    let f = setup();
    let id = create_default_timeline(&f);
    let tl: VestingTimeline = f.client.get_vesting_timeline(&id);
    assert_eq!(tl.id, id);
    assert_eq!(tl.cliff_duration, CLIFF);
    assert_eq!(tl.vesting_duration, DURATION);
    assert_eq!(tl.unlock_percentage, UNLOCK_BPS);
    assert_eq!(tl.admin, f.admin);
    assert_eq!(tl.name, String::from_str(&f.env, "Founder Vest"));
}

#[test]
fn create_timeline_emits_event_with_exact_args() {
    let f = setup();
    let id = create_default_timeline(&f);
    assert_vesting_timeline_created_event(&f.env, &f.contract_id, id, CLIFF, DURATION, &f.admin);
}

#[test]
fn create_timeline_event_count_increments_per_call() {
    let f = setup();
    create_default_timeline(&f);
    create_default_timeline(&f);
    assert_eq!(count_events(&f.env, &f.contract_id, "vesting_timeline_created_event"), 2);
}

#[test]
fn create_timeline_boundary_unlock_percentage_1_bps() {
    let f = setup();
    let id = f.client.create_vesting_timeline(
        &f.admin,
        &String::from_str(&f.env, "Min Unlock"),
        &CLIFF,
        &DURATION,
        &1_i128,
    );
    let tl: VestingTimeline = f.client.get_vesting_timeline(&id);
    assert_eq!(tl.unlock_percentage, 1);
}

#[test]
fn create_timeline_boundary_unlock_percentage_10000_bps() {
    let f = setup();
    let id = f.client.create_vesting_timeline(
        &f.admin,
        &String::from_str(&f.env, "Full Unlock"),
        &CLIFF,
        &DURATION,
        &10000_i128,
    );
    let tl: VestingTimeline = f.client.get_vesting_timeline(&id);
    assert_eq!(tl.unlock_percentage, 10000);
}

// ══════════════════════════════════════════════════════════════════════════════
// 2. create_vesting_timeline – error paths
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn create_timeline_zero_cliff_rejected() {
    let f = setup();
    let err = soroban_sdk::Error::from_contract_error(ContractError::InvalidInterval as u32);
    let res = f.client.try_create_vesting_timeline(
        &f.admin,
        &String::from_str(&f.env, "Bad Cliff"),
        &0u64,
        &DURATION,
        &UNLOCK_BPS,
    );
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

#[test]
fn create_timeline_zero_duration_rejected() {
    let f = setup();
    let err = soroban_sdk::Error::from_contract_error(ContractError::InvalidInterval as u32);
    let res = f.client.try_create_vesting_timeline(
        &f.admin,
        &String::from_str(&f.env, "Bad Duration"),
        &CLIFF,
        &0u64,
        &UNLOCK_BPS,
    );
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

#[test]
fn create_timeline_zero_unlock_pct_rejected() {
    let f = setup();
    let err = soroban_sdk::Error::from_contract_error(shade::errors::ContractError::InvalidAmount as u32);
    let res = f.client.try_create_vesting_timeline(
        &f.admin,
        &String::from_str(&f.env, "Zero Pct"),
        &CLIFF,
        &DURATION,
        &0_i128,
    );
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

#[test]
fn create_timeline_negative_unlock_pct_rejected() {
    let f = setup();
    let err = soroban_sdk::Error::from_contract_error(shade::errors::ContractError::InvalidAmount as u32);
    let res = f.client.try_create_vesting_timeline(
        &f.admin,
        &String::from_str(&f.env, "Neg Pct"),
        &CLIFF,
        &DURATION,
        &-1_i128,
    );
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

#[test]
fn create_timeline_unlock_pct_over_10000_rejected() {
    let f = setup();
    let err = soroban_sdk::Error::from_contract_error(shade::errors::ContractError::InvalidAmount as u32);
    let res = f.client.try_create_vesting_timeline(
        &f.admin,
        &String::from_str(&f.env, "Over 100%"),
        &CLIFF,
        &DURATION,
        &10001_i128,
    );
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

#[test]
fn create_timeline_non_admin_rejected() {
    let f = setup();
    let attacker = Address::generate(&f.env);
    // Disable mock auths so require_auth actually enforces
    f.env.set_auths(&[]);
    let res = f.client.try_create_vesting_timeline(
        &attacker,
        &String::from_str(&f.env, "Hack"),
        &CLIFF,
        &DURATION,
        &UNLOCK_BPS,
    );
    assert!(res.is_err());
}

#[test]
fn create_timeline_error_does_not_increment_counter() {
    let f = setup();
    let _ = f.client.try_create_vesting_timeline(
        &f.admin,
        &String::from_str(&f.env, "Bad"),
        &0u64,
        &DURATION,
        &UNLOCK_BPS,
    );
    // After the failure, a successful call must still return id=1
    let id = create_default_timeline(&f);
    assert_eq!(id, 1);
}

// ══════════════════════════════════════════════════════════════════════════════
// 3. get_vesting_timeline – edge cases
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn get_nonexistent_timeline_panics() {
    let f = setup();
    let err = soroban_sdk::Error::from_contract_error(ContractError::InvalidInterval as u32);
    let res = f.client.try_get_vesting_timeline(&999u64);
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

// ══════════════════════════════════════════════════════════════════════════════
// 4. update_vesting_timeline – happy path
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn update_timeline_changes_stored_fields() {
    let f = setup();
    let id = create_default_timeline(&f);
    let new_cliff = CLIFF * 2;
    let new_dur = DURATION * 2;
    f.client.update_vesting_timeline(&f.admin, &id, &new_cliff, &new_dur);
    let tl: VestingTimeline = f.client.get_vesting_timeline(&id);
    assert_eq!(tl.cliff_duration, new_cliff);
    assert_eq!(tl.vesting_duration, new_dur);
}

#[test]
fn update_timeline_does_not_change_unlock_percentage() {
    let f = setup();
    let id = create_default_timeline(&f);
    f.client.update_vesting_timeline(&f.admin, &id, &CLIFF, &DURATION);
    let tl: VestingTimeline = f.client.get_vesting_timeline(&id);
    assert_eq!(tl.unlock_percentage, UNLOCK_BPS);
}

#[test]
fn update_timeline_emits_event_with_exact_args() {
    let f = setup();
    let id = create_default_timeline(&f);
    let new_cliff = 7 * 24 * 3600u64;
    let new_dur = 180 * 24 * 3600u64;
    f.client.update_vesting_timeline(&f.admin, &id, &new_cliff, &new_dur);
    assert_vesting_timeline_updated_event(&f.env, &f.contract_id, id, new_cliff, new_dur);
}

#[test]
fn update_timeline_can_be_called_multiple_times() {
    let f = setup();
    let id = create_default_timeline(&f);
    f.client.update_vesting_timeline(&f.admin, &id, &1000u64, &2000u64);
    f.client.update_vesting_timeline(&f.admin, &id, &3000u64, &4000u64);
    let tl: VestingTimeline = f.client.get_vesting_timeline(&id);
    assert_eq!(tl.cliff_duration, 3000);
    assert_eq!(tl.vesting_duration, 4000);
}

// ── update_vesting_timeline – error paths ─────────────────────────────────────

#[test]
fn update_timeline_zero_cliff_rejected() {
    let f = setup();
    let id = create_default_timeline(&f);
    let err = soroban_sdk::Error::from_contract_error(ContractError::InvalidInterval as u32);
    let res = f.client.try_update_vesting_timeline(&f.admin, &id, &0u64, &DURATION);
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

#[test]
fn update_timeline_zero_duration_rejected() {
    let f = setup();
    let id = create_default_timeline(&f);
    let err = soroban_sdk::Error::from_contract_error(ContractError::InvalidInterval as u32);
    let res = f.client.try_update_vesting_timeline(&f.admin, &id, &CLIFF, &0u64);
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

#[test]
fn update_nonexistent_timeline_panics() {
    let f = setup();
    let err = soroban_sdk::Error::from_contract_error(ContractError::InvalidInterval as u32);
    let res = f.client.try_update_vesting_timeline(&f.admin, &999u64, &CLIFF, &DURATION);
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

#[test]
fn update_timeline_non_admin_rejected() {
    let f = setup();
    let id = create_default_timeline(&f);
    let attacker = Address::generate(&f.env);
    f.env.set_auths(&[]);
    let res = f.client.try_update_vesting_timeline(&attacker, &id, &CLIFF, &DURATION);
    assert!(res.is_err());
}

#[test]
fn update_timeline_non_admin_wrong_address_rejected() {
    let f = setup();
    let id = create_default_timeline(&f);
    let impostor = Address::generate(&f.env);
    // re-enable mock auths but the impostor address is not the stored admin
    let err = soroban_sdk::Error::from_contract_error(shade::errors::ContractError::NotAuthorized as u32);
    let res = f.client.try_update_vesting_timeline(&impostor, &id, &CLIFF, &DURATION);
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

#[test]
fn update_timeline_failure_preserves_original_values() {
    let f = setup();
    let id = create_default_timeline(&f);
    let _ = f.client.try_update_vesting_timeline(&f.admin, &id, &0u64, &DURATION);
    let tl: VestingTimeline = f.client.get_vesting_timeline(&id);
    assert_eq!(tl.cliff_duration, CLIFF);
    assert_eq!(tl.vesting_duration, DURATION);
}

#[test]
fn update_timeline_blocked_when_paused() {
    let f = setup();
    let id = create_default_timeline(&f);
    f.client.pause(&f.admin);
    let err = soroban_sdk::Error::from_contract_error(shade::errors::ContractError::ContractPaused as u32);
    let res = f.client.try_update_vesting_timeline(&f.admin, &id, &CLIFF, &DURATION);
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

// ══════════════════════════════════════════════════════════════════════════════
// 5. configure_crowdfund_vesting – happy path
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn configure_crowdfund_vesting_stores_config() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    f.client.configure_crowdfund_vesting(&f.admin, &1u64, &tl_id, &TOTAL_AMOUNT);
    let cfg: CrowdfundVestingConfig = f.client.get_crowdfund_vesting_config(&1u64);
    assert_eq!(cfg.crowdfund_id, 1);
    assert_eq!(cfg.timeline_id, tl_id);
    assert_eq!(cfg.total_vesting_amount, TOTAL_AMOUNT);
}

#[test]
fn configure_crowdfund_vesting_emits_event() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    f.client.configure_crowdfund_vesting(&f.admin, &42u64, &tl_id, &TOTAL_AMOUNT);
    assert_crowdfund_vesting_configured_event(&f.env, &f.contract_id, 42, tl_id, TOTAL_AMOUNT);
}

#[test]
fn configure_crowdfund_vesting_can_reconfigure() {
    let f = setup();
    let tl_id1 = create_default_timeline(&f);
    let tl_id2 = create_default_timeline(&f);
    f.client.configure_crowdfund_vesting(&f.admin, &1u64, &tl_id1, &500_000_i128);
    f.client.configure_crowdfund_vesting(&f.admin, &1u64, &tl_id2, &750_000_i128);
    let cfg: CrowdfundVestingConfig = f.client.get_crowdfund_vesting_config(&1u64);
    assert_eq!(cfg.timeline_id, tl_id2);
    assert_eq!(cfg.total_vesting_amount, 750_000);
}

#[test]
fn configure_crowdfund_vesting_different_crowdfunds_independent() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    f.client.configure_crowdfund_vesting(&f.admin, &10u64, &tl_id, &100_000_i128);
    f.client.configure_crowdfund_vesting(&f.admin, &20u64, &tl_id, &200_000_i128);
    let cfg10: CrowdfundVestingConfig = f.client.get_crowdfund_vesting_config(&10u64);
    let cfg20: CrowdfundVestingConfig = f.client.get_crowdfund_vesting_config(&20u64);
    assert_eq!(cfg10.total_vesting_amount, 100_000);
    assert_eq!(cfg20.total_vesting_amount, 200_000);
}

// ── configure_crowdfund_vesting – error paths ─────────────────────────────────

#[test]
fn configure_crowdfund_vesting_zero_amount_rejected() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    let err = soroban_sdk::Error::from_contract_error(shade::errors::ContractError::InvalidAmount as u32);
    let res = f.client.try_configure_crowdfund_vesting(&f.admin, &1u64, &tl_id, &0_i128);
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

#[test]
fn configure_crowdfund_vesting_negative_amount_rejected() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    let err = soroban_sdk::Error::from_contract_error(shade::errors::ContractError::InvalidAmount as u32);
    let res = f.client.try_configure_crowdfund_vesting(&f.admin, &1u64, &tl_id, &-1_i128);
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

#[test]
fn configure_crowdfund_vesting_nonexistent_timeline_panics() {
    let f = setup();
    let err = soroban_sdk::Error::from_contract_error(ContractError::InvalidInterval as u32);
    let res = f.client.try_configure_crowdfund_vesting(&f.admin, &1u64, &999u64, &TOTAL_AMOUNT);
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

#[test]
fn configure_crowdfund_vesting_non_admin_rejected() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    let impostor = Address::generate(&f.env);
    let err = soroban_sdk::Error::from_contract_error(shade::errors::ContractError::NotAuthorized as u32);
    let res = f.client.try_configure_crowdfund_vesting(&impostor, &1u64, &tl_id, &TOTAL_AMOUNT);
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

#[test]
fn configure_crowdfund_vesting_failure_leaves_no_config() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    let _ = f.client.try_configure_crowdfund_vesting(&f.admin, &7u64, &tl_id, &0_i128);
    let err = soroban_sdk::Error::from_contract_error(shade::errors::ContractError::InvoiceNotFound as u32);
    let res = f.client.try_get_crowdfund_vesting_config(&7u64);
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

#[test]
fn configure_crowdfund_vesting_blocked_when_paused() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    f.client.pause(&f.admin);
    let err = soroban_sdk::Error::from_contract_error(shade::errors::ContractError::ContractPaused as u32);
    let res = f.client.try_configure_crowdfund_vesting(&f.admin, &1u64, &tl_id, &TOTAL_AMOUNT);
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

#[test]
fn get_crowdfund_vesting_config_nonexistent_panics() {
    let f = setup();
    let err = soroban_sdk::Error::from_contract_error(shade::errors::ContractError::InvoiceNotFound as u32);
    let res = f.client.try_get_crowdfund_vesting_config(&404u64);
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

// ══════════════════════════════════════════════════════════════════════════════
// 6. add_vesting_schedule – happy path
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn add_schedule_stores_correct_fields() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    let unlock_at = f.env.ledger().timestamp() + CLIFF;
    add_schedule(&f, tl_id, 0, 250_000_i128, unlock_at);
    let s: VestingSchedule = f.client.get_vesting_schedule(&tl_id, &0u64);
    assert_eq!(s.timeline_id, tl_id);
    assert_eq!(s.tranche_index, 0);
    assert_eq!(s.unlock_amount, 250_000);
    assert_eq!(s.unlock_timestamp, unlock_at);
    assert!(!s.released);
}

#[test]
fn add_multiple_tranches_stored_independently() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    let t0 = f.env.ledger().timestamp() + 1000;
    let t1 = f.env.ledger().timestamp() + 2000;
    add_schedule(&f, tl_id, 0, 100_000_i128, t0);
    add_schedule(&f, tl_id, 1, 200_000_i128, t1);
    let s0: VestingSchedule = f.client.get_vesting_schedule(&tl_id, &0u64);
    let s1: VestingSchedule = f.client.get_vesting_schedule(&tl_id, &1u64);
    assert_eq!(s0.unlock_amount, 100_000);
    assert_eq!(s1.unlock_amount, 200_000);
    assert_eq!(s0.unlock_timestamp, t0);
    assert_eq!(s1.unlock_timestamp, t1);
}

#[test]
fn add_schedule_overwrites_existing_tranche() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    let t = f.env.ledger().timestamp() + 500;
    add_schedule(&f, tl_id, 0, 100_000_i128, t);
    add_schedule(&f, tl_id, 0, 999_999_i128, t + 1000);
    let s: VestingSchedule = f.client.get_vesting_schedule(&tl_id, &0u64);
    assert_eq!(s.unlock_amount, 999_999);
}

#[test]
fn add_schedule_with_zero_timestamp_allowed() {
    // unlock_timestamp=0 means "immediately releasable" — not validated
    let f = setup();
    let tl_id = create_default_timeline(&f);
    add_schedule(&f, tl_id, 0, 50_000_i128, 0u64);
    let s: VestingSchedule = f.client.get_vesting_schedule(&tl_id, &0u64);
    assert_eq!(s.unlock_timestamp, 0);
}

// ── add_vesting_schedule – error paths ───────────────────────────────────────

#[test]
fn add_schedule_zero_amount_rejected() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    let err = soroban_sdk::Error::from_contract_error(shade::errors::ContractError::InvalidAmount as u32);
    let res = f.client.try_add_vesting_schedule(
        &f.admin, &tl_id, &0u64, &0_i128, &1000u64,
    );
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

#[test]
fn add_schedule_negative_amount_rejected() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    let err = soroban_sdk::Error::from_contract_error(shade::errors::ContractError::InvalidAmount as u32);
    let res = f.client.try_add_vesting_schedule(
        &f.admin, &tl_id, &0u64, &-100_i128, &1000u64,
    );
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

#[test]
fn add_schedule_nonexistent_timeline_panics() {
    let f = setup();
    let err = soroban_sdk::Error::from_contract_error(ContractError::InvalidInterval as u32);
    let res = f.client.try_add_vesting_schedule(
        &f.admin, &999u64, &0u64, &100_i128, &1000u64,
    );
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

#[test]
fn add_schedule_non_admin_rejected() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    let impostor = Address::generate(&f.env);
    let err = soroban_sdk::Error::from_contract_error(shade::errors::ContractError::NotAuthorized as u32);
    let res = f.client.try_add_vesting_schedule(
        &impostor, &tl_id, &0u64, &100_i128, &1000u64,
    );
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

#[test]
fn add_schedule_blocked_when_paused() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    f.client.pause(&f.admin);
    let err = soroban_sdk::Error::from_contract_error(shade::errors::ContractError::ContractPaused as u32);
    let res = f.client.try_add_vesting_schedule(
        &f.admin, &tl_id, &0u64, &100_i128, &1000u64,
    );
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

#[test]
fn add_schedule_failure_does_not_store_schedule() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    // bad amount — should not write
    let _ = f.client.try_add_vesting_schedule(
        &f.admin, &tl_id, &5u64, &0_i128, &1000u64,
    );
    // Reading that tranche should panic with InvoiceNotFound
    let err = soroban_sdk::Error::from_contract_error(shade::errors::ContractError::InvoiceNotFound as u32);
    let res = f.client.try_get_vesting_schedule(&tl_id, &5u64);
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

// ══════════════════════════════════════════════════════════════════════════════
// 7. release_vesting_schedule – happy path
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn release_schedule_marks_released_true() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    let now = f.env.ledger().timestamp();
    add_schedule(&f, tl_id, 0, 250_000_i128, now);
    f.client.release_vesting_schedule(&f.admin, &tl_id, &0u64);
    let s: VestingSchedule = f.client.get_vesting_schedule(&tl_id, &0u64);
    assert!(s.released);
}

#[test]
fn release_schedule_emits_event_with_exact_args() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    let now = f.env.ledger().timestamp();
    add_schedule(&f, tl_id, 2, 333_000_i128, now);
    f.client.release_vesting_schedule(&f.admin, &tl_id, &2u64);
    assert_vesting_schedule_released_event(&f.env, &f.contract_id, tl_id, 2, 333_000);
}

#[test]
fn release_schedule_works_at_exact_unlock_timestamp() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    let unlock_at = 9999u64;
    f.env.ledger().with_mut(|l| l.timestamp = unlock_at);
    add_schedule(&f, tl_id, 0, 100_i128, unlock_at);
    f.client.release_vesting_schedule(&f.admin, &tl_id, &0u64);
    let s: VestingSchedule = f.client.get_vesting_schedule(&tl_id, &0u64);
    assert!(s.released);
}

#[test]
fn release_schedule_works_after_unlock_timestamp() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    let unlock_at = 1000u64;
    add_schedule(&f, tl_id, 0, 100_i128, unlock_at);
    // advance time well past unlock
    f.env.ledger().with_mut(|l| l.timestamp = 999_999);
    f.client.release_vesting_schedule(&f.admin, &tl_id, &0u64);
    let s: VestingSchedule = f.client.get_vesting_schedule(&tl_id, &0u64);
    assert!(s.released);
}

#[test]
fn release_different_tranches_independently() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    let now = f.env.ledger().timestamp();
    add_schedule(&f, tl_id, 0, 100_i128, now);
    add_schedule(&f, tl_id, 1, 200_i128, now);
    f.client.release_vesting_schedule(&f.admin, &tl_id, &0u64);
    let s0: VestingSchedule = f.client.get_vesting_schedule(&tl_id, &0u64);
    let s1: VestingSchedule = f.client.get_vesting_schedule(&tl_id, &1u64);
    assert!(s0.released);
    assert!(!s1.released);
}

// ── release_vesting_schedule – error paths ────────────────────────────────────

#[test]
fn release_schedule_before_unlock_time_rejected() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    let future = f.env.ledger().timestamp() + 100_000;
    add_schedule(&f, tl_id, 0, 100_i128, future);
    let err = soroban_sdk::Error::from_contract_error(shade::errors::ContractError::InvoiceExpired as u32);
    let res = f.client.try_release_vesting_schedule(&f.admin, &tl_id, &0u64);
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

#[test]
fn release_schedule_twice_rejected() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    let now = f.env.ledger().timestamp();
    add_schedule(&f, tl_id, 0, 100_i128, now);
    f.client.release_vesting_schedule(&f.admin, &tl_id, &0u64);
    let err = soroban_sdk::Error::from_contract_error(shade::errors::ContractError::InvalidInvoiceStatus as u32);
    let res = f.client.try_release_vesting_schedule(&f.admin, &tl_id, &0u64);
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

#[test]
fn release_nonexistent_schedule_panics() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    let err = soroban_sdk::Error::from_contract_error(shade::errors::ContractError::InvoiceNotFound as u32);
    let res = f.client.try_release_vesting_schedule(&f.admin, &tl_id, &99u64);
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

#[test]
fn release_schedule_non_admin_rejected() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    let now = f.env.ledger().timestamp();
    add_schedule(&f, tl_id, 0, 100_i128, now);
    let impostor = Address::generate(&f.env);
    let err = soroban_sdk::Error::from_contract_error(shade::errors::ContractError::NotAuthorized as u32);
    let res = f.client.try_release_vesting_schedule(&impostor, &tl_id, &0u64);
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

#[test]
fn release_schedule_blocked_when_paused() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    let now = f.env.ledger().timestamp();
    add_schedule(&f, tl_id, 0, 100_i128, now);
    f.client.pause(&f.admin);
    let err = soroban_sdk::Error::from_contract_error(shade::errors::ContractError::ContractPaused as u32);
    let res = f.client.try_release_vesting_schedule(&f.admin, &tl_id, &0u64);
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

#[test]
fn release_schedule_early_failure_does_not_set_released() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    let future = f.env.ledger().timestamp() + 9999;
    add_schedule(&f, tl_id, 0, 500_i128, future);
    let _ = f.client.try_release_vesting_schedule(&f.admin, &tl_id, &0u64);
    let s: VestingSchedule = f.client.get_vesting_schedule(&tl_id, &0u64);
    assert!(!s.released);
}

// ══════════════════════════════════════════════════════════════════════════════
// 8. Pause / unpause interaction
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn create_timeline_blocked_when_paused() {
    let f = setup();
    f.client.pause(&f.admin);
    let err = soroban_sdk::Error::from_contract_error(shade::errors::ContractError::ContractPaused as u32);
    let res = f.client.try_create_vesting_timeline(
        &f.admin,
        &String::from_str(&f.env, "Paused"),
        &CLIFF,
        &DURATION,
        &UNLOCK_BPS,
    );
    assert!(matches!(res, Err(Ok(e)) if e == err));
}

#[test]
fn all_write_ops_succeed_after_unpause() {
    let f = setup();
    f.client.pause(&f.admin);
    f.client.unpause(&f.admin);

    // create
    let tl_id = create_default_timeline(&f);
    // update
    f.client.update_vesting_timeline(&f.admin, &tl_id, &CLIFF, &DURATION);
    // configure
    f.client.configure_crowdfund_vesting(&f.admin, &1u64, &tl_id, &TOTAL_AMOUNT);
    // add schedule
    let now = f.env.ledger().timestamp();
    add_schedule(&f, tl_id, 0, 100_i128, now);
    // release
    f.client.release_vesting_schedule(&f.admin, &tl_id, &0u64);

    let s: VestingSchedule = f.client.get_vesting_schedule(&tl_id, &0u64);
    assert!(s.released);
}

// ══════════════════════════════════════════════════════════════════════════════
// 9. Boundary / overflow conditions
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn create_timeline_with_max_u64_durations() {
    let f = setup();
    let id = f.client.create_vesting_timeline(
        &f.admin,
        &String::from_str(&f.env, "Max Durations"),
        &u64::MAX,
        &u64::MAX,
        &UNLOCK_BPS,
    );
    let tl: VestingTimeline = f.client.get_vesting_timeline(&id);
    assert_eq!(tl.cliff_duration, u64::MAX);
    assert_eq!(tl.vesting_duration, u64::MAX);
}

#[test]
fn add_schedule_with_max_i128_amount() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    add_schedule(&f, tl_id, 0, i128::MAX, 0u64);
    let s: VestingSchedule = f.client.get_vesting_schedule(&tl_id, &0u64);
    assert_eq!(s.unlock_amount, i128::MAX);
}

#[test]
fn configure_crowdfund_vesting_with_max_i128_amount() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    f.client.configure_crowdfund_vesting(&f.admin, &1u64, &tl_id, &i128::MAX);
    let cfg: CrowdfundVestingConfig = f.client.get_crowdfund_vesting_config(&1u64);
    assert_eq!(cfg.total_vesting_amount, i128::MAX);
}

#[test]
fn many_timelines_all_retrievable() {
    let f = setup();
    for i in 1u64..=10 {
        let id = create_default_timeline(&f);
        assert_eq!(id, i);
    }
    for i in 1u64..=10 {
        let tl: VestingTimeline = f.client.get_vesting_timeline(&i);
        assert_eq!(tl.id, i);
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// 10. Full end-to-end flow
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn full_vesting_lifecycle_happy_path() {
    let f = setup();

    // Step 1: create a timeline
    let tl_id = f.client.create_vesting_timeline(
        &f.admin,
        &String::from_str(&f.env, "Team Vest 12M"),
        &CLIFF,
        &DURATION,
        &UNLOCK_BPS,
    );
    assert_eq!(tl_id, 1);

    // Step 2: link a crowdfund
    f.client.configure_crowdfund_vesting(&f.admin, &100u64, &tl_id, &TOTAL_AMOUNT);
    let cfg: CrowdfundVestingConfig = f.client.get_crowdfund_vesting_config(&100u64);
    assert_eq!(cfg.timeline_id, tl_id);

    // Step 3: add two tranches
    let now = f.env.ledger().timestamp();
    add_schedule(&f, tl_id, 0, 250_000_i128, now + CLIFF);
    add_schedule(&f, tl_id, 1, 750_000_i128, now + DURATION);

    // Step 4: advance past cliff, release tranche 0
    f.env.ledger().with_mut(|l| l.timestamp = now + CLIFF);
    f.client.release_vesting_schedule(&f.admin, &tl_id, &0u64);
    assert_vesting_schedule_released_event(&f.env, &f.contract_id, tl_id, 0, 250_000);

    // Tranche 1 still locked
    let err = soroban_sdk::Error::from_contract_error(shade::errors::ContractError::InvoiceExpired as u32);
    let res = f.client.try_release_vesting_schedule(&f.admin, &tl_id, &1u64);
    assert!(matches!(res, Err(Ok(e)) if e == err));

    // Step 5: advance past full duration, release tranche 1
    f.env.ledger().with_mut(|l| l.timestamp = now + DURATION);
    f.client.release_vesting_schedule(&f.admin, &tl_id, &1u64);
    assert_vesting_schedule_released_event(&f.env, &f.contract_id, tl_id, 1, 750_000);

    // Both tranches now released
    let s0: VestingSchedule = f.client.get_vesting_schedule(&tl_id, &0u64);
    let s1: VestingSchedule = f.client.get_vesting_schedule(&tl_id, &1u64);
    assert!(s0.released);
    assert!(s1.released);
}

#[test]
fn update_timeline_then_new_schedules_use_updated_values() {
    let f = setup();
    let tl_id = create_default_timeline(&f);

    // update to shorter durations
    let short_cliff = 10u64;
    f.client.update_vesting_timeline(&f.admin, &tl_id, &short_cliff, &100u64);
    let tl: VestingTimeline = f.client.get_vesting_timeline(&tl_id);
    assert_eq!(tl.cliff_duration, short_cliff);

    // add and release at the new (shorter) cliff
    let now = f.env.ledger().timestamp();
    add_schedule(&f, tl_id, 0, 500_i128, now + short_cliff);
    f.env.ledger().with_mut(|l| l.timestamp = now + short_cliff);
    f.client.release_vesting_schedule(&f.admin, &tl_id, &0u64);
    let s: VestingSchedule = f.client.get_vesting_schedule(&tl_id, &0u64);
    assert!(s.released);
}

// ══════════════════════════════════════════════════════════════════════════════
// 11. get_vesting_schedule helper (used internally above — verify here directly)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn get_vesting_schedule_nonexistent_panics() {
    let f = setup();
    let tl_id = create_default_timeline(&f);
    let err = soroban_sdk::Error::from_contract_error(shade::errors::ContractError::InvoiceNotFound as u32);
    let res = f.client.try_get_vesting_schedule(&tl_id, &77u64);
    assert!(matches!(res, Err(Ok(e)) if e == err));
}
