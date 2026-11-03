//! The entry points these modules exposed on the Shade hub, kept for reference.

use super::types::*;
use shade::types::*;
use soroban_sdk::{Address, BytesN, Env, String, Vec};

pub struct HubEntrypoints;

impl HubEntrypoints {
    pub fn emit_bridge_placeholder(env: Env, caller: Address, payload: CrossChainBridgePayload) {
        shade::components::pausable::assert_not_paused(&env);
        caller.require_auth();
        events::publish_bridge_placeholder_event(&env, caller, payload, env.ledger().timestamp());
    }

    pub fn register_bridge_listener(env: Env, admin: Address, listener: Address) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::bridge::register_bridge_listener(&env, &admin, &listener);
    }

    pub fn remove_bridge_listener(env: Env, admin: Address, listener: Address) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::bridge::remove_bridge_listener(&env, &admin, &listener);
    }

    pub fn is_bridge_listener(env: Env, listener: Address) -> bool {
        crate::from_shade::bridge::is_bridge_listener(&env, &listener)
    }

    pub fn record_bridge_deposit(
        env: Env,
        listener: Address,
        source_chain: String,
        source_tx_id: BytesN<32>,
        token: Address,
        amount: i128,
        recipient: Address,
    ) -> u64 {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::bridge::record_bridge_deposit(
            &env,
            &listener,
            source_chain,
            source_tx_id,
            token,
            amount,
            recipient,
        )
    }

    pub fn get_bridge_deposit(env: Env, deposit_id: u64) -> Option<BridgeDeposit> {
        crate::from_shade::bridge::get_bridge_deposit(&env, deposit_id)
    }

    pub fn is_bridge_deposit_processed(env: Env, source_tx_id: BytesN<32>) -> bool {
        crate::from_shade::bridge::is_bridge_deposit_processed(&env, &source_tx_id)
    }

    pub fn get_bridge_deposit_count(env: Env) -> u64 {
        crate::from_shade::bridge::get_bridge_deposit_count(&env)
    }

    pub fn get_bridge_credit(env: Env, recipient: Address, token: Address) -> i128 {
        crate::from_shade::bridge::get_bridge_credit(&env, &recipient, &token)
    }

    pub fn add_gov_member(env: Env, admin: Address, member: Address) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::governance::add_gov_member(&env, &admin, &member);
    }

    pub fn remove_gov_member(env: Env, admin: Address, member: Address) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::governance::remove_gov_member(&env, &admin, &member);
    }

    pub fn is_gov_member(env: Env, member: Address) -> bool {
        crate::from_shade::governance::is_gov_member(&env, &member)
    }

    pub fn get_gov_member_count(env: Env) -> u32 {
        crate::from_shade::governance::get_gov_member_count(&env)
    }

    pub fn set_governance_config(env: Env, admin: Address, voting_period: u64, quorum_bps: u32) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::governance::set_governance_config(&env, &admin, voting_period, quorum_bps);
    }

    pub fn propose_upgrade(env: Env, proposer: Address, wasm_hash: BytesN<32>) -> u64 {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::governance::propose_upgrade(&env, &proposer, wasm_hash)
    }

    pub fn vote_on_upgrade(env: Env, voter: Address, proposal_id: u64, approve: bool) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::governance::vote_on_upgrade(&env, &voter, proposal_id, approve);
    }

    pub fn finalize_upgrade(env: Env, caller: Address, proposal_id: u64) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::governance::finalize_upgrade(&env, &caller, proposal_id);
    }

    pub fn get_upgrade_proposal(env: Env, proposal_id: u64) -> Option<UpgradeProposal> {
        crate::from_shade::governance::get_upgrade_proposal(&env, proposal_id)
    }

    pub fn has_voted_on_upgrade(env: Env, proposal_id: u64, member: Address) -> bool {
        crate::from_shade::governance::has_voted(&env, proposal_id, &member)
    }

    pub fn set_multisig_threshold(env: Env, admin: Address, token: Address, threshold: i128) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::multisig_withdrawal::set_multisig_threshold(&env, &admin, &token, threshold);
    }

    pub fn get_multisig_threshold(env: Env, token: Address) -> i128 {
        crate::from_shade::multisig_withdrawal::get_multisig_threshold(&env, &token)
            .unwrap_or_else(|| panic_with_error!(&env, MultiSigError::ThresholdNotSet))
    }

    pub fn configure_multisig(env: Env, admin: Address, signers: Vec<Address>, quorum: u32) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::multisig_withdrawal::configure_multisig(&env, &admin, signers, quorum);
    }

    pub fn propose_withdrawal(
        env: Env,
        merchant: Address,
        token: Address,
        amount: i128,
        recipient: Address,
        note: String,
    ) -> u64 {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::multisig_withdrawal::propose_withdrawal(&env, &merchant, &token, amount, &recipient, note)
    }

    pub fn approve_withdrawal(env: Env, signer: Address, proposal_id: u64) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::multisig_withdrawal::approve_withdrawal(&env, &signer, proposal_id);
    }

    pub fn cancel_withdrawal(env: Env, caller: Address, proposal_id: u64) {
        shade::components::pausable::assert_not_paused(&env);
        crate::from_shade::multisig_withdrawal::cancel_withdrawal(&env, &caller, proposal_id);
    }

    pub fn get_withdrawal_proposal(env: Env, proposal_id: u64) -> WithdrawalProposal {
        crate::from_shade::multisig_withdrawal::get_withdrawal_proposal(&env, proposal_id)
    }

    pub fn has_approved_withdrawal(env: Env, signer: Address, proposal_id: u64) -> bool {
        crate::from_shade::multisig_withdrawal::has_approved(&env, &signer, proposal_id)
    }

    pub fn get_withdrawal_proposal_count(env: Env) -> u64 {
        crate::from_shade::multisig_withdrawal::get_proposal_count(&env)
    }

    pub fn search_withdrawal_proposals(
        env: Env,
        caller: Address,
        filter: WithdrawalProposalFilter,
    ) -> Vec<WithdrawalProposal> {
        crate::from_shade::search::search_withdrawal_proposals(&env, &caller, filter)
    }
}
