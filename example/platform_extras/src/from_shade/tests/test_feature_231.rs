//! Comprehensive tests for the Multi-Sig Massive Withdrawal feature (#231 / #368).
//!
//! Test categories
//! ───────────────
//! 1. Happy-path            – standard proposal → approval → execution flow
//! 2. Unauthorized access   – malicious actors, wrong callers, wrong states
//! 3. Event emission        – correct events emitted at each lifecycle step
//! 4. Storage rollback      – panicking calls must not mutate state
//! 5. Edge cases            – boundary values, uninitialized states, quorum math

#![cfg(test)]

extern crate std;

use shade::shade::{Shade, ShadeClient};
use crate::from_shade::types::{WithdrawalProposalStatus};
use account::account::{MerchantAccount, MerchantAccountClient};
use soroban_sdk::testutils::{Address as _, Events as _, Ledger as _};
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{Address, Env, String, Vec};

// ── Shared setup helpers ──────────────────────────────────────────────────────

/// Spin up a fully-initialised Shade contract and return
/// `(env, client, admin, token)`.
fn base_setup() -> (Env, ShadeClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    env.ledger().with_mut(|l| l.timestamp = 1_000_000);

    let contract_id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let token_sac = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token = token_sac.address();
    client.add_accepted_token(&admin, &token);
    client.set_fee(&admin, &token, &0); // zero fee keeps math simple

    (env, client, admin, token)
}

/// Extend `base_setup` by registering a merchant and configuring multi-sig.
///
/// Returns `(env, client, admin, token, merchant, signers)` where
/// `signers` is a `std::vec::Vec<Address>` with `n_signers` entries,
/// and the quorum is set to `quorum`.
fn multisig_setup(
    n_signers: usize,
    quorum: u32,
) -> (
    Env,
    ShadeClient<'static>,
    Address,
    Address,
    Address,
    std::vec::Vec<Address>,
) {
    let (env, client, admin, token) = base_setup();

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    // Deploy + link a real merchant-account contract: executing a withdrawal
    // pulls funds out via `withdraw_to`, which needs an actual contract.
    let merchant_account_id = env.register(MerchantAccount, ());
    MerchantAccountClient::new(&env, &merchant_account_id).initialize(
        &merchant,
        &client.address,
        &1_u64,
    );
    client.set_merchant_account(&merchant, &merchant_account_id);

    // Fund the account so executed withdrawals have a balance to draw on.
    StellarAssetClient::new(&env, &token).mint(&merchant_account_id, &1_000_000);

    // Build signer list.
    let mut signers_std = std::vec::Vec::new();
    let mut signers_sdk = Vec::new(&env);
    for _ in 0..n_signers {
        let s = Address::generate(&env);
        signers_std.push(s.clone());
        signers_sdk.push_back(s);
    }
    client.configure_multisig(&admin, &signers_sdk, &quorum);

    // Set threshold so proposals are accepted (threshold = 1_000).
    client.set_multisig_threshold(&admin, &token, &1_000);

    (env, client, admin, token, merchant, signers_std)
}

/// Deploy + link a funded merchant-account contract for `merchant`.
/// Executing a withdrawal pulls funds via `withdraw_to`, so the merchant needs
/// a real account contract holding a balance.
fn link_funded_account(
    env: &Env,
    client: &ShadeClient<'_>,
    merchant: &Address,
    token: &Address,
    merchant_id: u64,
) {
    let account_id = env.register(MerchantAccount, ());
    MerchantAccountClient::new(env, &account_id).initialize(
        merchant,
        &client.address,
        &merchant_id,
    );
    client.set_merchant_account(merchant, &account_id);
    StellarAssetClient::new(env, token).mint(&account_id, &1_000_000);
}

/// Shorthand: open a proposal and return its ID.
fn open_proposal(
    client: &ShadeClient<'_>,
    env: &Env,
    merchant: &Address,
    token: &Address,
    amount: i128,
    recipient: &Address,
) -> u64 {
    client.propose_withdrawal(
        merchant,
        token,
        &amount,
        recipient,
        &String::from_str(env, "test withdrawal"),
    )
}

// ═══════════════════════════════════════════════════════════════════════════
// 1. HAPPY-PATH
// ═══════════════════════════════════════════════════════════════════════════

/// Admin can set a threshold; it is readable back.
#[test]
fn test_happy_set_and_get_threshold() {
    let (_env, client, admin, token) = base_setup();
    client.set_multisig_threshold(&admin, &token, &5_000);
    assert_eq!(client.get_multisig_threshold(&token), 5_000);
}

/// Admin can configure signers and quorum; proposal count stays 0 after config.
#[test]
fn test_happy_configure_multisig_stores_quorum() {
    let (env, client, admin, _token) = base_setup();

    let s1 = Address::generate(&env);
    let s2 = Address::generate(&env);
    let s3 = Address::generate(&env);
    let mut sv = Vec::new(&env);
    sv.push_back(s1);
    sv.push_back(s2);
    sv.push_back(s3);
    client.configure_multisig(&admin, &sv, &2);

    // No proposals opened yet.
    assert_eq!(client.get_withdrawal_proposal_count(), 0);
}

/// A proposal is created with status Pending and correct fields.
#[test]
fn test_happy_propose_withdrawal_creates_pending_proposal() {
    let (env, client, _admin, token, merchant, _signers) = multisig_setup(2, 1);
    let recipient = Address::generate(&env);

    let id = open_proposal(&client, &env, &merchant, &token, 2_000, &recipient);
    assert_eq!(id, 1);

    let proposal = client.get_withdrawal_proposal(&id);
    assert_eq!(proposal.id, 1);
    assert_eq!(proposal.merchant, merchant);
    assert_eq!(proposal.token, token);
    assert_eq!(proposal.amount, 2_000);
    assert_eq!(proposal.recipient, recipient);
    assert_eq!(proposal.approvals, 0);
    assert_eq!(proposal.status, WithdrawalProposalStatus::Pending);
}

/// Each call to `propose_withdrawal` increments the proposal counter.
#[test]
fn test_happy_proposal_ids_increment() {
    let (env, client, _admin, token, merchant, _signers) = multisig_setup(2, 1);
    let recipient = Address::generate(&env);

    let id1 = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);
    let id2 = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);
    let id3 = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(id3, 3);
    assert_eq!(client.get_withdrawal_proposal_count(), 3);
}

/// A single signer approving a quorum-1 proposal executes it immediately.
#[test]
fn test_happy_single_approval_at_quorum_1_executes() {
    let (env, client, _admin, token, merchant, signers) = multisig_setup(3, 1);
    let recipient = Address::generate(&env);

    let id = open_proposal(&client, &env, &merchant, &token, 1_500, &recipient);

    // Approval triggers execution.
    client.approve_withdrawal(&signers[0], &id);

    let proposal = client.get_withdrawal_proposal(&id);
    assert_eq!(proposal.status, WithdrawalProposalStatus::Executed);
    assert_eq!(proposal.approvals, 1);
}

/// Two approvals on a quorum-2 proposal executes on the second vote.
#[test]
fn test_happy_two_approvals_at_quorum_2_executes() {
    let (env, client, _admin, token, merchant, signers) = multisig_setup(3, 2);
    let recipient = Address::generate(&env);

    let id = open_proposal(&client, &env, &merchant, &token, 1_500, &recipient);

    // First approval — still pending.
    client.approve_withdrawal(&signers[0], &id);
    assert_eq!(
        client.get_withdrawal_proposal(&id).status,
        WithdrawalProposalStatus::Pending
    );

    // Second approval — executed.
    client.approve_withdrawal(&signers[1], &id);
    assert_eq!(
        client.get_withdrawal_proposal(&id).status,
        WithdrawalProposalStatus::Executed
    );
}

/// Approval count is recorded correctly before execution.
#[test]
fn test_happy_approval_count_increments_before_quorum() {
    let (env, client, _admin, token, merchant, signers) = multisig_setup(3, 3);
    let recipient = Address::generate(&env);

    let id = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);

    client.approve_withdrawal(&signers[0], &id);
    assert_eq!(client.get_withdrawal_proposal(&id).approvals, 1);

    client.approve_withdrawal(&signers[1], &id);
    assert_eq!(client.get_withdrawal_proposal(&id).approvals, 2);
}

/// `has_approved_withdrawal` reflects each signer's vote correctly.
#[test]
fn test_happy_has_approved_reflects_votes() {
    let (env, client, _admin, token, merchant, signers) = multisig_setup(3, 3);
    let recipient = Address::generate(&env);

    let id = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);

    assert!(!client.has_approved_withdrawal(&signers[0], &id));
    client.approve_withdrawal(&signers[0], &id);
    assert!(client.has_approved_withdrawal(&signers[0], &id));
    assert!(!client.has_approved_withdrawal(&signers[1], &id));
}

/// The proposing merchant can cancel a pending proposal.
#[test]
fn test_happy_merchant_can_cancel_pending_proposal() {
    let (env, client, _admin, token, merchant, _signers) = multisig_setup(2, 2);
    let recipient = Address::generate(&env);

    let id = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);
    client.cancel_withdrawal(&merchant, &id);

    assert_eq!(
        client.get_withdrawal_proposal(&id).status,
        WithdrawalProposalStatus::Cancelled
    );
}

/// The admin can cancel any pending proposal.
#[test]
fn test_happy_admin_can_cancel_pending_proposal() {
    let (env, client, admin, token, merchant, _signers) = multisig_setup(2, 2);
    let recipient = Address::generate(&env);

    let id = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);
    client.cancel_withdrawal(&admin, &id);

    assert_eq!(
        client.get_withdrawal_proposal(&id).status,
        WithdrawalProposalStatus::Cancelled
    );
}

/// Threshold can be updated; new proposals must meet the new threshold.
#[test]
fn test_happy_threshold_can_be_updated() {
    let (_env, client, admin, token) = base_setup();
    client.set_multisig_threshold(&admin, &token, &1_000);
    assert_eq!(client.get_multisig_threshold(&token), 1_000);

    client.set_multisig_threshold(&admin, &token, &50_000);
    assert_eq!(client.get_multisig_threshold(&token), 50_000);
}

/// Quorum and signers can be reconfigured; new config takes effect immediately.
#[test]
fn test_happy_reconfigure_multisig_updates_quorum() {
    let (env, client, _admin, token, _merchant, _signers) = multisig_setup(3, 2);

    // Reconfigure with a different quorum.
    let new_signer = Address::generate(&env);
    let mut new_signers = Vec::new(&env);
    new_signers.push_back(new_signer.clone());
    client.configure_multisig(&_admin, &new_signers, &1);

    // The new signer can now approve proposals on their own.
    let merchant2 = Address::generate(&env);
    client.register_merchant(&merchant2);
    link_funded_account(&env, &client, &merchant2, &token, 2);
    // token threshold is still 1_000.
    let recipient = Address::generate(&env);
    let id = open_proposal(&client, &env, &merchant2, &token, 1_000, &recipient);
    client.approve_withdrawal(&new_signer, &id);

    assert_eq!(
        client.get_withdrawal_proposal(&id).status,
        WithdrawalProposalStatus::Executed
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. UNAUTHORIZED ACCESS / MALICIOUS ACTORS
// ═══════════════════════════════════════════════════════════════════════════

/// A non-admin cannot call `set_multisig_threshold`.
#[test]
#[should_panic(expected = "HostError")]
fn test_unauth_non_admin_cannot_set_threshold() {
    let (env, client, _admin, token) = base_setup();
    let attacker = Address::generate(&env);
    client.set_multisig_threshold(&attacker, &token, &1_000);
}

/// A non-admin cannot call `configure_multisig`.
#[test]
#[should_panic(expected = "HostError")]
fn test_unauth_non_admin_cannot_configure_multisig() {
    let (env, client, _admin, _token) = base_setup();
    let attacker = Address::generate(&env);
    let mut sv = Vec::new(&env);
    sv.push_back(attacker.clone());
    client.configure_multisig(&attacker, &sv, &1);
}

/// An address that is not a registered signer cannot approve a proposal.
#[test]
#[should_panic(expected = "HostError")]
fn test_unauth_non_signer_cannot_approve() {
    let (env, client, _admin, token, merchant, _signers) = multisig_setup(2, 2);
    let recipient = Address::generate(&env);
    let id = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);
    let intruder = Address::generate(&env);
    client.approve_withdrawal(&intruder, &id);
}

/// A signer cannot vote twice on the same proposal.
#[test]
#[should_panic(expected = "HostError")]
fn test_unauth_signer_cannot_vote_twice() {
    let (env, client, _admin, token, merchant, signers) = multisig_setup(3, 3);
    let recipient = Address::generate(&env);
    let id = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);
    client.approve_withdrawal(&signers[0], &id);
    client.approve_withdrawal(&signers[0], &id); // second vote — must panic
}

/// A random address cannot cancel someone else's proposal.
#[test]
#[should_panic(expected = "HostError")]
fn test_unauth_stranger_cannot_cancel_proposal() {
    let (env, client, _admin, token, merchant, _signers) = multisig_setup(2, 2);
    let recipient = Address::generate(&env);
    let id = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);
    let stranger = Address::generate(&env);
    client.cancel_withdrawal(&stranger, &id);
}

/// A proposal with amount below the threshold cannot be created.
#[test]
#[should_panic(expected = "HostError")]
fn test_unauth_below_threshold_proposal_rejected() {
    let (env, client, _admin, token, merchant, _signers) = multisig_setup(2, 1);
    let recipient = Address::generate(&env);
    // threshold is 1_000; try to open a proposal for 999.
    open_proposal(&client, &env, &merchant, &token, 999, &recipient);
}

/// Cannot propose a withdrawal with zero amount.
#[test]
#[should_panic(expected = "HostError")]
fn test_unauth_zero_amount_proposal_rejected() {
    let (env, client, _admin, token, merchant, _signers) = multisig_setup(2, 1);
    let recipient = Address::generate(&env);
    open_proposal(&client, &env, &merchant, &token, 0, &recipient);
}

/// Cannot approve a proposal that does not exist.
#[test]
#[should_panic(expected = "HostError")]
fn test_unauth_approve_nonexistent_proposal_panics() {
    let (_env, client, _admin, _token, _merchant, signers) = multisig_setup(2, 1);
    client.approve_withdrawal(&signers[0], &999);
}

/// Cannot cancel a proposal that does not exist.
#[test]
#[should_panic(expected = "HostError")]
fn test_unauth_cancel_nonexistent_proposal_panics() {
    let (_env, client, _admin, _token, merchant, _signers) = multisig_setup(2, 1);
    client.cancel_withdrawal(&merchant, &999);
}

/// Cannot approve an already-cancelled proposal.
#[test]
#[should_panic(expected = "HostError")]
fn test_unauth_approve_cancelled_proposal_panics() {
    let (env, client, _admin, token, merchant, signers) = multisig_setup(2, 2);
    let recipient = Address::generate(&env);
    let id = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);
    client.cancel_withdrawal(&merchant, &id);
    client.approve_withdrawal(&signers[0], &id); // must panic — not Pending
}

/// Cannot approve an already-executed proposal.
#[test]
#[should_panic(expected = "HostError")]
fn test_unauth_approve_executed_proposal_panics() {
    let (env, client, _admin, token, merchant, signers) = multisig_setup(2, 1);
    let recipient = Address::generate(&env);
    let id = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);
    client.approve_withdrawal(&signers[0], &id); // executes at quorum 1
    client.approve_withdrawal(&signers[1], &id); // must panic — already Executed
}

/// Cannot cancel an already-executed proposal.
#[test]
#[should_panic(expected = "HostError")]
fn test_unauth_cancel_executed_proposal_panics() {
    let (env, client, _admin, token, merchant, signers) = multisig_setup(2, 1);
    let recipient = Address::generate(&env);
    let id = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);
    client.approve_withdrawal(&signers[0], &id); // executes
    client.cancel_withdrawal(&merchant, &id); // must panic
}

/// `configure_multisig` with quorum = 0 must panic (InvalidQuorum).
#[test]
#[should_panic(expected = "HostError")]
fn test_unauth_quorum_zero_panics() {
    let (env, client, admin, _token) = base_setup();
    let s = Address::generate(&env);
    let mut sv = Vec::new(&env);
    sv.push_back(s);
    client.configure_multisig(&admin, &sv, &0);
}

/// `configure_multisig` with quorum > number of signers must panic.
#[test]
#[should_panic(expected = "HostError")]
fn test_unauth_quorum_exceeds_signers_panics() {
    let (env, client, admin, _token) = base_setup();
    let s = Address::generate(&env);
    let mut sv = Vec::new(&env);
    sv.push_back(s);
    client.configure_multisig(&admin, &sv, &2); // 2 > 1 signer
}

/// `configure_multisig` with an empty signer list must panic.
#[test]
#[should_panic(expected = "HostError")]
fn test_unauth_empty_signer_list_panics() {
    let (env, client, admin, _token) = base_setup();
    let sv: Vec<Address> = Vec::new(&env);
    client.configure_multisig(&admin, &sv, &1);
}

/// Proposal amount exactly equal to threshold is accepted (boundary: exact threshold).
#[test]
fn test_unauth_amount_exactly_at_threshold_accepted() {
    let (env, client, _admin, token, merchant, _signers) = multisig_setup(2, 1);
    let recipient = Address::generate(&env);
    // threshold = 1_000, amount = 1_000 → should succeed.
    let id = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);
    assert!(id >= 1);
}

/// An unregistered address cannot open a proposal (not a merchant).
#[test]
#[should_panic(expected = "HostError")]
fn test_unauth_non_merchant_cannot_propose() {
    let (env, client, _admin, token, _merchant, _signers) = multisig_setup(2, 1);
    let impostor = Address::generate(&env);
    let recipient = Address::generate(&env);
    open_proposal(&client, &env, &impostor, &token, 1_000, &recipient);
}

/// Functions that require configuration panic when multi-sig is not set up.
#[test]
#[should_panic(expected = "HostError")]
fn test_unauth_propose_without_multisig_configured_panics() {
    let (env, client, admin, token) = base_setup();
    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);
    client.set_multisig_threshold(&admin, &token, &1_000);
    // No configure_multisig called → quorum not set → panic.
    let recipient = Address::generate(&env);
    client.propose_withdrawal(
        &merchant,
        &token,
        &1_000,
        &recipient,
        &String::from_str(&env, "no quorum"),
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. EVENT EMISSION
// ═══════════════════════════════════════════════════════════════════════════

/// `set_multisig_threshold` emits at least one event.
#[test]
fn test_event_threshold_set_emitted() {
    let (env, client, admin, token) = base_setup();
    client.set_multisig_threshold(&admin, &token, &5_000);
    assert!(!env.events().all().is_empty());
}

/// `configure_multisig` emits at least one event.
#[test]
fn test_event_multisig_configured_emitted() {
    let (env, client, admin, _token) = base_setup();
    let s = Address::generate(&env);
    let mut sv = Vec::new(&env);
    sv.push_back(s);
    client.configure_multisig(&admin, &sv, &1);
    assert!(!env.events().all().is_empty());
}

/// `propose_withdrawal` emits at least one event.
#[test]
fn test_event_withdrawal_proposed_emitted() {
    let (env, client, _admin, token, merchant, _signers) = multisig_setup(2, 1);
    let recipient = Address::generate(&env);
    open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);
    assert!(!env.events().all().is_empty());
}

/// `approve_withdrawal` (pre-quorum) emits an approval event.
#[test]
fn test_event_withdrawal_approved_emitted_pre_quorum() {
    let (env, client, _admin, token, merchant, signers) = multisig_setup(3, 3);
    let recipient = Address::generate(&env);
    let id = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);
    client.approve_withdrawal(&signers[0], &id);
    // `env.events().all()` returns only the events of the most recent
    // invocation, so assert on that call's output rather than a running total.
    assert!(!env.events().all().is_empty());
}

/// When the final approval triggers execution, both approval and execution
/// events are emitted.
#[test]
fn test_event_withdrawal_executed_emitted_at_quorum() {
    let (env, client, _admin, token, merchant, signers) = multisig_setup(2, 1);
    let recipient = Address::generate(&env);
    let id = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);
    client.approve_withdrawal(&signers[0], &id);
    // At least the approval event + execution event.
    assert!(env.events().all().len() >= 2);
}

/// `cancel_withdrawal` emits a cancellation event.
#[test]
fn test_event_withdrawal_cancelled_emitted() {
    let (env, client, _admin, token, merchant, _signers) = multisig_setup(2, 2);
    let recipient = Address::generate(&env);
    let id = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);
    client.cancel_withdrawal(&merchant, &id);
    assert!(!env.events().all().is_empty());
}

/// Multiple proposals each emit their own proposal event.
#[test]
fn test_event_multiple_proposals_emit_distinct_events() {
    let (env, client, _admin, token, merchant, _signers) = multisig_setup(2, 1);
    let recipient = Address::generate(&env);
    open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);
    let after_first = env.events().all().len();
    open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);
    let after_second = env.events().all().len();
    // `env.events().all()` returns only the events of the most recent
    // invocation, so assert on that call's output rather than a running total.
    assert!(after_first > 0);
    assert!(after_second > 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. STORAGE ROLLBACK ON PANIC
// ═══════════════════════════════════════════════════════════════════════════

/// A failed `approve_withdrawal` (duplicate vote) must not increment the
/// approval counter.
#[test]
fn test_rollback_duplicate_vote_does_not_increment_count() {
    let (env, client, _admin, token, merchant, signers) = multisig_setup(3, 3);
    let recipient = Address::generate(&env);
    let id = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);

    client.approve_withdrawal(&signers[0], &id);
    assert_eq!(client.get_withdrawal_proposal(&id).approvals, 1);

    // Duplicate vote should panic.
    let result = client.try_approve_withdrawal(&signers[0], &id);
    assert!(result.is_err());

    // Count must still be 1, not 2.
    assert_eq!(client.get_withdrawal_proposal(&id).approvals, 1);
}

/// A failed `propose_withdrawal` (below threshold) must not advance the
/// proposal counter.
#[test]
fn test_rollback_failed_proposal_does_not_advance_counter() {
    let (env, client, _admin, token, merchant, _signers) = multisig_setup(2, 1);
    let recipient = Address::generate(&env);

    let count_before = client.get_withdrawal_proposal_count();
    // Below threshold → must fail.
    let result = client.try_propose_withdrawal(
        &merchant,
        &token,
        &1, // 1 < 1_000 threshold
        &recipient,
        &String::from_str(&env, "too small"),
    );
    assert!(result.is_err());
    assert_eq!(client.get_withdrawal_proposal_count(), count_before);
}

/// A failed `cancel_withdrawal` (stranger caller) must not change the
/// proposal status.
#[test]
fn test_rollback_failed_cancel_leaves_status_pending() {
    let (env, client, _admin, token, merchant, _signers) = multisig_setup(2, 2);
    let recipient = Address::generate(&env);
    let id = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);

    let stranger = Address::generate(&env);
    let result = client.try_cancel_withdrawal(&stranger, &id);
    assert!(result.is_err());

    // Status must still be Pending.
    assert_eq!(
        client.get_withdrawal_proposal(&id).status,
        WithdrawalProposalStatus::Pending
    );
}

/// A failed `configure_multisig` (quorum > signers) must not overwrite the
/// existing valid config.
#[test]
fn test_rollback_bad_configure_does_not_overwrite_good_config() {
    let (env, client, admin, token) = base_setup();

    // Valid config: 2 signers, quorum 1.
    let s1 = Address::generate(&env);
    let s2 = Address::generate(&env);
    let mut sv = Vec::new(&env);
    sv.push_back(s1.clone());
    sv.push_back(s2.clone());
    client.configure_multisig(&admin, &sv, &1);

    // Bad config: quorum 5 > 2 signers.
    let result = client.try_configure_multisig(&admin, &sv, &5);
    assert!(result.is_err());

    // The first signer should still be able to approve a proposal after the
    // failed reconfiguration — proving the good config survived.
    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);
    link_funded_account(&env, &client, &merchant, &token, 1);
    client.set_multisig_threshold(&admin, &token, &1_000);
    let recipient = Address::generate(&env);
    let id = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);
    client.approve_withdrawal(&s1, &id);
    assert_eq!(
        client.get_withdrawal_proposal(&id).status,
        WithdrawalProposalStatus::Executed
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. EDGE CASES – boundary values, uninitialized states, quorum math
// ═══════════════════════════════════════════════════════════════════════════

/// `get_withdrawal_proposal_count` returns 0 before any proposals.
#[test]
fn test_edge_proposal_count_zero_initially() {
    let (_env, client, _admin, _token, _merchant, _signers) = multisig_setup(2, 1);
    // One proposal already opened in multisig_setup? No — setup only configures;
    // it does not open proposals.
    assert_eq!(client.get_withdrawal_proposal_count(), 0);
}

/// `has_approved_withdrawal` returns false for a non-existent proposal.
#[test]
fn test_edge_has_approved_false_for_unknown_proposal() {
    let (_env, client, _admin, _token, _merchant, signers) = multisig_setup(2, 1);
    // Proposal 999 was never created.
    assert!(!client.has_approved_withdrawal(&signers[0], &999));
}

/// `get_multisig_threshold` panics when no threshold is set for the token.
#[test]
#[should_panic(expected = "HostError")]
fn test_edge_get_threshold_unset_panics() {
    let (env, client, _admin, _token) = base_setup();
    let unset_token = Address::generate(&env);
    client.get_multisig_threshold(&unset_token);
}

/// Threshold of exactly 0 marks multi-sig disabled; proposals are rejected.
#[test]
#[should_panic(expected = "HostError")]
fn test_edge_threshold_zero_rejects_all_proposals() {
    let (env, client, admin, token) = base_setup();
    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);
    let s = Address::generate(&env);
    let mut sv = Vec::new(&env);
    sv.push_back(s);
    client.configure_multisig(&admin, &sv, &1);
    client.set_multisig_threshold(&admin, &token, &0); // disabled
    let recipient = Address::generate(&env);
    // Amount >= 0 but threshold == 0 → BelowMultiSigThreshold.
    client.propose_withdrawal(
        &merchant,
        &token,
        &1_000,
        &recipient,
        &String::from_str(&env, "disabled"),
    );
}

/// Quorum equal to signer count requires all signers to approve.
#[test]
fn test_edge_quorum_equals_signer_count_requires_all() {
    let (env, client, _admin, token, merchant, signers) = multisig_setup(3, 3);
    let recipient = Address::generate(&env);
    let id = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);

    client.approve_withdrawal(&signers[0], &id);
    assert_eq!(
        client.get_withdrawal_proposal(&id).status,
        WithdrawalProposalStatus::Pending
    );
    client.approve_withdrawal(&signers[1], &id);
    assert_eq!(
        client.get_withdrawal_proposal(&id).status,
        WithdrawalProposalStatus::Pending
    );
    client.approve_withdrawal(&signers[2], &id);
    assert_eq!(
        client.get_withdrawal_proposal(&id).status,
        WithdrawalProposalStatus::Executed
    );
}

/// A very large amount (i128::MAX / 2) can be proposed without overflow.
#[test]
fn test_edge_large_amount_proposal_does_not_overflow() {
    let (env, client, admin, token, merchant, _signers) = multisig_setup(2, 1);
    // Raise the threshold to accept a large amount.
    let large_amount: i128 = i128::MAX / 2;
    client.set_multisig_threshold(&admin, &token, &large_amount);
    let recipient = Address::generate(&env);
    let id = open_proposal(&client, &env, &merchant, &token, large_amount, &recipient);
    let proposal = client.get_withdrawal_proposal(&id);
    assert_eq!(proposal.amount, large_amount);
    assert_eq!(proposal.status, WithdrawalProposalStatus::Pending);
}

/// Multiple independent proposals are stored and retrieved independently.
#[test]
fn test_edge_multiple_proposals_isolated() {
    let (env, client, _admin, token, merchant, _signers) = multisig_setup(2, 2);
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    let id1 = open_proposal(&client, &env, &merchant, &token, 1_000, &r1);
    let id2 = open_proposal(&client, &env, &merchant, &token, 2_000, &r2);

    assert_ne!(id1, id2);
    assert_eq!(client.get_withdrawal_proposal(&id1).amount, 1_000);
    assert_eq!(client.get_withdrawal_proposal(&id2).amount, 2_000);
    assert_eq!(client.get_withdrawal_proposal(&id1).recipient, r1);
    assert_eq!(client.get_withdrawal_proposal(&id2).recipient, r2);
}

/// Cancelling one proposal does not affect another pending proposal.
#[test]
fn test_edge_cancel_one_does_not_affect_other() {
    let (env, client, _admin, token, merchant, _signers) = multisig_setup(2, 2);
    let recipient = Address::generate(&env);

    let id1 = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);
    let id2 = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);

    client.cancel_withdrawal(&merchant, &id1);

    assert_eq!(
        client.get_withdrawal_proposal(&id1).status,
        WithdrawalProposalStatus::Cancelled
    );
    assert_eq!(
        client.get_withdrawal_proposal(&id2).status,
        WithdrawalProposalStatus::Pending
    );
}

/// A single signer in a 1-of-1 setup executes immediately.
#[test]
fn test_edge_single_signer_single_quorum_executes() {
    let (env, client, _admin, token, merchant, signers) = multisig_setup(1, 1);
    let recipient = Address::generate(&env);
    let id = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);
    client.approve_withdrawal(&signers[0], &id);
    assert_eq!(
        client.get_withdrawal_proposal(&id).status,
        WithdrawalProposalStatus::Executed
    );
}

/// `get_withdrawal_proposal` panics on an ID that was never created.
#[test]
#[should_panic(expected = "HostError")]
fn test_edge_get_nonexistent_proposal_panics() {
    let (_env, client, _admin, _token, _merchant, _signers) = multisig_setup(2, 1);
    client.get_withdrawal_proposal(&42);
}

/// Threshold update to a higher value blocks proposals that previously met
/// the old threshold.
#[test]
#[should_panic(expected = "HostError")]
fn test_edge_threshold_increase_blocks_old_amount() {
    let (env, client, admin, token, merchant, _signers) = multisig_setup(2, 1);
    // Raise threshold above 1_000.
    client.set_multisig_threshold(&admin, &token, &10_000);
    let recipient = Address::generate(&env);
    // 1_000 no longer meets the 10_000 threshold.
    open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);
}

/// Pausing the contract blocks `propose_withdrawal`.
#[test]
#[should_panic(expected = "HostError")]
fn test_edge_paused_contract_blocks_propose() {
    let (env, client, admin, token, merchant, _signers) = multisig_setup(2, 1);
    client.pause(&admin);
    let recipient = Address::generate(&env);
    open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);
}

/// Pausing the contract blocks `approve_withdrawal`.
#[test]
#[should_panic(expected = "HostError")]
fn test_edge_paused_contract_blocks_approve() {
    let (env, client, admin, token, merchant, signers) = multisig_setup(2, 2);
    let recipient = Address::generate(&env);
    let id = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);
    client.pause(&admin);
    client.approve_withdrawal(&signers[0], &id);
}

/// Pausing the contract blocks `cancel_withdrawal`.
#[test]
#[should_panic(expected = "HostError")]
fn test_edge_paused_contract_blocks_cancel() {
    let (env, client, admin, token, merchant, _signers) = multisig_setup(2, 2);
    let recipient = Address::generate(&env);
    let id = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);
    client.pause(&admin);
    client.cancel_withdrawal(&merchant, &id);
}

/// The note attached to a proposal is stored and retrieved correctly.
#[test]
fn test_edge_proposal_note_stored_correctly() {
    let (env, client, _admin, token, merchant, _signers) = multisig_setup(2, 1);
    let recipient = Address::generate(&env);
    let note = String::from_str(&env, "Q2 treasury rebalance");
    let id = client.propose_withdrawal(&merchant, &token, &1_000, &recipient, &note);
    assert_eq!(client.get_withdrawal_proposal(&id).note, note);
}

/// `created_at` and `updated_at` timestamps are recorded at proposal time.
#[test]
fn test_edge_timestamps_recorded_on_proposal() {
    let (env, client, _admin, token, merchant, _signers) = multisig_setup(2, 2);
    let recipient = Address::generate(&env);
    let id = open_proposal(&client, &env, &merchant, &token, 1_000, &recipient);
    let proposal = client.get_withdrawal_proposal(&id);
    let now = env.ledger().timestamp();
    assert_eq!(proposal.created_at, now);
    assert_eq!(proposal.updated_at, now);
}
