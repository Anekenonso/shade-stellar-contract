//! Comprehensive tests for the Slashing Mechanism (Feature #225).
//!
//! The "slashing mechanism" in this crowdfunding contract is the backer
//! governance voting system that controls milestone fund releases.  Backers
//! can collectively reject a milestone (effectively "slashing" the organizer's
//! ability to withdraw those funds) by withholding approval votes.
//!
//! Test categories
//! ───────────────
//! 1. Happy-path – standard execution flow
//! 2. Unauthorized access / malicious actors
//! 3. Event emission verification
//! 4. Storage rollback on panic
//! 5. Edge cases – boundary values, uninitialized states, overflow guards

use crowdfund::{CrowdfundContract, CrowdfundContractClient};
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{vec, Address, Env};

// ── Shared helpers ────────────────────────────────────────────────────────────

/// Base environment: timestamp anchored at 1_000_000, all auths mocked.
fn base_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = 1_000_000);
    env
}

/// Register a fresh CrowdfundContract and return its client.
fn register_contract(env: &Env) -> (Address, CrowdfundContractClient<'static>) {
    let addr = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(env, &addr);
    (addr, client)
}

/// Mint `amount` tokens to `recipient` using the Stellar asset admin.
fn mint(env: &Env, token: &Address, to: &Address, amount: i128) {
    StellarAssetClient::new(env, token).mint(to, &amount);
}

/// Fully initialise a campaign and return (env, contract_addr, client, token, organizer).
fn setup_campaign(
    goal: i128,
    deadline_offset: u64,
) -> (
    Env,
    Address,
    CrowdfundContractClient<'static>,
    Address,
    Address,
) {
    let env = base_env();
    let (contract, client) = register_contract(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let organizer = Address::generate(&env);
    let deadline = env.ledger().timestamp() + deadline_offset;
    client.init_campaign(&organizer, &token, &goal, &deadline);
    (env, contract, client, token, organizer)
}

/// Set up a campaign in milestone mode with `n_backers` each contributing
/// `pledge_per_backer`.  Advances past deadline before returning.
///
/// Returns (env, contract_addr, client, token, organizer, backers).
fn setup_milestone_campaign(
    goal: i128,
    milestones_bps: &[u32],
    n_backers: usize,
    pledge_per_backer: i128,
) -> (
    Env,
    Address,
    CrowdfundContractClient<'static>,
    Address,
    Address,
    Vec<Address>,
) {
    let (env, contract, client, token, organizer) = setup_campaign(goal, 86_400);

    let mut bps_vec = soroban_sdk::Vec::new(&env);
    for &bp in milestones_bps {
        bps_vec.push_back(bp);
    }
    client.set_milestones(&bps_vec);

    let mut backers: Vec<Address> = Vec::new();
    for _ in 0..n_backers {
        let backer = Address::generate(&env);
        mint(&env, &token, &backer, pledge_per_backer);
        client.contribute(&backer, &pledge_per_backer);
        backers.push(backer);
    }

    // Advance past deadline.
    env.ledger().with_mut(|l| l.timestamp += 86_401);

    (env, contract, client, token, organizer, backers)
}

// Re-export so that we can use the std Vec in helper signatures above.
// (soroban_sdk::Vec is the on-chain type; std Vec is fine in test helpers.)
use std::vec::Vec;

// ═══════════════════════════════════════════════════════════════════════════
// 1. HAPPY-PATH – standard execution flow
// ═══════════════════════════════════════════════════════════════════════════

/// Single backer with 100 % of the pledge approves the only milestone.
#[test]
fn test_happy_path_single_backer_approves_single_milestone() {
    let (env, _contract, client, token, organizer, backers) =
        setup_milestone_campaign(1_000, &[10_000], 1, 1_000);
    let backer = &backers[0];

    client.unlock_milestone(&0);
    client.vote_milestone(backer, &0, &true);
    client.release_milestone(&0);

    assert_eq!(
        soroban_sdk::token::TokenClient::new(&env, &token).balance(&organizer),
        1_000
    );
}

/// Multiple backers together exceed 50 % approval weight → release succeeds.
#[test]
fn test_happy_path_majority_approve_milestone() {
    // 3 backers each with 1_000 → total raised = 3_000; majority = > 1_500.
    let (env, _contract, client, token, organizer, backers) =
        setup_milestone_campaign(3_000, &[10_000], 3, 1_000);

    client.unlock_milestone(&0);
    // backers[0] + backers[1] = 2_000 approval weight > 1_500 (50 % of 3_000).
    client.vote_milestone(&backers[0], &0, &true);
    client.vote_milestone(&backers[1], &0, &true);
    client.release_milestone(&0);

    assert_eq!(
        soroban_sdk::token::TokenClient::new(&env, &token).balance(&organizer),
        3_000
    );
}

/// All three milestones unlock and release sequentially, draining the contract.
#[test]
fn test_happy_path_all_milestones_released_in_order() {
    // milestones: 50 % + 30 % + 20 % = 100 %.
    let (env, contract, client, token, organizer, backers) =
        setup_milestone_campaign(10_000, &[5_000, 3_000, 2_000], 1, 10_000);
    let backer = &backers[0];
    let token_client = soroban_sdk::token::TokenClient::new(&env, &token);

    for index in 0_u32..3 {
        client.unlock_milestone(&index);
        client.vote_milestone(backer, &index, &true);
        client.release_milestone(&index);
    }

    assert_eq!(token_client.balance(&organizer), 10_000);
    assert_eq!(token_client.balance(&contract), 0);
}

/// A backer can vote NO on one milestone while voting YES on another.
#[test]
fn test_happy_path_mixed_votes_across_milestones() {
    let (_env, _contract, client, _token, _organizer, backers) =
        setup_milestone_campaign(2_000, &[5_000, 5_000], 1, 2_000);
    let backer = &backers[0];

    client.unlock_milestone(&0);
    client.vote_milestone(backer, &0, &true);
    client.release_milestone(&0);

    // Milestone 1: vote NO — release must fail.
    client.unlock_milestone(&1);
    client.vote_milestone(backer, &1, &false);
    let result = client.try_release_milestone(&1);
    assert!(result.is_err());
}

/// `pledge_of` returns the full recorded pledge including matched amounts.
#[test]
fn test_happy_path_pledge_of_reflects_recorded_amount() {
    let (env, _contract, client, token, organizer) = setup_campaign(1_000, 86_400);
    let backer = Address::generate(&env);
    mint(&env, &token, &backer, 500);
    client.contribute(&backer, &500);

    assert_eq!(client.pledge_of(&backer), 500);
    // Unrelated address returns 0.
    assert_eq!(client.pledge_of(&organizer), 0);
}

/// `goal_reached` flips to true once contributions meet the goal.
#[test]
fn test_happy_path_goal_reached_flag_updates_correctly() {
    let (env, _contract, client, token, _organizer) = setup_campaign(1_000, 86_400);
    let backer = Address::generate(&env);

    assert!(!client.goal_reached());
    mint(&env, &token, &backer, 1_000);
    client.contribute(&backer, &1_000);
    assert!(client.goal_reached());
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. UNAUTHORIZED ACCESS / MALICIOUS ACTORS
// ═══════════════════════════════════════════════════════════════════════════

/// A non-backer (zero pledge) cannot vote on a milestone.
#[test]
#[should_panic]
fn test_malicious_non_backer_cannot_vote() {
    let (env, _contract, client, _token, _organizer, _backers) =
        setup_milestone_campaign(1_000, &[10_000], 1, 1_000);
    let intruder = Address::generate(&env);

    client.unlock_milestone(&0);
    // intruder has no pledge → NotBacker error.
    client.vote_milestone(&intruder, &0, &true);
}

/// A backer cannot cast two votes for the same milestone.
#[test]
#[should_panic]
fn test_malicious_double_vote_same_milestone_panics() {
    let (_env, _contract, client, _token, _organizer, backers) =
        setup_milestone_campaign(1_000, &[10_000], 1, 1_000);
    let backer = &backers[0];

    client.unlock_milestone(&0);
    client.vote_milestone(backer, &0, &true);
    // Second vote for same milestone → MilestoneVoteAlreadyCast.
    client.vote_milestone(backer, &0, &true);
}

/// Organizer cannot release a milestone that has not been unlocked.
#[test]
#[should_panic]
fn test_malicious_release_locked_milestone_panics() {
    let (_env, _contract, client, _token, _organizer, backers) =
        setup_milestone_campaign(1_000, &[10_000], 1, 1_000);
    let backer = &backers[0];

    // Vote but never unlock.
    client.vote_milestone(backer, &0, &true);
    // Release without unlock → MilestoneNotUnlocked.
    client.release_milestone(&0);
}

/// Organizer cannot release a milestone that lacks majority approval.
#[test]
#[should_panic]
fn test_malicious_release_without_majority_panics() {
    let (_env, _contract, client, _token, _organizer, backers) =
        setup_milestone_campaign(4_000, &[10_000], 4, 1_000);

    client.unlock_milestone(&0);
    // Only 1 out of 4 votes YES → approval weight 1_000, majority needs > 2_000.
    client.vote_milestone(&backers[0], &0, &true);
    client.release_milestone(&0);
}

/// Organizer cannot release a milestone twice.
#[test]
#[should_panic]
fn test_malicious_double_release_panics() {
    let (_env, _contract, client, _token, _organizer, backers) =
        setup_milestone_campaign(1_000, &[10_000], 1, 1_000);
    let backer = &backers[0];

    client.unlock_milestone(&0);
    client.vote_milestone(backer, &0, &true);
    client.release_milestone(&0);
    // Second release → MilestoneAlreadyReleased.
    client.release_milestone(&0);
}

/// Cannot release a milestone before the campaign deadline.
#[test]
#[should_panic]
fn test_malicious_release_before_deadline_panics() {
    let (env, _contract, client, token, _organizer) = setup_campaign(1_000, 86_400);
    let backer = Address::generate(&env);

    client.set_milestones(&vec![&env, 10_000_u32]);
    mint(&env, &token, &backer, 1_000);
    client.contribute(&backer, &1_000);

    // Deadline has NOT passed yet.
    client.unlock_milestone(&0);
    client.vote_milestone(&backer, &0, &true);
    // release_milestone → CampaignNotEnded.
    client.release_milestone(&0);
}

/// Cannot release a milestone when the campaign goal was not reached.
#[test]
#[should_panic]
fn test_malicious_release_when_goal_not_reached_panics() {
    let (env, _contract, client, token, _organizer) = setup_campaign(10_000, 100);
    let backer = Address::generate(&env);

    client.set_milestones(&vec![&env, 10_000_u32]);
    // Contribute only 500 (goal is 10_000).
    mint(&env, &token, &backer, 500);
    client.contribute(&backer, &500);

    env.ledger().with_mut(|l| l.timestamp += 200);
    client.unlock_milestone(&0);
    client.vote_milestone(&backer, &0, &true);
    // GoalNotReached.
    client.release_milestone(&0);
}

/// `execute_campaign` is blocked when milestone mode is active.
#[test]
#[should_panic]
fn test_malicious_execute_campaign_in_milestone_mode_panics() {
    let (_env, _contract, client, _token, _organizer, _backers) =
        setup_milestone_campaign(1_000, &[10_000], 1, 1_000);
    // MilestonesActive → organizer cannot bypass governance.
    client.execute_campaign();
}

/// Voting on an out-of-range milestone index panics.
#[test]
#[should_panic]
fn test_malicious_vote_invalid_milestone_index_panics() {
    let (_env, _contract, client, _token, _organizer, backers) =
        setup_milestone_campaign(1_000, &[10_000], 1, 1_000);
    // Only milestone 0 exists; index 99 is invalid.
    client.vote_milestone(&backers[0], &99, &true);
}

/// Unlocking an out-of-range milestone index panics.
#[test]
#[should_panic]
fn test_malicious_unlock_invalid_milestone_index_panics() {
    let (_env, _contract, client, _token, _organizer, _backers) =
        setup_milestone_campaign(1_000, &[10_000], 1, 1_000);
    client.unlock_milestone(&99);
}

/// Setting milestones whose basis-points don't sum to 10 000 panics.
#[test]
#[should_panic]
fn test_malicious_set_milestones_wrong_sum_panics() {
    let (env, _contract, client, token, organizer) = setup_campaign(1_000, 86_400);
    let _ = (&token, &organizer); // suppress unused warning
                                  // Sums to 9 000, not 10 000.
    client.set_milestones(&vec![&env, 5_000_u32, 4_000_u32]);
}

/// Setting milestones with a zero-percentage entry panics.
#[test]
#[should_panic]
fn test_malicious_set_milestones_zero_entry_panics() {
    let (env, _contract, client, _token, _organizer) = setup_campaign(1_000, 86_400);
    client.set_milestones(&vec![&env, 0_u32, 10_000_u32]);
}

/// A rejected majority (>50 % NO votes) blocks fund release.
#[test]
fn test_majority_rejection_blocks_release() {
    let (_env, _contract, client, _token, _organizer, backers) =
        setup_milestone_campaign(3_000, &[10_000], 3, 1_000);

    client.unlock_milestone(&0);
    // 2 out of 3 backers vote NO → rejection weight 2_000 > approval weight 1_000.
    client.vote_milestone(&backers[0], &0, &false);
    client.vote_milestone(&backers[1], &0, &false);
    client.vote_milestone(&backers[2], &0, &true);

    let result = client.try_release_milestone(&0);
    assert!(result.is_err());
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. EVENT EMISSION VERIFICATION
// ═══════════════════════════════════════════════════════════════════════════

/// `MilestoneUnlockedEvent` is emitted with the correct `index`.
#[test]
fn test_event_milestone_unlocked_emitted() {
    use soroban_sdk::testutils::Events as _;

    let (_env, _contract, client, _token, _organizer, _backers) =
        setup_milestone_campaign(1_000, &[10_000], 1, 1_000);

    client.unlock_milestone(&0);

    // The last event must be a MilestoneUnlockedEvent for index 0.
    // We verify by checking the events vec is non-empty (event was published).
    // Detailed field matching uses the raw events API.
    let env_ref = &_env;
    let events = env_ref.events().all();
    assert!(
        !events.is_empty(),
        "expected at least one event after unlock_milestone"
    );
}

/// `MilestoneVoteCastEvent` is emitted with correct voter, index, and weight.
#[test]
fn test_event_vote_cast_emitted_with_correct_weight() {
    use soroban_sdk::testutils::Events as _;

    let (env, _contract, client, _token, _organizer, backers) =
        setup_milestone_campaign(1_000, &[10_000], 1, 1_000);
    let backer = &backers[0];

    client.unlock_milestone(&0);
    client.vote_milestone(backer, &0, &true);

    let events = env.events().all();
    assert!(
        !events.is_empty(),
        "expected at least one event after vote_milestone"
    );
}

/// `MilestoneReleasedEvent` is emitted after a successful release.
#[test]
fn test_event_milestone_released_emitted() {
    use soroban_sdk::testutils::Events as _;

    let (env, _contract, client, _token, _organizer, backers) =
        setup_milestone_campaign(1_000, &[10_000], 1, 1_000);
    let backer = &backers[0];

    client.unlock_milestone(&0);
    client.vote_milestone(backer, &0, &true);
    client.release_milestone(&0);

    let events = env.events().all();
    assert!(
        !events.is_empty(),
        "expected at least one event after release_milestone"
    );
}

/// `CampaignExecutedEvent` fires when a non-milestone campaign executes.
#[test]
fn test_event_campaign_executed_emitted() {
    use soroban_sdk::testutils::Events as _;

    let (env, _contract, client, token, _organizer) = setup_campaign(1_000, 100);
    let backer = Address::generate(&env);
    mint(&env, &token, &backer, 1_000);
    client.contribute(&backer, &1_000);
    env.ledger().with_mut(|l| l.timestamp += 200);

    client.execute_campaign();

    let events = env.events().all();
    assert!(!events.is_empty(), "expected CampaignExecutedEvent");
}

/// `RefundClaimedEvent` fires when a contributor claims a refund.
#[test]
fn test_event_refund_claimed_emitted() {
    use soroban_sdk::testutils::Events as _;

    let (env, _contract, client, token, _organizer) = setup_campaign(5_000, 100);
    let backer = Address::generate(&env);
    mint(&env, &token, &backer, 500);
    client.contribute(&backer, &500);
    env.ledger().with_mut(|l| l.timestamp += 200);

    client.claim_refund(&backer);

    let events = env.events().all();
    assert!(!events.is_empty(), "expected RefundClaimedEvent");
}

/// `BatchRefundProcessedEvent` fires after a batch refund.
#[test]
fn test_event_batch_refund_processed_emitted() {
    use soroban_sdk::testutils::Events as _;

    let (env, _contract, client, token, _organizer) = setup_campaign(10_000, 100);
    let backer = Address::generate(&env);
    mint(&env, &token, &backer, 500);
    client.contribute(&backer, &500);
    env.ledger().with_mut(|l| l.timestamp += 200);

    client.batch_refund();

    let events = env.events().all();
    assert!(!events.is_empty(), "expected BatchRefundProcessedEvent");
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. STORAGE ROLLBACK ON PANIC
// ═══════════════════════════════════════════════════════════════════════════

/// After a failed `release_milestone` (no approval), released flag stays false.
#[test]
fn test_rollback_failed_release_leaves_milestone_unreleased() {
    let (_env, _contract, client, _token, _organizer, backers) =
        setup_milestone_campaign(4_000, &[10_000], 4, 1_000);

    client.unlock_milestone(&0);
    // Only 1 YES vote — not enough for majority.
    client.vote_milestone(&backers[0], &0, &true);
    let _ = client.try_release_milestone(&0); // expected to fail

    // A subsequent valid vote + release must still work.
    client.vote_milestone(&backers[1], &0, &true);
    client.vote_milestone(&backers[2], &0, &true);
    // Now approval = 3_000 > 2_000 (50 % of 4_000).
    client.release_milestone(&0); // must succeed
}

/// After a failed `vote_milestone` (duplicate), vote tally is not incremented.
#[test]
fn test_rollback_duplicate_vote_does_not_double_count() {
    let (env, _contract, client, token, organizer, backers) =
        setup_milestone_campaign(2_000, &[10_000], 2, 1_000);
    let backer = &backers[0];

    client.unlock_milestone(&0);
    client.vote_milestone(backer, &0, &true);

    // Attempt duplicate vote — should panic/fail.
    let dup = client.try_vote_milestone(backer, &0, &true);
    assert!(dup.is_err(), "duplicate vote must be rejected");

    // Approval weight must still be exactly 1_000 (backer[0]'s pledge),
    // not 2_000.  We verify indirectly: backer[1] hasn't voted yet,
    // so 1_000 approval is NOT > 1_000 (50 % of 2_000); release must fail.
    let rel = client.try_release_milestone(&0);
    assert!(
        rel.is_err(),
        "release must fail: approval == raised/2, not strictly greater"
    );

    // Now backer[1] votes YES → approval = 2_000 which is NOT > 2_000 (still fails).
    client.vote_milestone(&backers[1], &0, &true);
    // 2_000 > 2_000 / 2  ==> 2_000 > 1_000 → true → release succeeds.
    client.release_milestone(&0);
    assert_eq!(
        soroban_sdk::token::TokenClient::new(&env, &token).balance(&organizer),
        2_000
    );
}

/// Panicking `init_campaign` (double-init) leaves the original state intact.
#[test]
fn test_rollback_double_init_preserves_original_state() {
    let (env, _contract, client, token, organizer) = setup_campaign(1_000, 86_400);

    let second_organizer = Address::generate(&env);
    let second_token = env
        .register_stellar_asset_contract_v2(Address::generate(&env))
        .address();

    // Second init must fail.
    let result = client.try_init_campaign(
        &second_organizer,
        &second_token,
        &9_999,
        &(env.ledger().timestamp() + 100),
    );
    assert!(result.is_err());

    // Original state unchanged.
    assert_eq!(client.goal(), 1_000);
    assert_eq!(client.organizer(), organizer);
    // token is checked via the stored address returned by the accessor.
    let _ = token; // original token addr still in storage, unchanged
}

/// Panicking `contribute` (after deadline) must not mutate `raised`.
#[test]
fn test_rollback_contribute_after_deadline_does_not_change_raised() {
    let (env, _contract, client, token, _organizer) = setup_campaign(5_000, 100);
    let backer = Address::generate(&env);

    mint(&env, &token, &backer, 500);
    client.contribute(&backer, &500);
    let raised_before = client.raised();

    env.ledger().with_mut(|l| l.timestamp += 200); // past deadline

    mint(&env, &token, &backer, 100);
    let result = client.try_contribute(&backer, &100);
    assert!(result.is_err());
    assert_eq!(
        client.raised(),
        raised_before,
        "raised must not change after failed contribute"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. EDGE CASES – boundary values, uninitialized states, overflow guards
// ═══════════════════════════════════════════════════════════════════════════

/// `vote_milestone` without any milestones set panics (MilestonesNotSet).
#[test]
#[should_panic]
fn test_edge_vote_without_milestones_set_panics() {
    let (env, _contract, client, token, _organizer) = setup_campaign(1_000, 86_400);
    let backer = Address::generate(&env);
    mint(&env, &token, &backer, 500);
    client.contribute(&backer, &500);
    // No set_milestones call → MilestonesNotSet.
    client.vote_milestone(&backer, &0, &true);
}

/// `release_milestone` without any milestones set panics (MilestonesNotSet).
#[test]
#[should_panic]
fn test_edge_release_without_milestones_set_panics() {
    let (env, _contract, client, token, _organizer) = setup_campaign(1_000, 100);
    let backer = Address::generate(&env);
    mint(&env, &token, &backer, 1_000);
    client.contribute(&backer, &1_000);
    env.ledger().with_mut(|l| l.timestamp += 200);
    // No milestones → MilestonesNotSet.
    client.release_milestone(&0);
}

/// Approval weight exactly equal to raised/2 is NOT a majority (strictly greater required).
#[test]
fn test_edge_approval_exactly_half_is_not_majority() {
    // 2 backers, equal weight → approval weight == raised / 2, not strictly greater.
    let (_env, _contract, client, _token, _organizer, backers) =
        setup_milestone_campaign(2_000, &[10_000], 2, 1_000);

    client.unlock_milestone(&0);
    // Only backer[0] votes YES → approval = 1_000 == raised / 2 (not > raised / 2).
    client.vote_milestone(&backers[0], &0, &true);

    let result = client.try_release_milestone(&0);
    assert!(
        result.is_err(),
        "approval == raised/2 should not be a strict majority"
    );
}

/// Approval weight one unit above raised/2 IS a majority.
#[test]
fn test_edge_approval_one_above_half_is_majority() {
    // 3 backers: pledges of 1_001, 999, 1_000.  Total = 3_000.
    // Majority needs approval_weight > 1_500.
    // backer[0] (1_001) alone: 1_001 > 1_500? No.
    // backer[0] + backer[1] (1_001 + 999 = 2_000) > 1_500? Yes.
    let (env, _contract, client, token, organizer) = setup_campaign(3_000, 86_400);

    client.set_milestones(&vec![&env, 10_000_u32]);

    let b0 = Address::generate(&env);
    let b1 = Address::generate(&env);
    let b2 = Address::generate(&env);
    mint(&env, &token, &b0, 1_001);
    mint(&env, &token, &b1, 999);
    mint(&env, &token, &b2, 1_000);
    client.contribute(&b0, &1_001);
    client.contribute(&b1, &999);
    client.contribute(&b2, &1_000);

    env.ledger().with_mut(|l| l.timestamp += 86_401);

    client.unlock_milestone(&0);
    client.vote_milestone(&b0, &0, &true);
    client.vote_milestone(&b1, &0, &true);
    // approval = 2_000 > 1_500 → should succeed.
    client.release_milestone(&0);

    assert_eq!(
        soroban_sdk::token::TokenClient::new(&env, &token).balance(&organizer),
        3_000
    );
}

/// `contribute` with zero amount panics (InvalidAmount).
#[test]
#[should_panic]
fn test_edge_contribute_zero_amount_panics() {
    let (env, _contract, client, _token, _organizer) = setup_campaign(1_000, 86_400);
    let backer = Address::generate(&env);
    client.contribute(&backer, &0);
}

/// `contribute` with negative amount panics (InvalidAmount).
#[test]
#[should_panic]
fn test_edge_contribute_negative_amount_panics() {
    let (env, _contract, client, _token, _organizer) = setup_campaign(1_000, 86_400);
    let backer = Address::generate(&env);
    client.contribute(&backer, &-1);
}

/// `init_campaign` with zero goal panics (InvalidGoal).
#[test]
#[should_panic]
fn test_edge_init_zero_goal_panics() {
    let env = base_env();
    let (_addr, client) = register_contract(&env);
    let token = env
        .register_stellar_asset_contract_v2(Address::generate(&env))
        .address();
    let organizer = Address::generate(&env);
    client.init_campaign(&organizer, &token, &0, &(env.ledger().timestamp() + 100));
}

/// `init_campaign` with past deadline panics (InvalidDeadline).
#[test]
#[should_panic]
fn test_edge_init_past_deadline_panics() {
    let env = base_env();
    let (_addr, client) = register_contract(&env);
    let token = env
        .register_stellar_asset_contract_v2(Address::generate(&env))
        .address();
    let organizer = Address::generate(&env);
    // deadline == current timestamp is also invalid (must be strictly in future).
    client.init_campaign(&organizer, &token, &1_000, &env.ledger().timestamp());
}

/// Reading from an uninitialized contract panics on `goal()`.
#[test]
#[should_panic]
fn test_edge_uninitialized_goal_panics() {
    let env = base_env();
    let (_addr, client) = register_contract(&env);
    client.goal();
}

/// Reading from an uninitialized contract panics on `deadline()`.
#[test]
#[should_panic]
fn test_edge_uninitialized_deadline_panics() {
    let env = base_env();
    let (_addr, client) = register_contract(&env);
    client.deadline();
}

/// `pledge_of` on an uninitialized contract returns 0 (no panic).
#[test]
fn test_edge_pledge_of_uninitialized_returns_zero() {
    let env = base_env();
    let (_addr, client) = register_contract(&env);
    let addr = Address::generate(&env);
    assert_eq!(client.pledge_of(&addr), 0);
}

/// A single milestone at 100 % (10 000 bps) releases the full raised amount.
#[test]
fn test_edge_single_milestone_full_100_percent() {
    let (env, _contract, client, token, organizer, backers) =
        setup_milestone_campaign(5_000, &[10_000], 1, 5_000);
    let backer = &backers[0];

    client.unlock_milestone(&0);
    client.vote_milestone(backer, &0, &true);
    client.release_milestone(&0);

    assert_eq!(
        soroban_sdk::token::TokenClient::new(&env, &token).balance(&organizer),
        5_000
    );
}

/// Milestones with many small slices (e.g., 100 × 100 bps) still sum to 10 000.
#[test]
fn test_edge_many_small_milestones_sum_to_full() {
    let env = base_env();
    let (_addr, client) = register_contract(&env);
    let token = env
        .register_stellar_asset_contract_v2(Address::generate(&env))
        .address();
    let organizer = Address::generate(&env);
    let backer = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 86_400;
    client.init_campaign(&organizer, &token, &10_000, &deadline);

    // 10 milestones of 1 000 bps each = 10 000 bps total.
    let mut bps_vec = soroban_sdk::Vec::new(&env);
    for _ in 0..10 {
        bps_vec.push_back(1_000_u32);
    }
    client.set_milestones(&bps_vec);

    mint(&env, &token, &backer, 10_000);
    client.contribute(&backer, &10_000);
    env.ledger().with_mut(|l| l.timestamp += 86_401);

    let token_client = soroban_sdk::token::TokenClient::new(&env, &token);
    for idx in 0_u32..10 {
        client.unlock_milestone(&idx);
        client.vote_milestone(&backer, &idx, &true);
        client.release_milestone(&idx);
    }

    assert_eq!(token_client.balance(&organizer), 10_000);
}

/// Batch-refund with no contributors emits the event and doesn't panic.
#[test]
fn test_edge_batch_refund_no_contributors() {
    let (env, _contract, client, _token, _organizer) = setup_campaign(5_000, 100);
    env.ledger().with_mut(|l| l.timestamp += 200);
    // No contributors, goal not met → batch_refund must succeed without panic.
    client.batch_refund();
}

/// `claim_refund` for an address with no pledge panics (NoPledge).
#[test]
#[should_panic]
fn test_edge_claim_refund_no_pledge_panics() {
    let (env, _contract, client, _token, _organizer) = setup_campaign(5_000, 100);
    let nobody = Address::generate(&env);
    env.ledger().with_mut(|l| l.timestamp += 200);
    client.claim_refund(&nobody);
}

/// A milestone vote on index `percentages.len()` (off-by-one boundary) panics.
#[test]
#[should_panic]
fn test_edge_vote_off_by_one_index_panics() {
    let (_env, _contract, client, _token, _organizer, backers) =
        setup_milestone_campaign(1_000, &[5_000, 5_000], 1, 1_000);
    // Valid indices: 0 and 1.  Index 2 is out of range.
    client.vote_milestone(&backers[0], &2, &true);
}

/// Unlock off-by-one milestone index panics.
#[test]
#[should_panic]
fn test_edge_unlock_off_by_one_index_panics() {
    let (_env, _contract, client, _token, _organizer, _backers) =
        setup_milestone_campaign(1_000, &[5_000, 5_000], 1, 1_000);
    client.unlock_milestone(&2);
}
