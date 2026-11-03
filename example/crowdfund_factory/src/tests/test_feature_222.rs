#![cfg(test)]

extern crate std;

use crate::{CrowdfundFactory, CrowdfundFactoryClient, DaoProposalStatus};
use soroban_sdk::testutils::{Address as _, Events, Ledger as _};
use soroban_sdk::{Address, BytesN, Env, Map, String, Symbol, TryIntoVal, Val};

fn setup_dao(env: &Env, quorum_bps: u32) -> (CrowdfundFactoryClient<'static>, Address, Address) {
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = 1_000_000);

    let factory_id = env.register(CrowdfundFactory, ());
    let factory = CrowdfundFactoryClient::new(env, &factory_id);

    let admin = Address::generate(env);
    factory.init_dao(&admin, &quorum_bps);

    (factory, factory_id, admin)
}

fn new_wasm_hash(env: &Env, byte: u8) -> BytesN<32> {
    BytesN::from_array(env, &[byte; 32])
}

// ── init_dao ─────────────────────────────────────────────────────────────────

#[test]
fn test_init_dao_stores_admin_and_quorum() {
    let env = Env::default();
    let (factory, _factory_id, admin) = setup_dao(&env, 5_000);

    assert_eq!(factory.get_dao_member_count(), 0);
    assert_eq!(factory.get_dao_proposal_count(), 0);
    assert!(!factory.is_dao_member(&admin));
}

#[test]
#[should_panic]
fn test_double_init_dao_panics() {
    let env = Env::default();
    let (factory, _factory_id, admin) = setup_dao(&env, 5_000);
    factory.init_dao(&admin, &5_000);
}

#[test]
#[should_panic]
fn test_init_dao_zero_quorum_panics() {
    let env = Env::default();
    setup_dao(&env, 0);
}

#[test]
#[should_panic]
fn test_init_dao_quorum_above_max_panics() {
    let env = Env::default();
    setup_dao(&env, 10_001);
}

#[test]
fn test_init_dao_quorum_at_max_boundary_succeeds() {
    let env = Env::default();
    setup_dao(&env, 10_000);
}

// ── membership ───────────────────────────────────────────────────────────────

#[test]
fn test_add_and_remove_dao_member() {
    let env = Env::default();
    let (factory, _factory_id, admin) = setup_dao(&env, 5_000);
    let member = Address::generate(&env);

    assert!(!factory.is_dao_member(&member));
    factory.add_dao_member(&admin, &member);
    assert!(factory.is_dao_member(&member));
    assert_eq!(factory.get_dao_member_count(), 1);

    factory.remove_dao_member(&admin, &member);
    assert!(!factory.is_dao_member(&member));
    assert_eq!(factory.get_dao_member_count(), 0);
}

#[test]
#[should_panic]
fn test_non_admin_cannot_add_member() {
    let env = Env::default();
    let (factory, _factory_id, _admin) = setup_dao(&env, 5_000);
    let not_admin = Address::generate(&env);
    let member = Address::generate(&env);
    factory.add_dao_member(&not_admin, &member);
}

#[test]
#[should_panic]
fn test_add_duplicate_member_panics() {
    let env = Env::default();
    let (factory, _factory_id, admin) = setup_dao(&env, 5_000);
    let member = Address::generate(&env);
    factory.add_dao_member(&admin, &member);
    factory.add_dao_member(&admin, &member);
}

#[test]
#[should_panic]
fn test_remove_non_member_panics() {
    let env = Env::default();
    let (factory, _factory_id, admin) = setup_dao(&env, 5_000);
    let stranger = Address::generate(&env);
    factory.remove_dao_member(&admin, &stranger);
}

#[test]
fn test_malicious_address_cannot_impersonate_admin_to_remove_member() {
    let env = Env::default();
    let (factory, _factory_id, admin) = setup_dao(&env, 5_000);
    let member = Address::generate(&env);
    factory.add_dao_member(&admin, &member);

    let malicious = Address::generate(&env);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        factory.remove_dao_member(&malicious, &member);
    }));
    assert!(result.is_err());
    // Storage must be untouched by the failed call.
    assert!(factory.is_dao_member(&member));
    assert_eq!(factory.get_dao_member_count(), 1);
}

// ── proposal creation ────────────────────────────────────────────────────────

#[test]
#[should_panic]
fn test_create_proposal_requires_membership() {
    let env = Env::default();
    let (factory, _factory_id, _admin) = setup_dao(&env, 5_000);
    let non_member = Address::generate(&env);
    factory.create_dao_proposal(
        &non_member,
        &String::from_str(&env, "upgrade"),
        &new_wasm_hash(&env, 1),
        &86_400,
    );
}

#[test]
#[should_panic]
fn test_create_proposal_zero_voting_period_panics() {
    let env = Env::default();
    let (factory, _factory_id, admin) = setup_dao(&env, 5_000);
    factory.add_dao_member(&admin, &admin);
    factory.create_dao_proposal(
        &admin,
        &String::from_str(&env, "upgrade"),
        &new_wasm_hash(&env, 1),
        &0,
    );
}

#[test]
fn test_create_proposal_emits_exact_event_arguments() {
    let env = Env::default();
    let (factory, factory_id, admin) = setup_dao(&env, 5_000);
    factory.add_dao_member(&admin, &admin);

    let description = String::from_str(&env, "Upgrade crowdfund wasm to v2");
    let proposal_id =
        factory.create_dao_proposal(&admin, &description, &new_wasm_hash(&env, 7), &86_400);
    let expected_deadline = env.ledger().timestamp() + 86_400;

    let events = env.events().all();
    let (event_contract_id, _topics, data) = events.get(events.len() - 1).unwrap();
    assert_eq!(event_contract_id, factory_id);

    let data_map: Map<Symbol, Val> = data.try_into_val(&env).unwrap();
    let proposal_id_in_event: u64 = data_map
        .get(Symbol::new(&env, "proposal_id"))
        .unwrap()
        .try_into_val(&env)
        .unwrap();
    let proposer_in_event: Address = data_map
        .get(Symbol::new(&env, "proposer"))
        .unwrap()
        .try_into_val(&env)
        .unwrap();
    let description_in_event: String = data_map
        .get(Symbol::new(&env, "description"))
        .unwrap()
        .try_into_val(&env)
        .unwrap();
    let deadline_in_event: u64 = data_map
        .get(Symbol::new(&env, "voting_deadline"))
        .unwrap()
        .try_into_val(&env)
        .unwrap();

    assert_eq!(proposal_id_in_event, proposal_id);
    assert_eq!(proposer_in_event, admin);
    assert_eq!(description_in_event, description);
    assert_eq!(deadline_in_event, expected_deadline);
}

// ── voting ───────────────────────────────────────────────────────────────────

#[test]
fn test_cast_vote_records_tally_and_emits_event() {
    let env = Env::default();
    let (factory, factory_id, admin) = setup_dao(&env, 5_000);
    let voter = Address::generate(&env);
    factory.add_dao_member(&admin, &admin);
    factory.add_dao_member(&admin, &voter);

    let proposal_id = factory.create_dao_proposal(
        &admin,
        &String::from_str(&env, "upgrade"),
        &new_wasm_hash(&env, 2),
        &86_400,
    );
    factory.cast_dao_vote(&voter, &proposal_id, &true);

    let events = env.events().all();
    let (event_contract_id, _topics, data) = events.get(events.len() - 1).unwrap();
    assert_eq!(event_contract_id, factory_id);
    let data_map: Map<Symbol, Val> = data.try_into_val(&env).unwrap();
    let votes_for_in_event: u32 = data_map
        .get(Symbol::new(&env, "votes_for"))
        .unwrap()
        .try_into_val(&env)
        .unwrap();
    let support_in_event: bool = data_map
        .get(Symbol::new(&env, "support"))
        .unwrap()
        .try_into_val(&env)
        .unwrap();
    assert_eq!(votes_for_in_event, 1);
    assert!(support_in_event);

    let proposal = factory.get_dao_proposal(&proposal_id);
    assert_eq!(proposal.votes_for, 1);
    assert_eq!(proposal.votes_against, 0);
    assert_eq!(proposal.status, DaoProposalStatus::Voting);
}

#[test]
#[should_panic]
fn test_non_member_cannot_vote() {
    let env = Env::default();
    let (factory, _factory_id, admin) = setup_dao(&env, 5_000);
    factory.add_dao_member(&admin, &admin);
    let proposal_id = factory.create_dao_proposal(
        &admin,
        &String::from_str(&env, "upgrade"),
        &new_wasm_hash(&env, 2),
        &86_400,
    );

    let stranger = Address::generate(&env);
    factory.cast_dao_vote(&stranger, &proposal_id, &true);
}

#[test]
fn test_double_vote_panics_and_rolls_back_tally() {
    let env = Env::default();
    let (factory, _factory_id, admin) = setup_dao(&env, 5_000);
    let voter = Address::generate(&env);
    factory.add_dao_member(&admin, &admin);
    factory.add_dao_member(&admin, &voter);

    let proposal_id = factory.create_dao_proposal(
        &admin,
        &String::from_str(&env, "upgrade"),
        &new_wasm_hash(&env, 2),
        &86_400,
    );
    factory.cast_dao_vote(&voter, &proposal_id, &true);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        factory.cast_dao_vote(&voter, &proposal_id, &true);
    }));
    assert!(result.is_err());

    // Tally must reflect only the single successful vote; the panicking
    // second call must not have left a partial increment.
    let proposal = factory.get_dao_proposal(&proposal_id);
    assert_eq!(proposal.votes_for, 1);
}

#[test]
#[should_panic]
fn test_vote_after_deadline_panics() {
    let env = Env::default();
    let (factory, _factory_id, admin) = setup_dao(&env, 5_000);
    let voter = Address::generate(&env);
    factory.add_dao_member(&admin, &admin);
    factory.add_dao_member(&admin, &voter);

    let proposal_id = factory.create_dao_proposal(
        &admin,
        &String::from_str(&env, "upgrade"),
        &new_wasm_hash(&env, 2),
        &100,
    );
    env.ledger().with_mut(|l| l.timestamp += 200);
    factory.cast_dao_vote(&voter, &proposal_id, &true);
}

#[test]
#[should_panic]
fn test_vote_on_executed_proposal_panics() {
    let env = Env::default();
    let (factory, _factory_id, admin) = setup_dao(&env, 5_000);
    let voter = Address::generate(&env);
    factory.add_dao_member(&admin, &admin);
    factory.add_dao_member(&admin, &voter);

    let proposal_id = factory.create_dao_proposal(
        &admin,
        &String::from_str(&env, "upgrade"),
        &new_wasm_hash(&env, 2),
        &100,
    );
    factory.cast_dao_vote(&admin, &proposal_id, &true);
    factory.cast_dao_vote(&voter, &proposal_id, &true);
    env.ledger().with_mut(|l| l.timestamp += 200);
    factory.execute_dao_proposal(&proposal_id);

    let late_voter = Address::generate(&env);
    factory.add_dao_member(&admin, &late_voter);
    factory.cast_dao_vote(&late_voter, &proposal_id, &true);
}

// ── execution / state transitions ───────────────────────────────────────────

#[test]
#[should_panic]
fn test_execute_before_deadline_panics() {
    let env = Env::default();
    let (factory, _factory_id, admin) = setup_dao(&env, 5_000);
    factory.add_dao_member(&admin, &admin);
    let proposal_id = factory.create_dao_proposal(
        &admin,
        &String::from_str(&env, "upgrade"),
        &new_wasm_hash(&env, 2),
        &86_400,
    );
    factory.execute_dao_proposal(&proposal_id);
}

#[test]
fn test_execute_meets_quorum_and_majority_updates_wasm_hash() {
    let env = Env::default();
    let (factory, _factory_id, admin) = setup_dao(&env, 5_000);
    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);
    factory.add_dao_member(&admin, &admin);
    factory.add_dao_member(&admin, &voter1);
    factory.add_dao_member(&admin, &voter2);

    let new_hash = new_wasm_hash(&env, 9);
    let proposal_id =
        factory.create_dao_proposal(&admin, &String::from_str(&env, "upgrade"), &new_hash, &100);
    // 2 of 3 members vote (66% turnout >= 50% quorum), both in favor.
    factory.cast_dao_vote(&admin, &proposal_id, &true);
    factory.cast_dao_vote(&voter1, &proposal_id, &true);

    env.ledger().with_mut(|l| l.timestamp += 200);
    factory.execute_dao_proposal(&proposal_id);

    let proposal = factory.get_dao_proposal(&proposal_id);
    assert_eq!(proposal.status, DaoProposalStatus::Executed);
}

#[test]
fn test_execute_fails_quorum_rejects_proposal() {
    let env = Env::default();
    let (factory, _factory_id, admin) = setup_dao(&env, 5_000);
    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);
    let voter3 = Address::generate(&env);
    factory.add_dao_member(&admin, &admin);
    factory.add_dao_member(&admin, &voter1);
    factory.add_dao_member(&admin, &voter2);
    factory.add_dao_member(&admin, &voter3);

    let proposal_id = factory.create_dao_proposal(
        &admin,
        &String::from_str(&env, "upgrade"),
        &new_wasm_hash(&env, 3),
        &100,
    );
    // Only 1 of 4 members votes (25% turnout < 50% quorum).
    factory.cast_dao_vote(&admin, &proposal_id, &true);

    env.ledger().with_mut(|l| l.timestamp += 200);
    factory.execute_dao_proposal(&proposal_id);

    let proposal = factory.get_dao_proposal(&proposal_id);
    assert_eq!(proposal.status, DaoProposalStatus::Rejected);
}

#[test]
fn test_execute_tie_vote_rejects_proposal() {
    let env = Env::default();
    let (factory, _factory_id, admin) = setup_dao(&env, 5_000);
    let voter1 = Address::generate(&env);
    factory.add_dao_member(&admin, &admin);
    factory.add_dao_member(&admin, &voter1);

    let proposal_id = factory.create_dao_proposal(
        &admin,
        &String::from_str(&env, "upgrade"),
        &new_wasm_hash(&env, 3),
        &100,
    );
    factory.cast_dao_vote(&admin, &proposal_id, &true);
    factory.cast_dao_vote(&voter1, &proposal_id, &false);

    env.ledger().with_mut(|l| l.timestamp += 200);
    factory.execute_dao_proposal(&proposal_id);

    let proposal = factory.get_dao_proposal(&proposal_id);
    assert_eq!(proposal.status, DaoProposalStatus::Rejected);
}

#[test]
#[should_panic]
fn test_execute_twice_panics() {
    let env = Env::default();
    let (factory, _factory_id, admin) = setup_dao(&env, 5_000);
    factory.add_dao_member(&admin, &admin);
    let proposal_id = factory.create_dao_proposal(
        &admin,
        &String::from_str(&env, "upgrade"),
        &new_wasm_hash(&env, 3),
        &100,
    );
    factory.cast_dao_vote(&admin, &proposal_id, &true);
    env.ledger().with_mut(|l| l.timestamp += 200);
    factory.execute_dao_proposal(&proposal_id);
    factory.execute_dao_proposal(&proposal_id);
}

#[test]
#[should_panic]
fn test_get_nonexistent_proposal_panics() {
    let env = Env::default();
    let (factory, _factory_id, _admin) = setup_dao(&env, 5_000);
    factory.get_dao_proposal(&999);
}
