use soroban_sdk::contracterror;

/// Error codes that moved out of the hub's `ContractError`.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ContractError {
    /// Generic "record does not exist" for lookups without a dedicated code.
    NotFound = 48,
}

/// Crowdfunding campaign errors: campaigns, categories, tags, backer reward
/// tiers, perks, comments and hard-cap voting. Codes 200–239.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum CampaignError {
    /// Referenced campaign does not exist.
    CampaignNotFound = 200,
    /// Referenced campaign category does not exist.
    CampaignCategoryNotFound = 201,
    /// A category with the supplied name has already been registered.
    CampaignCategoryAlreadyExists = 202,
    /// Referenced campaign category exists but is not active.
    CampaignCategoryInactive = 203,
    /// Referenced campaign tag does not exist.
    CampaignTagNotFound = 204,
    /// A tag with the supplied name has already been registered.
    CampaignTagAlreadyExists = 205,
    /// Campaign goal_amount must be positive.
    InvalidCampaignGoal = 206,
    /// Campaign deadline must be in the future.
    InvalidCampaignDeadline = 207,
    /// Operation referred to a campaign that has been deactivated.
    CampaignInactive = 208,
    /// The caller is not the merchant that owns the campaign.
    NotCampaignMerchant = 209,
    /// The campaign's deadline has passed; no further contributions accepted.
    CampaignExpired = 210,
    /// The campaign has already been closed out.
    CampaignEnded = 211,
    /// The campaign is not currently active.
    CampaignNotActive = 212,
    /// Referenced affiliate is not registered on this campaign.
    AffiliateNotFound = 213,
    /// Referenced reward tier does not exist on this campaign.
    InvalidRewardTier = 214,
    /// Pledge amount is below the selected tier's minimum.
    PledgeBelowTierMinimum = 215,
    /// The selected reward tier has reached its backer limit.
    RewardTierAtCapacity = 216,
    /// This backer's reward has already been marked fulfilled.
    BackerRewardAlreadyFulfilled = 217,
    /// The reward must be fulfilled before this operation.
    BackerRewardNotFulfilled = 218,
    /// The caller has not backed this campaign.
    NotBacker = 219,
    /// Referenced perk does not exist on the tier.
    PerkNotFound = 220,
    /// This perk has already been claimed by the backer.
    PerkAlreadyClaimed = 221,
    /// Reward tiers must be supplied in ascending minimum-pledge order.
    InvalidTierOrdering = 222,
    /// Generic NFT reward failure.
    NftError = 223,
    /// Referenced comment does not exist.
    CommentNotFound = 224,
    /// Comment body may not be empty.
    EmptyComment = 225,
    /// The comment is not in a state that permits this operation.
    InvalidCommentStatus = 226,
    /// Referenced hard-cap vote does not exist.
    VotingNotFound = 227,
    /// The hard-cap vote is not currently open.
    VotingNotActive = 228,
    /// This address has already cast a hard-cap vote.
    AlreadyVoted = 229,
}

/// Stretch-goal milestone errors. Codes 240–259.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum StretchGoalError {
    /// No stretch goal exists for the supplied id.
    StretchGoalNotFound = 240,
    /// The goal has not been unlocked yet.
    StretchGoalNotUnlocked = 241,
    /// The goal has already left the `Pending` state.
    GoalAlreadyUnlocked = 242,
    /// This backer has already claimed the reward for this goal.
    RewardAlreadyClaimed = 243,
    /// No reward has been granted to this backer for this goal.
    RewardNotFound = 244,
    /// A reward has already been granted to this backer for this goal.
    RewardAlreadyGranted = 245,
    /// The goal's target must exceed the campaign's base funding goal.
    TargetBelowBaseGoal = 246,
    /// Stretch goal targets must be strictly increasing per campaign.
    TargetNotIncreasing = 247,
    /// The campaign has not raised enough to unlock this goal.
    TargetNotReached = 248,
    /// The goal was cancelled and can no longer be unlocked or claimed.
    GoalCancelled = 249,
    /// Only the campaign's owning merchant may perform this operation.
    NotGoalOwner = 250,
    /// The campaign already has the maximum number of stretch goals.
    TooManyStretchGoals = 251,
}

/// Creator fund-vesting errors. Codes 260–279.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum VestingError {
    /// No vesting schedule exists for the supplied campaign.
    VestingNotFound = 260,
    /// The campaign already has a vesting schedule; schedules are immutable
    /// once created so backers can rely on the published terms.
    VestingAlreadyExists = 261,
    /// The caller is not the creator this schedule pays out to.
    NotVestingBeneficiary = 262,
    /// `vesting_duration` is zero, or the cliff outlasts the whole schedule.
    InvalidVestingDuration = 263,
    /// `initial_unlock_bps` exceeds 100% (10_000 basis points).
    InvalidUnlockBps = 264,
    /// The schedule commits more than the campaign has actually raised.
    VestingAmountExceedsRaised = 265,
    /// Nothing has vested since the last release.
    NothingToRelease = 266,
    /// The schedule was revoked; nothing further will vest.
    VestingRevoked = 267,
    /// Every vested token has already been released.
    VestingCompleted = 268,
    /// `start_time` is in the past, which would unlock funds retroactively.
    InvalidVestingStart = 269,
    /// This creator already holds the maximum number of vesting schedules.
    TooManyVestingSchedules = 270,
}

/// Fiat-pegged campaign goal errors. Codes 280–299.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum FiatGoalError {
    /// No fiat-pegged goal exists for the supplied campaign.
    FiatGoalNotFound = 280,
    /// The campaign already has a fiat goal; the peg is immutable once
    /// published so backers can rely on the target they were shown.
    FiatGoalAlreadyExists = 281,
    /// The fiat target must be positive.
    InvalidFiatGoalAmount = 282,
    /// The quote currency is empty or longer than the accepted maximum.
    InvalidFiatCurrency = 283,
    /// `decimals` exceeds the accepted maximum, or an oracle's configured
    /// decimals are large enough to overflow the conversion.
    InvalidFiatDecimals = 284,
    /// The goal has been wound down; no further contributions are valued.
    FiatGoalClosed = 285,
    /// The caller is not the merchant that owns the pegged campaign.
    NotFiatGoalOwner = 286,
    /// The contribution is worth less than one minor unit of the goal's
    /// currency at the current price, so crediting it would round to nothing.
    FiatValueTooSmall = 287,
}

/// Campaign analytics export errors. Codes 300–319.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum AnalyticsError {
    /// No export exists under the supplied ID.
    AnalyticsExportNotFound = 300,
    /// The caller is not the merchant that owns the campaign being exported.
    NotAnalyticsExportOwner = 301,
    /// The campaign has neither tracked contributions nor a raise of its own,
    /// so an export would snapshot an empty dataset and pay rent for nothing.
    NothingToExport = 302,
    /// This campaign has already run the maximum number of exports.
    TooManyExports = 303,
    /// The campaign has never been exported.
    NoExportsYet = 304,
}
