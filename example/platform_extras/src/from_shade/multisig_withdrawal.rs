//! Multi-sig massive withdrawal component.
//!
//! Any withdrawal whose amount meets or exceeds the per-token threshold must
//! go through a proposal → approval → execution lifecycle instead of being
//! sent directly.  A configurable quorum of registered signers must approve a
//! proposal before funds can move.
//!
//! # Lifecycle
//! 1. Merchant calls `propose_withdrawal` → creates a `WithdrawalProposal`.
//! 2. Each registered signer calls `approve_withdrawal` → increments approval count.
//!    When approvals reach quorum, funds are transferred and the proposal is
//!    marked `Executed` atomically.
//! 3. The proposing merchant (or admin) can call `cancel_withdrawal` at any
//!    time while the proposal is still `Pending`.

use shade::components::core as core_component;
use shade::components::merchant;
use crate::from_shade::errors::{ContractError, MultiSigError};
use crate::from_shade::events;
use crate::from_shade::types::{MultiSigKey, WithdrawalProposal, WithdrawalProposalStatus};
use soroban_sdk::{contractclient, panic_with_error, Address, Env, String, Vec};

// ── Internal cross-contract client ───────────────────────────────────────────

/// Thin client used to pull funds from a merchant-account contract.
#[contractclient(name = "MerchantAccountWithdrawClient")]
pub trait MerchantAccountWithdraw {
    fn withdraw_to(env: Env, token: Address, amount: i128, to: Address);
}

// ── Configuration helpers ─────────────────────────────────────────────────────

/// Set (or update) the withdrawal threshold for a specific token.
/// Only the contract admin may call this.
pub fn set_multisig_threshold(env: &Env, admin: &Address, token: &Address, threshold: i128) {
    core_component::assert_admin(env, admin);
    if threshold < 0 {
        panic_with_error!(env, shade::errors::ContractError::InvalidAmount);
    }
    env.storage()
        .persistent()
        .set(&MultiSigKey::MultiSigThreshold(token.clone()), &threshold);
    events::publish_multisig_threshold_set_event(
        env,
        token.clone(),
        threshold,
        admin.clone(),
        env.ledger().timestamp(),
    );
}

/// Return the configured threshold for `token`, or `None` if not set.
pub fn get_multisig_threshold(env: &Env, token: &Address) -> Option<i128> {
    env.storage()
        .persistent()
        .get(&MultiSigKey::MultiSigThreshold(token.clone()))
}

/// Replace the signer list and quorum in one atomic call.
/// Only the contract admin may call this.
pub fn configure_multisig(env: &Env, admin: &Address, signers: Vec<Address>, quorum: u32) {
    core_component::assert_admin(env, admin);
    if signers.is_empty() {
        panic_with_error!(env, MultiSigError::MultiSigSignersNotSet);
    }
    if quorum == 0 || quorum > signers.len() {
        panic_with_error!(env, MultiSigError::InvalidQuorum);
    }
    env.storage()
        .persistent()
        .set(&MultiSigKey::MultiSigSigners, &signers);
    env.storage()
        .persistent()
        .set(&MultiSigKey::MultiSigQuorum, &quorum);
    events::publish_multisig_configured_event(
        env,
        signers,
        quorum,
        admin.clone(),
        env.ledger().timestamp(),
    );
}

/// Return the registered signers list, panicking if not configured.
pub fn get_signers(env: &Env) -> Vec<Address> {
    env.storage()
        .persistent()
        .get(&MultiSigKey::MultiSigSigners)
        .unwrap_or_else(|| panic_with_error!(env, MultiSigError::MultiSigSignersNotSet))
}

/// Return the required quorum, panicking if not configured.
pub fn get_quorum(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&MultiSigKey::MultiSigQuorum)
        .unwrap_or_else(|| panic_with_error!(env, MultiSigError::InvalidQuorum))
}

// ── Proposal helpers ──────────────────────────────────────────────────────────

fn next_proposal_id(env: &Env) -> u64 {
    let count: u64 = env
        .storage()
        .persistent()
        .get(&MultiSigKey::WithdrawalProposalCount)
        .unwrap_or(0);
    let new_id = count + 1;
    env.storage()
        .persistent()
        .set(&MultiSigKey::WithdrawalProposalCount, &new_id);
    new_id
}

fn load_proposal(env: &Env, proposal_id: u64) -> WithdrawalProposal {
    env.storage()
        .persistent()
        .get(&MultiSigKey::WithdrawalProposal(proposal_id))
        .unwrap_or_else(|| panic_with_error!(env, MultiSigError::ProposalNotFound))
}

fn save_proposal(env: &Env, proposal: &WithdrawalProposal) {
    env.storage()
        .persistent()
        .set(&MultiSigKey::WithdrawalProposal(proposal.id), proposal);
}

fn is_signer(env: &Env, address: &Address) -> bool {
    let signers: Vec<Address> = env
        .storage()
        .persistent()
        .get(&MultiSigKey::MultiSigSigners)
        .unwrap_or_else(|| Vec::new(env));
    for s in signers.iter() {
        if s == *address {
            return true;
        }
    }
    false
}

// ── Public actions ────────────────────────────────────────────────────────────

/// Open a new multi-sig withdrawal proposal.
///
/// The merchant must be registered and active.  The requested amount must meet
/// or exceed the threshold configured for the given token.  Returns the new
/// proposal ID.
pub fn propose_withdrawal(
    env: &Env,
    merchant: &Address,
    token: &Address,
    amount: i128,
    recipient: &Address,
    note: String,
) -> u64 {
    merchant.require_auth();

    if amount <= 0 {
        panic_with_error!(env, shade::errors::ContractError::InvalidAmount);
    }

    // Validate merchant exists and is active.
    let merchant_id = merchant::get_merchant_id(env, merchant);
    let merchant_data = merchant::get_merchant(env, merchant_id);
    if !merchant_data.active {
        panic_with_error!(env, shade::errors::ContractError::MerchantNotActive);
    }

    // Enforce threshold — caller must not bypass multi-sig for sub-threshold amounts.
    let threshold: i128 = get_multisig_threshold(env, token)
        .unwrap_or_else(|| panic_with_error!(env, MultiSigError::ThresholdNotSet));
    if threshold == 0 || amount < threshold {
        panic_with_error!(env, MultiSigError::BelowMultiSigThreshold);
    }

    // Signers and quorum must already be configured.
    let quorum = get_quorum(env);

    let proposal_id = next_proposal_id(env);
    let now = env.ledger().timestamp();

    let proposal = WithdrawalProposal {
        id: proposal_id,
        merchant: merchant.clone(),
        token: token.clone(),
        amount,
        recipient: recipient.clone(),
        approvals: 0,
        status: WithdrawalProposalStatus::Pending,
        created_at: now,
        updated_at: now,
        note,
    };
    save_proposal(env, &proposal);

    events::publish_withdrawal_proposed_event(
        env,
        proposal_id,
        merchant.clone(),
        token.clone(),
        amount,
        recipient.clone(),
        quorum,
        now,
    );

    proposal_id
}

/// Cast an approval vote for a pending withdrawal proposal.
///
/// If this vote brings the approval count to quorum, the funds are transferred
/// immediately and the proposal is marked `Executed`.
pub fn approve_withdrawal(env: &Env, signer: &Address, proposal_id: u64) {
    signer.require_auth();

    if !is_signer(env, signer) {
        panic_with_error!(env, MultiSigError::NotASigner);
    }

    let mut proposal = load_proposal(env, proposal_id);

    if proposal.status != WithdrawalProposalStatus::Pending {
        panic_with_error!(env, MultiSigError::ProposalNotPending);
    }

    // Prevent the same signer from voting twice.
    let approval_key = MultiSigKey::WithdrawalApproval(proposal_id, signer.clone());
    if env.storage().persistent().has(&approval_key) {
        panic_with_error!(env, MultiSigError::AlreadyApproved);
    }
    env.storage().persistent().set(&approval_key, &true);

    proposal.approvals = proposal.approvals.saturating_add(1);
    proposal.updated_at = env.ledger().timestamp();

    let quorum = get_quorum(env);
    let now = env.ledger().timestamp();

    events::publish_withdrawal_approved_event(
        env,
        proposal_id,
        signer.clone(),
        proposal.approvals,
        quorum,
        now,
    );

    if proposal.approvals >= quorum {
        // Execute: pull funds from the merchant account contract and send to recipient.
        let merchant_id = merchant::get_merchant_id(env, &proposal.merchant);
        let merchant_account = merchant::get_merchant_account(env, merchant_id);

        MerchantAccountWithdrawClient::new(env, &merchant_account).withdraw_to(
            &proposal.token,
            &proposal.amount,
            &proposal.recipient,
        );

        proposal.status = WithdrawalProposalStatus::Executed;
        save_proposal(env, &proposal);

        events::publish_withdrawal_executed_event(
            env,
            proposal_id,
            proposal.merchant.clone(),
            proposal.token.clone(),
            proposal.amount,
            proposal.recipient.clone(),
            signer.clone(),
            now,
        );
    } else {
        save_proposal(env, &proposal);
    }
}

/// Cancel a pending proposal.
///
/// Only the original proposing merchant or the contract admin may cancel.
pub fn cancel_withdrawal(env: &Env, caller: &Address, proposal_id: u64) {
    caller.require_auth();

    let mut proposal = load_proposal(env, proposal_id);

    if proposal.status != WithdrawalProposalStatus::Pending {
        panic_with_error!(env, MultiSigError::ProposalNotPending);
    }

    // Allow either the proposer (merchant) or the admin.
    let admin = core_component::get_admin(env);
    if *caller != proposal.merchant && *caller != admin {
        panic_with_error!(env, MultiSigError::NotProposer);
    }

    proposal.status = WithdrawalProposalStatus::Cancelled;
    proposal.updated_at = env.ledger().timestamp();
    save_proposal(env, &proposal);

    events::publish_withdrawal_cancelled_event(
        env,
        proposal_id,
        caller.clone(),
        env.ledger().timestamp(),
    );
}

/// Read a proposal by ID (read-only).
pub fn get_withdrawal_proposal(env: &Env, proposal_id: u64) -> WithdrawalProposal {
    load_proposal(env, proposal_id)
}

/// Check whether `signer` has already approved `proposal_id`.
pub fn has_approved(env: &Env, signer: &Address, proposal_id: u64) -> bool {
    env.storage()
        .persistent()
        .has(&MultiSigKey::WithdrawalApproval(
            proposal_id,
            signer.clone(),
        ))
}

/// Return the total number of proposals ever created.
pub fn get_proposal_count(env: &Env) -> u64 {
    env.storage()
        .persistent()
        .get(&MultiSigKey::WithdrawalProposalCount)
        .unwrap_or(0)
}
