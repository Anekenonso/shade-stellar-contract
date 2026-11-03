#![cfg(test)]

use shade::shade::{Shade, ShadeClient};
use crate::from_shade::types::{GovKey, ProposalStatus, UpgradeProposal};
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{Address, BytesN, Env};

const V2_WASM: &[u8] = include_bytes!("fixtures/upgrade_v2_contract.wasm");
const VOTING_PERIOD: u64 = 86_400; // 1 day

struct GovCtx<'a> {
    env: Env,
    client: ShadeClient<'a>,
    contract_id: Address,
    admin: Address,
    members: [Address; 3],
    wasm_hash: BytesN<32>,
}

/// Campaign with a 3-member council and configured voting params.
fn setup(quorum_bps: u32) -> GovCtx<'static> {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_700_000_000);

    let contract_id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let members = [
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
    ];
    for m in members.iter() {
        client.add_gov_member(&admin, m);
    }
    client.set_governance_config(&admin, &VOTING_PERIOD, &quorum_bps);

    let wasm_hash = env.deployer().upload_contract_wasm(V2_WASM);
    GovCtx {
        env,
        client,
        contract_id,
        admin,
        members,
        wasm_hash,
    }
}

fn advance_past_voting(ctx: &GovCtx) {
    ctx.env
        .ledger()
        .set_timestamp(ctx.env.ledger().timestamp() + VOTING_PERIOD + 1);
}

// ── Membership & config ───────────────────────────────────────────────────────

#[test]
fn test_membership_management_is_idempotent() {
    let ctx = setup(6_000);
    assert_eq!(ctx.client.get_gov_member_count(), 3);
    assert!(ctx.client.is_gov_member(&ctx.members[0]));

    // Re-adding is a no-op.
    ctx.client.add_gov_member(&ctx.admin, &ctx.members[0]);
    assert_eq!(ctx.client.get_gov_member_count(), 3);

    ctx.client.remove_gov_member(&ctx.admin, &ctx.members[0]);
    assert_eq!(ctx.client.get_gov_member_count(), 2);
    assert!(!ctx.client.is_gov_member(&ctx.members[0]));

    // Removing again is a no-op.
    ctx.client.remove_gov_member(&ctx.admin, &ctx.members[0]);
    assert_eq!(ctx.client.get_gov_member_count(), 2);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_non_admin_cannot_add_member() {
    let ctx = setup(6_000);
    let stranger = Address::generate(&ctx.env);
    let new_member = Address::generate(&ctx.env);
    ctx.client.add_gov_member(&stranger, &new_member);
}

#[test]
#[should_panic(expected = "Error(Contract, #102)")]
fn test_invalid_config_rejected() {
    let ctx = setup(6_000);
    // quorum > 100%
    ctx.client
        .set_governance_config(&ctx.admin, &VOTING_PERIOD, &10_001);
}

// ── Proposal lifecycle ────────────────────────────────────────────────────────

#[test]
fn test_full_upgrade_flow_passes_and_executes() {
    let ctx = setup(6_000); // required = ceil(3 * 0.6) = 2
    let id = ctx.client.propose_upgrade(&ctx.members[0], &ctx.wasm_hash);
    assert_eq!(id, 1);

    ctx.client.vote_on_upgrade(&ctx.members[0], &id, &true);
    ctx.client.vote_on_upgrade(&ctx.members[1], &id, &true);

    let p = ctx.client.get_upgrade_proposal(&id).unwrap();
    assert_eq!(p.approvals, 2);
    assert_eq!(p.rejections, 0);
    assert_eq!(p.status, ProposalStatus::Active);
    assert!(ctx.client.has_voted_on_upgrade(&id, &ctx.members[0]));

    advance_past_voting(&ctx);
    ctx.client.finalize_upgrade(&ctx.members[2], &id);

    // The contract WASM has now been replaced by the V2 fixture, so we read the
    // finalized proposal straight from storage rather than via a contract call.
    let stored: UpgradeProposal = ctx.env.as_contract(&ctx.contract_id, || {
        ctx.env
            .storage()
            .persistent()
            .get(&GovKey::Proposal(id))
            .unwrap()
    });
    assert_eq!(stored.status, ProposalStatus::Executed);
}

#[test]
fn test_proposal_defeated_when_quorum_not_reached() {
    let ctx = setup(6_000); // required = 2 approving-side votes total
    let id = ctx.client.propose_upgrade(&ctx.members[0], &ctx.wasm_hash);

    // Only one member votes → total votes (1) < required quorum (2).
    ctx.client.vote_on_upgrade(&ctx.members[0], &id, &true);

    advance_past_voting(&ctx);
    ctx.client.finalize_upgrade(&ctx.members[0], &id);

    assert_eq!(
        ctx.client.get_upgrade_proposal(&id).unwrap().status,
        ProposalStatus::Defeated
    );
}

#[test]
fn test_proposal_defeated_on_tie() {
    let ctx = setup(6_000);
    let id = ctx.client.propose_upgrade(&ctx.members[0], &ctx.wasm_hash);

    // Quorum met (2 votes) but no majority (1 approve, 1 reject).
    ctx.client.vote_on_upgrade(&ctx.members[0], &id, &true);
    ctx.client.vote_on_upgrade(&ctx.members[1], &id, &false);

    advance_past_voting(&ctx);
    ctx.client.finalize_upgrade(&ctx.members[0], &id);

    assert_eq!(
        ctx.client.get_upgrade_proposal(&id).unwrap().status,
        ProposalStatus::Defeated
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #100)")]
fn test_non_member_cannot_propose() {
    let ctx = setup(6_000);
    let stranger = Address::generate(&ctx.env);
    ctx.client.propose_upgrade(&stranger, &ctx.wasm_hash);
}

#[test]
#[should_panic(expected = "Error(Contract, #100)")]
fn test_non_member_cannot_vote() {
    let ctx = setup(6_000);
    let id = ctx.client.propose_upgrade(&ctx.members[0], &ctx.wasm_hash);
    let stranger = Address::generate(&ctx.env);
    ctx.client.vote_on_upgrade(&stranger, &id, &true);
}

#[test]
#[should_panic(expected = "Error(Contract, #107)")]
fn test_double_vote_rejected() {
    let ctx = setup(6_000);
    let id = ctx.client.propose_upgrade(&ctx.members[0], &ctx.wasm_hash);
    ctx.client.vote_on_upgrade(&ctx.members[0], &id, &true);
    ctx.client.vote_on_upgrade(&ctx.members[0], &id, &true);
}

#[test]
#[should_panic(expected = "Error(Contract, #105)")]
fn test_vote_after_window_closed_rejected() {
    let ctx = setup(6_000);
    let id = ctx.client.propose_upgrade(&ctx.members[0], &ctx.wasm_hash);
    advance_past_voting(&ctx);
    ctx.client.vote_on_upgrade(&ctx.members[0], &id, &true);
}

#[test]
#[should_panic(expected = "Error(Contract, #106)")]
fn test_finalize_before_window_closes_rejected() {
    let ctx = setup(6_000);
    let id = ctx.client.propose_upgrade(&ctx.members[0], &ctx.wasm_hash);
    ctx.client.vote_on_upgrade(&ctx.members[0], &id, &true);
    // Voting window still open.
    ctx.client.finalize_upgrade(&ctx.members[1], &id);
}

#[test]
#[should_panic(expected = "Error(Contract, #104)")]
fn test_cannot_finalize_twice() {
    let ctx = setup(6_000);
    let id = ctx.client.propose_upgrade(&ctx.members[0], &ctx.wasm_hash);
    // A single vote leaves the proposal short of quorum, so finalizing marks it
    // Defeated without swapping the WASM — letting us call finalize again.
    ctx.client.vote_on_upgrade(&ctx.members[0], &id, &true);
    advance_past_voting(&ctx);
    ctx.client.finalize_upgrade(&ctx.members[0], &id);
    ctx.client.finalize_upgrade(&ctx.members[0], &id);
}

#[test]
#[should_panic(expected = "Error(Contract, #103)")]
fn test_vote_on_unknown_proposal_rejected() {
    let ctx = setup(6_000);
    ctx.client.vote_on_upgrade(&ctx.members[0], &999, &true);
}

#[test]
fn test_get_unknown_proposal_returns_none() {
    let ctx = setup(6_000);
    assert!(ctx.client.get_upgrade_proposal(&42).is_none());
}

#[test]
#[should_panic(expected = "Error(Contract, #101)")]
fn test_propose_before_config_rejected() {
    // Manually build a council without configuring voting params.
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_700_000_000);
    let contract_id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    let member = Address::generate(&env);
    client.add_gov_member(&admin, &member);

    let wasm_hash = env.deployer().upload_contract_wasm(V2_WASM);
    client.propose_upgrade(&member, &wasm_hash);
}
