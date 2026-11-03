//! DAO governance for protocol upgrades.
//!
//! A curated *governance council* (admin-managed allowlist) decides whether the
//! contract's WASM may be upgraded. The flow is:
//!
//!   1. Admin registers council members and sets voting parameters.
//!   2. A member `propose_upgrade`s a target WASM hash, opening a voting window.
//!   3. Members `vote_on_upgrade` (one member, one vote) before the window ends.
//!   4. After the window closes, any member `finalize`s: if quorum + a simple
//!      majority approve, the upgrade is applied; otherwise the proposal is
//!      marked `Defeated`.
//!
//! This preserves the existing admin emergency `upgrade` path while adding a
//! decentralized route. Replay/double-action is prevented by per-member vote
//! flags and the proposal status machine, so concurrent calls converge safely.
//!
//! All voting parameters and counters live in a single `GovState` record to
//! respect Soroban's 50-case cap on `#[contracttype]` enums (DataKey) and to
//! keep the storage footprint small.

use shade::components::core;
use crate::from_shade::errors::GovernanceError;
use crate::from_shade::events;
use crate::from_shade::types::{GovKey, GovState, ProposalStatus, UpgradeProposal};
use soroban_sdk::{panic_with_error, Address, BytesN, Env};

const MAX_QUORUM_BPS: u32 = 10_000;

fn load_state(env: &Env) -> GovState {
    env.storage()
        .persistent()
        .get(&GovKey::State)
        .unwrap_or(GovState {
            voting_period: 0,
            quorum_bps: 0,
            member_count: 0,
            proposal_count: 0,
        })
}

fn save_state(env: &Env, state: &GovState) {
    env.storage().persistent().set(&GovKey::State, state);
}

pub fn is_gov_member(env: &Env, member: &Address) -> bool {
    env.storage()
        .persistent()
        .has(&GovKey::Member(member.clone()))
}

pub fn get_gov_member_count(env: &Env) -> u32 {
    load_state(env).member_count
}

/// Register a council member. Admin only; idempotent.
pub fn add_gov_member(env: &Env, admin: &Address, member: &Address) {
    core::assert_admin(env, admin);
    if is_gov_member(env, member) {
        return;
    }
    env.storage()
        .persistent()
        .set(&GovKey::Member(member.clone()), &true);
    let mut state = load_state(env);
    state.member_count = state.member_count.saturating_add(1);
    save_state(env, &state);
    events::publish_gov_member_added_event(
        env,
        admin.clone(),
        member.clone(),
        state.member_count,
        env.ledger().timestamp(),
    );
}

/// Revoke a council member. Admin only; idempotent.
pub fn remove_gov_member(env: &Env, admin: &Address, member: &Address) {
    core::assert_admin(env, admin);
    if !is_gov_member(env, member) {
        return;
    }
    env.storage()
        .persistent()
        .remove(&GovKey::Member(member.clone()));
    let mut state = load_state(env);
    state.member_count = state.member_count.saturating_sub(1);
    save_state(env, &state);
    events::publish_gov_member_removed_event(
        env,
        admin.clone(),
        member.clone(),
        state.member_count,
        env.ledger().timestamp(),
    );
}

/// Set the voting window length (seconds) and approval quorum (bps of members).
/// Admin only.
pub fn set_governance_config(env: &Env, admin: &Address, voting_period: u64, quorum_bps: u32) {
    core::assert_admin(env, admin);
    if voting_period == 0 || quorum_bps == 0 || quorum_bps > MAX_QUORUM_BPS {
        panic_with_error!(env, GovernanceError::InvalidGovConfig);
    }
    let mut state = load_state(env);
    state.voting_period = voting_period;
    state.quorum_bps = quorum_bps;
    save_state(env, &state);
    events::publish_gov_config_set_event(
        env,
        admin.clone(),
        voting_period,
        quorum_bps,
        env.ledger().timestamp(),
    );
}

fn assert_member(env: &Env, caller: &Address) {
    caller.require_auth();
    if !is_gov_member(env, caller) {
        panic_with_error!(env, GovernanceError::NotGovMember);
    }
}

/// Open a new upgrade proposal. Caller must be a council member. Returns its id.
pub fn propose_upgrade(env: &Env, proposer: &Address, wasm_hash: BytesN<32>) -> u64 {
    assert_member(env, proposer);

    let mut state = load_state(env);
    if state.voting_period == 0 {
        panic_with_error!(env, GovernanceError::GovNotConfigured);
    }

    let now = env.ledger().timestamp();
    let id = state.proposal_count + 1;
    state.proposal_count = id;
    save_state(env, &state);

    let proposal = UpgradeProposal {
        id,
        proposer: proposer.clone(),
        wasm_hash: wasm_hash.clone(),
        created_at: now,
        voting_ends_at: now.saturating_add(state.voting_period),
        approvals: 0,
        rejections: 0,
        status: ProposalStatus::Active,
    };
    env.storage()
        .persistent()
        .set(&GovKey::Proposal(id), &proposal);

    events::publish_upgrade_proposed_event(
        env,
        id,
        proposer.clone(),
        wasm_hash,
        proposal.voting_ends_at,
        now,
    );
    id
}

pub fn get_upgrade_proposal(env: &Env, proposal_id: u64) -> Option<UpgradeProposal> {
    env.storage()
        .persistent()
        .get(&GovKey::Proposal(proposal_id))
}

pub fn has_voted(env: &Env, proposal_id: u64, member: &Address) -> bool {
    env.storage()
        .persistent()
        .has(&GovKey::Vote(proposal_id, member.clone()))
}

fn load_proposal(env: &Env, proposal_id: u64) -> UpgradeProposal {
    env.storage()
        .persistent()
        .get(&GovKey::Proposal(proposal_id))
        .unwrap_or_else(|| panic_with_error!(env, GovernanceError::ProposalNotFound))
}

/// Cast a one-member-one-vote ballot on an active proposal within its window.
pub fn vote_on_upgrade(env: &Env, voter: &Address, proposal_id: u64, approve: bool) {
    assert_member(env, voter);

    let mut proposal = load_proposal(env, proposal_id);
    if proposal.status != ProposalStatus::Active {
        panic_with_error!(env, GovernanceError::ProposalNotActive);
    }
    if env.ledger().timestamp() > proposal.voting_ends_at {
        panic_with_error!(env, GovernanceError::VotingClosed);
    }
    if has_voted(env, proposal_id, voter) {
        panic_with_error!(env, GovernanceError::AlreadyVoted);
    }

    env.storage()
        .persistent()
        .set(&GovKey::Vote(proposal_id, voter.clone()), &approve);
    if approve {
        proposal.approvals = proposal.approvals.saturating_add(1);
    } else {
        proposal.rejections = proposal.rejections.saturating_add(1);
    }
    env.storage()
        .persistent()
        .set(&GovKey::Proposal(proposal_id), &proposal);

    events::publish_upgrade_vote_cast_event(
        env,
        proposal_id,
        voter.clone(),
        approve,
        proposal.approvals,
        proposal.rejections,
        env.ledger().timestamp(),
    );
}

/// Minimum approving votes required: ceil(member_count * quorum_bps / 10_000),
/// floored at 1 so an empty council can never auto-pass.
fn required_quorum(members: u32, quorum_bps: u32) -> u32 {
    let numerator = members as u64 * quorum_bps as u64;
    let required = numerator.div_ceil(MAX_QUORUM_BPS as u64) as u32;
    required.max(1)
}

/// Finalize a proposal after its voting window closes. Any member may call this.
/// Applies the upgrade when quorum and a simple majority approve; otherwise
/// marks the proposal `Defeated`. Either way the proposal is closed exactly once.
pub fn finalize_upgrade(env: &Env, caller: &Address, proposal_id: u64) {
    assert_member(env, caller);

    let mut proposal = load_proposal(env, proposal_id);
    if proposal.status != ProposalStatus::Active {
        panic_with_error!(env, GovernanceError::ProposalNotActive);
    }
    if env.ledger().timestamp() <= proposal.voting_ends_at {
        panic_with_error!(env, GovernanceError::VotingStillOpen);
    }

    let state = load_state(env);
    if state.quorum_bps == 0 {
        panic_with_error!(env, GovernanceError::GovNotConfigured);
    }

    let required = required_quorum(state.member_count, state.quorum_bps);
    let total_votes = proposal.approvals.saturating_add(proposal.rejections);
    let approved = total_votes >= required && proposal.approvals > proposal.rejections;

    proposal.status = if approved {
        ProposalStatus::Executed
    } else {
        ProposalStatus::Defeated
    };
    env.storage()
        .persistent()
        .set(&GovKey::Proposal(proposal_id), &proposal);

    if approved {
        env.deployer()
            .update_current_contract_wasm(proposal.wasm_hash.clone());
        shade::events::publish_contract_upgraded_event(
            env,
            proposal.wasm_hash.clone(),
            env.ledger().timestamp(),
        );
    }

    events::publish_upgrade_proposal_finalized_event(
        env,
        proposal_id,
        caller.clone(),
        approved,
        proposal.approvals,
        proposal.rejections,
        state.member_count,
        env.ledger().timestamp(),
    );
}
