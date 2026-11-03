#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{Address, BytesN, Env};

fn register_campaign_ref(
    env: &Env,
    factory_id: &Address,
    organizer: Address,
    contract: Address,
) -> CampaignRef {
    env.as_contract(factory_id, || {
        let campaign_id = get_campaign_count(env) + 1;
        let deployed_at = env.ledger().timestamp();
        let campaign_ref = CampaignRef {
            campaign_id,
            contract,
            organizer,
            deployed_at,
        };
        env.storage()
            .persistent()
            .set(&DataKey::CampaignRef(campaign_id), &campaign_ref);
        env.storage()
            .persistent()
            .set(&DataKey::CampaignRefCount, &campaign_id);
        campaign_ref
    })
}

#[test]
fn test_deploy_campaign_tracks_active_protocols() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|ledger| ledger.timestamp = 1_000_000);

    let factory_id = env.register(CrowdfundFactory, ());
    let factory = CrowdfundFactoryClient::new(&env, &factory_id);
    let fake_wasm_hash = BytesN::from_array(&env, &[7u8; 32]);
    factory.initialize(&fake_wasm_hash);

    let organizer_a = Address::generate(&env);
    let organizer_b = Address::generate(&env);
    let campaign_a = register_campaign_ref(
        &env,
        &factory_id,
        organizer_a.clone(),
        Address::generate(&env),
    );
    let campaign_b = register_campaign_ref(
        &env,
        &factory_id,
        organizer_b.clone(),
        Address::generate(&env),
    );

    assert_eq!(factory.get_campaign_count(), 2);
    assert_eq!(campaign_a.campaign_id, 1);
    assert_eq!(campaign_b.campaign_id, 2);

    let campaigns = factory.get_all_campaigns();
    assert_eq!(campaigns.len(), 2);
    assert_eq!(campaigns.get_unchecked(0).organizer, organizer_a);
    assert_eq!(campaigns.get_unchecked(1).organizer, organizer_b);
}

// ── Campaign approval governance (#358) ──────────────────────────────────────

fn setup_governance(env: &Env) -> (CrowdfundFactoryClient<'static>, Address, Address) {
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = 1_000_000);

    let factory_id = env.register(CrowdfundFactory, ());
    let factory = CrowdfundFactoryClient::new(env, &factory_id);
    // `initialize` only records the hash; these tests never deploy, so a fake
    // hash avoids depending on a built .wasm artifact (which is gitignored).
    // Same pattern as test_deploy_campaign_tracks_active_protocols above.
    let crowdfund_wasm_hash = BytesN::from_array(env, &[7u8; 32]);
    factory.initialize(&crowdfund_wasm_hash);

    let admin = Address::generate(env);
    factory.init_governance(&admin);

    (factory, factory_id, admin)
}

#[test]
fn test_propose_campaign_creates_pending_proposal() {
    let env = Env::default();
    let (factory, _factory_id, _admin) = setup_governance(&env);

    let organizer = Address::generate(&env);
    let token = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 86_400;

    let proposal_id = factory.propose_campaign(&organizer, &token, &10_000, &deadline);
    assert_eq!(proposal_id, 1);

    let proposal = factory.get_campaign_proposal(&proposal_id);
    assert_eq!(proposal.organizer, organizer);
    assert_eq!(proposal.token, token);
    assert_eq!(proposal.goal, 10_000);
    assert_eq!(proposal.deadline, deadline);
    assert_eq!(proposal.status, ProposalStatus::Pending);
    assert_eq!(factory.get_campaign_proposal_count(), 1);
}

#[test]
#[should_panic]
fn test_propose_campaign_without_governance_panics() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = 1_000_000);

    let factory_id = env.register(CrowdfundFactory, ());
    let factory = CrowdfundFactoryClient::new(&env, &factory_id);
    // `initialize` only records the hash; these tests never deploy, so a fake
    // hash avoids depending on a built .wasm artifact (which is gitignored).
    // Same pattern as test_deploy_campaign_tracks_active_protocols above.
    let crowdfund_wasm_hash = BytesN::from_array(&env, &[7u8; 32]);
    factory.initialize(&crowdfund_wasm_hash);

    let organizer = Address::generate(&env);
    let token = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 86_400;
    factory.propose_campaign(&organizer, &token, &10_000, &deadline);
}

#[test]
#[should_panic]
fn test_propose_campaign_invalid_goal_panics() {
    let env = Env::default();
    let (factory, _factory_id, _admin) = setup_governance(&env);
    let organizer = Address::generate(&env);
    let token = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 86_400;
    factory.propose_campaign(&organizer, &token, &0, &deadline);
}

#[test]
#[should_panic]
fn test_propose_campaign_past_deadline_panics() {
    let env = Env::default();
    let (factory, _factory_id, _admin) = setup_governance(&env);
    let organizer = Address::generate(&env);
    let token = Address::generate(&env);
    factory.propose_campaign(&organizer, &token, &10_000, &(env.ledger().timestamp() - 1));
}

#[test]
fn test_admin_can_approve_proposal_directly() {
    let env = Env::default();
    let (factory, _factory_id, admin) = setup_governance(&env);
    let organizer = Address::generate(&env);
    let token = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 86_400;

    let proposal_id = factory.propose_campaign(&organizer, &token, &10_000, &deadline);
    factory.approve_campaign_proposal(&admin, &proposal_id);

    let proposal = factory.get_campaign_proposal(&proposal_id);
    assert_eq!(proposal.status, ProposalStatus::Approved);
}

#[test]
fn test_grant_and_revoke_reviewer() {
    let env = Env::default();
    let (factory, _factory_id, admin) = setup_governance(&env);
    let reviewer = Address::generate(&env);

    assert!(!factory.is_reviewer(&reviewer));
    factory.grant_reviewer(&admin, &reviewer);
    assert!(factory.is_reviewer(&reviewer));
    factory.revoke_reviewer(&admin, &reviewer);
    assert!(!factory.is_reviewer(&reviewer));
}

#[test]
#[should_panic]
fn test_non_admin_cannot_grant_reviewer() {
    let env = Env::default();
    let (factory, _factory_id, _admin) = setup_governance(&env);
    let not_admin = Address::generate(&env);
    let reviewer = Address::generate(&env);
    factory.grant_reviewer(&not_admin, &reviewer);
}

#[test]
fn test_granted_reviewer_can_approve_proposal() {
    let env = Env::default();
    let (factory, _factory_id, admin) = setup_governance(&env);
    let reviewer = Address::generate(&env);
    factory.grant_reviewer(&admin, &reviewer);

    let organizer = Address::generate(&env);
    let token = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 86_400;
    let proposal_id = factory.propose_campaign(&organizer, &token, &10_000, &deadline);

    factory.approve_campaign_proposal(&reviewer, &proposal_id);
    assert_eq!(
        factory.get_campaign_proposal(&proposal_id).status,
        ProposalStatus::Approved
    );
}

#[test]
#[should_panic]
fn test_unauthorized_address_cannot_approve_proposal() {
    let env = Env::default();
    let (factory, _factory_id, _admin) = setup_governance(&env);
    let organizer = Address::generate(&env);
    let token = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 86_400;
    let proposal_id = factory.propose_campaign(&organizer, &token, &10_000, &deadline);

    let malicious = Address::generate(&env);
    factory.approve_campaign_proposal(&malicious, &proposal_id);
}

#[test]
fn test_reject_proposal_sets_status_and_blocks_further_action() {
    let env = Env::default();
    let (factory, _factory_id, admin) = setup_governance(&env);
    let organizer = Address::generate(&env);
    let token = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 86_400;
    let proposal_id = factory.propose_campaign(&organizer, &token, &10_000, &deadline);

    let reason = String::from_str(&env, "insufficient documentation");
    factory.reject_campaign_proposal(&admin, &proposal_id, &reason);

    assert_eq!(
        factory.get_campaign_proposal(&proposal_id).status,
        ProposalStatus::Rejected
    );
}

#[test]
#[should_panic]
fn test_cannot_approve_already_rejected_proposal() {
    let env = Env::default();
    let (factory, _factory_id, admin) = setup_governance(&env);
    let organizer = Address::generate(&env);
    let token = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 86_400;
    let proposal_id = factory.propose_campaign(&organizer, &token, &10_000, &deadline);

    let reason = String::from_str(&env, "no");
    factory.reject_campaign_proposal(&admin, &proposal_id, &reason);
    factory.approve_campaign_proposal(&admin, &proposal_id);
}

#[test]
#[should_panic]
fn test_cannot_approve_already_approved_proposal() {
    let env = Env::default();
    let (factory, _factory_id, admin) = setup_governance(&env);
    let organizer = Address::generate(&env);
    let token = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 86_400;
    let proposal_id = factory.propose_campaign(&organizer, &token, &10_000, &deadline);

    factory.approve_campaign_proposal(&admin, &proposal_id);
    factory.approve_campaign_proposal(&admin, &proposal_id);
}

// Ignored: this is the only governance test that performs a real on-chain
// deploy, so it needs an uploaded crowdfund WASM. It originally referenced
// `crowdfund_wasm::WASM`, a module that does not exist and is generated
// nowhere in this repo, so it has never run. A native test cannot produce a
// WASM hash for a Rust-defined contract, and `*.wasm` artifacts are gitignored
// (CI's test job also runs before the build job). Re-enable by committing a
// crowdfund WASM fixture and pulling it in with `contractimport!`.
#[test]
#[ignore = "needs an uploaded crowdfund WASM; see comment above"]
fn test_execute_approved_proposal_deploys_campaign() {
    let env = Env::default();
    let (factory, _factory_id, admin) = setup_governance(&env);
    let organizer = Address::generate(&env);
    let token = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 86_400;
    let proposal_id = factory.propose_campaign(&organizer, &token, &10_000, &deadline);

    factory.approve_campaign_proposal(&admin, &proposal_id);
    let campaign_ref = factory.execute_campaign_proposal(&proposal_id);

    assert_eq!(campaign_ref.campaign_id, 1);
    assert_eq!(campaign_ref.organizer, organizer);
    assert_eq!(factory.get_campaign_count(), 1);
    assert_eq!(
        factory.get_campaign_proposal(&proposal_id).status,
        ProposalStatus::Executed
    );
}

#[test]
#[should_panic]
fn test_execute_pending_proposal_panics() {
    let env = Env::default();
    let (factory, _factory_id, _admin) = setup_governance(&env);
    let organizer = Address::generate(&env);
    let token = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 86_400;
    let proposal_id = factory.propose_campaign(&organizer, &token, &10_000, &deadline);

    factory.execute_campaign_proposal(&proposal_id);
}

#[test]
#[should_panic]
fn test_execute_proposal_twice_panics() {
    let env = Env::default();
    let (factory, _factory_id, admin) = setup_governance(&env);
    let organizer = Address::generate(&env);
    let token = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 86_400;
    let proposal_id = factory.propose_campaign(&organizer, &token, &10_000, &deadline);

    factory.approve_campaign_proposal(&admin, &proposal_id);
    factory.execute_campaign_proposal(&proposal_id);
    factory.execute_campaign_proposal(&proposal_id);
}

#[test]
#[should_panic]
fn test_get_nonexistent_proposal_panics() {
    let env = Env::default();
    let (factory, _factory_id, _admin) = setup_governance(&env);
    factory.get_campaign_proposal(&999);
}

#[test]
#[should_panic]
fn test_double_init_governance_panics() {
    let env = Env::default();
    let (factory, _factory_id, _admin) = setup_governance(&env);
    let other_admin = Address::generate(&env);
    factory.init_governance(&other_admin);
}
