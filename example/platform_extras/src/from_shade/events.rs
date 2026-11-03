use soroban_sdk::{contractevent, Address, BytesN, Env, String, Vec};
use super::types::*;
use shade::types::*;

#[contractevent]
pub struct BridgePlaceholderEvent {
    pub caller: Address,
    pub payload: crate::types::CrossChainBridgePayload,
    pub timestamp: u64,
}

pub fn publish_bridge_placeholder_event(
    env: &Env,
    caller: Address,
    payload: crate::types::CrossChainBridgePayload,
    timestamp: u64,
) {
    BridgePlaceholderEvent {
        caller,
        payload,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct BridgeListenerRegisteredEvent {
    pub admin: Address,
    pub listener: Address,
    pub timestamp: u64,
}

pub fn publish_bridge_listener_registered_event(
    env: &Env,
    admin: Address,
    listener: Address,
    timestamp: u64,
) {
    BridgeListenerRegisteredEvent {
        admin,
        listener,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct BridgeListenerRemovedEvent {
    pub admin: Address,
    pub listener: Address,
    pub timestamp: u64,
}

pub fn publish_bridge_listener_removed_event(
    env: &Env,
    admin: Address,
    listener: Address,
    timestamp: u64,
) {
    BridgeListenerRemovedEvent {
        admin,
        listener,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct BridgeDepositRecordedEvent {
    pub deposit_id: u64,
    pub listener: Address,
    pub source_chain: String,
    pub source_tx_id: BytesN<32>,
    pub token: Address,
    pub amount: i128,
    pub recipient: Address,
    pub timestamp: u64,
}

#[allow(clippy::too_many_arguments)]
pub fn publish_bridge_deposit_recorded_event(
    env: &Env,
    deposit_id: u64,
    listener: Address,
    source_chain: String,
    source_tx_id: BytesN<32>,
    token: Address,
    amount: i128,
    recipient: Address,
    timestamp: u64,
) {
    BridgeDepositRecordedEvent {
        deposit_id,
        listener,
        source_chain,
        source_tx_id,
        token,
        amount,
        recipient,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct GovMemberAddedEvent {
    pub admin: Address,
    pub member: Address,
    pub member_count: u32,
    pub timestamp: u64,
}

pub fn publish_gov_member_added_event(
    env: &Env,
    admin: Address,
    member: Address,
    member_count: u32,
    timestamp: u64,
) {
    GovMemberAddedEvent {
        admin,
        member,
        member_count,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct GovMemberRemovedEvent {
    pub admin: Address,
    pub member: Address,
    pub member_count: u32,
    pub timestamp: u64,
}

pub fn publish_gov_member_removed_event(
    env: &Env,
    admin: Address,
    member: Address,
    member_count: u32,
    timestamp: u64,
) {
    GovMemberRemovedEvent {
        admin,
        member,
        member_count,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct GovConfigSetEvent {
    pub admin: Address,
    pub voting_period: u64,
    pub quorum_bps: u32,
    pub timestamp: u64,
}

pub fn publish_gov_config_set_event(
    env: &Env,
    admin: Address,
    voting_period: u64,
    quorum_bps: u32,
    timestamp: u64,
) {
    GovConfigSetEvent {
        admin,
        voting_period,
        quorum_bps,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct UpgradeProposedEvent {
    pub proposal_id: u64,
    pub proposer: Address,
    pub wasm_hash: BytesN<32>,
    pub voting_ends_at: u64,
    pub timestamp: u64,
}

pub fn publish_upgrade_proposed_event(
    env: &Env,
    proposal_id: u64,
    proposer: Address,
    wasm_hash: BytesN<32>,
    voting_ends_at: u64,
    timestamp: u64,
) {
    UpgradeProposedEvent {
        proposal_id,
        proposer,
        wasm_hash,
        voting_ends_at,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct UpgradeVoteCastEvent {
    pub proposal_id: u64,
    pub voter: Address,
    pub approve: bool,
    pub approvals: u32,
    pub rejections: u32,
    pub timestamp: u64,
}

pub fn publish_upgrade_vote_cast_event(
    env: &Env,
    proposal_id: u64,
    voter: Address,
    approve: bool,
    approvals: u32,
    rejections: u32,
    timestamp: u64,
) {
    UpgradeVoteCastEvent {
        proposal_id,
        voter,
        approve,
        approvals,
        rejections,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct UpgradeProposalFinalizedEvent {
    pub proposal_id: u64,
    pub executor: Address,
    pub approved: bool,
    pub approvals: u32,
    pub rejections: u32,
    pub member_count: u32,
    pub timestamp: u64,
}

pub fn publish_upgrade_proposal_finalized_event(
    env: &Env,
    proposal_id: u64,
    executor: Address,
    approved: bool,
    approvals: u32,
    rejections: u32,
    member_count: u32,
    timestamp: u64,
) {
    UpgradeProposalFinalizedEvent {
        proposal_id,
        executor,
        approved,
        approvals,
        rejections,
        member_count,
        timestamp,
    }
    .publish(env);
}

/// Emitted each time a registered signer approves a proposal.
#[contractevent]
pub struct WithdrawalApprovedEvent {
    pub proposal_id: u64,
    /// Signer that cast this approval.
    pub signer: Address,
    /// Running approval count after this vote.
    pub approvals_so_far: u32,
    /// Quorum still needed (0 means ready to execute).
    pub quorum_required: u32,
    pub timestamp: u64,
}

pub fn publish_withdrawal_approved_event(
    env: &Env,
    proposal_id: u64,
    signer: Address,
    approvals_so_far: u32,
    quorum_required: u32,
    timestamp: u64,
) {
    WithdrawalApprovedEvent {
        proposal_id,
        signer,
        approvals_so_far,
        quorum_required,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct MultisigConfiguredEvent {
    pub signers: Vec<Address>,
    pub quorum: u32,
    pub admin: Address,
    pub timestamp: u64,
}

pub fn publish_multisig_configured_event(
    env: &Env,
    signers: Vec<Address>,
    quorum: u32,
    admin: Address,
    timestamp: u64,
) {
    MultisigConfiguredEvent {
        signers,
        quorum,
        admin,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct MultisigThresholdSetEvent {
    pub token: Address,
    pub threshold: i128,
    pub admin: Address,
    pub timestamp: u64,
}

pub fn publish_multisig_threshold_set_event(
    env: &Env,
    token: Address,
    threshold: i128,
    admin: Address,
    timestamp: u64,
) {
    MultisigThresholdSetEvent {
        token,
        threshold,
        admin,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct WithdrawalCancelledEvent {
    pub proposal_id: u64,
    pub caller: Address,
    pub timestamp: u64,
}

pub fn publish_withdrawal_cancelled_event(
    env: &Env,
    proposal_id: u64,
    caller: Address,
    timestamp: u64,
) {
    WithdrawalCancelledEvent {
        proposal_id,
        caller,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct WithdrawalExecutedEvent {
    pub proposal_id: u64,
    pub merchant: Address,
    pub token: Address,
    pub amount: i128,
    pub recipient: Address,
    pub signer: Address,
    pub now: u64,
}

pub fn publish_withdrawal_executed_event(
    env: &Env,
    proposal_id: u64,
    merchant: Address,
    token: Address,
    amount: i128,
    recipient: Address,
    signer: Address,
    now: u64,
) {
    WithdrawalExecutedEvent {
        proposal_id,
        merchant,
        token,
        amount,
        recipient,
        signer,
        now,
    }
    .publish(env);
}

#[contractevent]
pub struct WithdrawalProposalSearchEvent {
    pub caller: Address,
    pub count: u32,
    pub timestamp: u64,
}

pub fn publish_withdrawal_proposal_search_event(
    env: &Env,
    caller: Address,
    count: u32,
    timestamp: u64,
) {
    WithdrawalProposalSearchEvent {
        caller,
        count,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct WithdrawalProposedEvent {
    pub proposal_id: u64,
    pub merchant: Address,
    pub token: Address,
    pub amount: i128,
    pub recipient: Address,
    pub quorum: u32,
    pub now: u64,
}

pub fn publish_withdrawal_proposed_event(
    env: &Env,
    proposal_id: u64,
    merchant: Address,
    token: Address,
    amount: i128,
    recipient: Address,
    quorum: u32,
    now: u64,
) {
    WithdrawalProposedEvent {
        proposal_id,
        merchant,
        token,
        amount,
        recipient,
        quorum,
        now,
    }
    .publish(env);
}
