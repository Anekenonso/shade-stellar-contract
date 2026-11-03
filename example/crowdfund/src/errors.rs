use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum CrowdfundError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    InvalidGoal = 3,
    InvalidDeadline = 4,
    InvalidAmount = 5,
    CampaignEnded = 6,
    // Campaign has not yet passed its deadline.
    CampaignNotEnded = 7,
    // Goal was not reached; organizer cannot withdraw.
    GoalNotReached = 8,
    // Goal was reached; refunds are not available.
    GoalReached = 9,
    // Contributor has no recorded pledge to refund.
    NoPledge = 10,
    // Funds have already been withdrawn by the organizer.
    AlreadyExecuted = 11,
    // Reward for this backer has already been marked fulfilled.
    AlreadyFulfilled = 12,
    // Contributor's total pledge is below the selected tier's minimum.
    PledgeBelowTierMinimum = 13,
    // The supplied tier index does not exist.
    InvalidTier = 14,
    // No milestones have been set on this campaign.
    MilestonesNotSet = 15,
    // This milestone has already been released.
    MilestoneAlreadyReleased = 16,
    // This milestone has not yet been unlocked by the organizer.
    MilestoneNotUnlocked = 17,
    // Milestone percentages must be non-zero, and sum to exactly 10 000 bps (100 %).
    InvalidMilestonePercentages = 18,
    // Campaign is in milestone mode; use release_milestone instead of execute_campaign.
    MilestonesActive = 19,
    // Milestone release does not have a strict majority approval from backers.
    MilestoneNotApproved = 20,
    // Only contributors with a recorded pledge can vote on milestone release.
    NotBacker = 21,
    // A backer can only vote once per milestone.
    MilestoneVoteAlreadyCast = 22,
    // Shade payment gateway address has not been configured.
    ShadeGatewayNotSet = 23,
    // Merchant account address has not been configured.
    MerchantAccountNotSet = 24,
    // Batch refund has already been processed.
    RefundAlreadyProcessed = 25,
    // Sponsor matching pool cannot satisfy a match request.
    InsufficientMatchingPool = 26,
    // Pledge comment exceeds the configured maximum length.
    CommentTooLong = 27,
    // The caller is not registered as an affiliate for this campaign.
    AffiliateNotRegistered = 28,
    // This referral code has already been registered by another affiliate.
    ReferralCodeAlreadyTaken = 29,
    // Commission basis points must be in the range 0–10_000 (0–100%).
    InvalidCommissionBps = 30,
    // No affiliate is registered with the supplied referral code.
    ReferralCodeNotFound = 31,
    // This contributor has already used a referral code for this campaign.
    ReferralAlreadyUsed = 32,
    // Caller is not authorized for this privileged view (organizer only).
    NotAuthorized = 33,
    // The backer already holds this badge.
    BadgeAlreadyAwarded = 34,
    // The backer does not meet this badge's on-chain eligibility rules.
    BadgeNotEligible = 35,
    // Badge eligibility thresholds have not been configured by the organizer.
    BadgeConfigNotSet = 36,
    // ── Social recovery / guardians ─────────────────────────────────────────
    // No guardian set has been configured for this campaign.
    GuardiansNotSet = 37,
    // The same address appears twice in the supplied guardian list.
    DuplicateGuardian = 38,
    // Threshold must be non-zero and no greater than the number of guardians.
    InvalidThreshold = 39,
    // A recovery is already pending; cancel it before starting another.
    RecoveryAlreadyPending = 40,
    // No recovery is currently pending.
    NoPendingRecovery = 41,
    // The caller is not in the registered guardian set.
    NotGuardian = 42,
    // This guardian has already approved the pending recovery.
    AlreadyApprovedRecovery = 43,
    // ── KYC gating ──────────────────────────────────────────────────────────
    // The campaign requires KYC and the contributor is not verified.
    KYCRequired = 44,
}
