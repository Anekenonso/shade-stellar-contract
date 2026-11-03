use soroban_sdk::{contracttype, Address, BytesN, String, Vec};
use shade::types::*;

/// Crowdfunding campaign storage keys.
///
/// Three historically separate features each defined their own `Campaign`
/// record but shared one storage key, which silently overwrote one another.
/// They now have distinct keys and distinct value types:
/// [`Campaign`] (categories/tags), [`FeeCampaign`] (fee policy, staking,
/// affiliates) and [`PledgeCampaign`] (pledges and refunds).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CampaignKey {
    // --- Campaign categories & tagging (#352) ---
    /// A predefined campaign category created by the admin.
    CampaignCategory(u64),
    /// Total number of campaign categories ever created (never decreases).
    CampaignCategoryCount,
    /// Name -> category_id, used to enforce unique category names.
    CampaignCategoryName(String),
    /// A free-form campaign tag created by a merchant.
    CampaignTag(u64),
    /// Total number of campaign tags ever created (never decreases).
    CampaignTagCount,
    /// Name -> tag_id, used to enforce unique tag names.
    CampaignTagName(String),
    /// A fundraising campaign registered by a merchant.
    Campaign(u64),
    /// Total number of campaigns ever created (never decreases).
    CampaignCount,
    /// Reverse index: category_id -> ordered list of campaign IDs.
    CategoryCampaigns(u64),
    /// Reverse index: tag_id -> ordered list of campaign IDs.
    TagCampaigns(u64),
    /// Reverse index: merchant_id -> ordered list of campaign IDs.
    MerchantCampaigns(u64),
    /// Reverse index: campaign_id -> ordered list of attached tag IDs.
    CampaignTagList(u64),
    // --- Fee-policy / staking / affiliate campaigns ---
    FeeCampaign(u64),
    FeeCampaignCount,
    CampaignParticipants(u64),
    CampaignParticipant(u64, Address),
    CampaignAffiliate(u64, Address),
    // --- Pledge-based campaigns with refunds ---
    PledgeCampaign(u64),
    PledgeCampaignCount,
    Pledge(u64),
    PledgeCount,
    CampaignPledges(u64),
    ContributorPledges(Address),
    // --- Donor leaderboard ---
    CampaignOwner(u64),
    CampaignTopDonors(u64),
    CampaignDonorAmount(u64, Address),
    // --- Backer comments & moderation ---
    Comment(u64),
    CommentCount,
    CommentFlag(u64),
    CrowdfundComments(u64),
    UserComments(Address),
    // --- Vesting ---
    CrowdfundVestingConfig(u64),
    VestingSchedule(u64, u64),
    VestingTimeline(u64),
    VestingTimelineCount,
    // --- Dynamic hard-cap voting ---
    DynamicHardCap(u64),
    HardCapVote(u64, Address),
    HardCapVoting(u64),
}

/// Backer reward tier / perk storage keys.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackerKey {
    Campaign(u64),
    CampaignCount,
    RewardTiers(u64),
    Pledge(u64, Address),
    SelectedTier(u64, Address),
    RewardFulfilled(u64, Address),
    PerkClaimed(u64, Address, u32),
    TierBackerCount(u64, u32),
}

/// Stretch-goal milestone storage keys.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StretchKey {
    /// A single stretch goal, keyed by its global ID.
    StretchGoal(u64),
    /// Running counter for stretch goal IDs (never decreases).
    StretchGoalCount,
    /// Per-backer reward for a goal. Keyed by `(goal_id, backer)` so a goal can
    /// grant rewards to any number of backers.
    StretchGoalReward(u64, Address),
    /// Reverse index: campaign_id -> ordered list of that campaign's goal IDs.
    CampaignStretchGoals(u64),
}

/// Creator fund-vesting storage keys.
///
/// A dedicated enum, like [`StretchKey`], so this feature adds no cases to
/// [`CampaignKey`] (Soroban caps every enum at 50 cases).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VestingKey {
    /// The single vesting schedule attached to a backer campaign, keyed by
    /// `campaign_id`. Keying on the campaign avoids a separate ID counter and
    /// its extra storage entry, and makes "does this campaign vest?" a single
    /// read.
    CreatorVesting(u64),
    /// Reverse index: creator address -> the campaign IDs they vest funds from.
    /// Holds only `u64` IDs rather than whole records, keeping the entry small.
    CreatorVestingList(Address),
}

/// Fiat-pegged campaign goal storage keys.
///
/// A dedicated enum, like [`VestingKey`], so this feature adds no cases to the
/// near-full [`CampaignKey`] (Soroban caps every enum at 50 cases).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FiatGoalKey {
    /// The single fiat-denominated goal pegged to a campaign, keyed by
    /// `campaign_id`. Keying on the campaign avoids a separate ID counter and
    /// its extra storage entry, and makes "is this campaign fiat-pegged?" a
    /// single read.
    CampaignFiatGoal(u64),
    /// Cumulative fiat value one backer has contributed to one campaign, keyed
    /// by `(campaign_id, backer)`. A bare `i128` rather than a record, and
    /// updated in place, so per-backer tracking costs one small entry that
    /// never grows.
    BackerFiatContribution(u64, Address),
}

/// Campaign analytics and creator export storage keys.
///
/// A dedicated enum, like [`FiatGoalKey`], so this feature adds no cases to the
/// near-full [`CampaignKey`] (Soroban caps every enum at 50 cases).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnalyticsKey {
    /// Running aggregate for one backer campaign *and* the cursor marking how
    /// much of it has already been exported, keyed by `campaign_id`. The two
    /// live in one entry deliberately: an export reads and writes them
    /// together, so folding them into a single record halves both the rent and
    /// the storage round-trips a snapshot costs.
    CampaignStats(u64),
    /// A stored export snapshot, keyed by its global ID.
    AnalyticsExport(u64),
    /// Running counter for export IDs (never decreases).
    AnalyticsExportCount,
    /// Reverse index: campaign_id -> its export IDs, in run order. Holds only
    /// `u64` IDs rather than whole records, keeping the entry small, and is
    /// capped so it cannot grow without bound. There is deliberately no
    /// creator-keyed twin: campaign ownership is 1:1 with the creator and every
    /// export event carries the creator address, so a second index would pay
    /// rent for something the emitted events already answer.
    CampaignExports(u64),
}

/// NFT reward storage keys.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NftKey {
    NftCollection(u64),
    NftCollectionCount,
    Nft(u64),
    NftCount,
    CollectionNfts(u64),
    UserNfts(Address),
    NftClaimed(u64, Address),
}

/// Cross-chain bridge storage keys.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BridgeKey {
    BridgeListener(Address),
    BridgeListenerCount,
    BridgeDeposit(u64),
    BridgeDepositCount,
    /// Credited balance from bridged deposits, keyed by `(recipient, token)`.
    BridgeCredit(Address, Address),
    /// Idempotency guard keyed by the origin-chain transaction hash.
    ProcessedBridgeDeposit(BytesN<32>),
    CrossChainPledge(u64),
    CrossChainPledgeCount,
    /// `(source_chain, source_pledge_id)` -> local pledge ID.
    PledgeIdBySourceChain(String, u64),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CrossChainPledgeStatus {
    Pending,
    Completed,
    Failed,
    Refunded,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CrossChainPledge {
    pub id: u64,
    pub source_chain: String,
    pub source_pledge_id: u64,
    pub destination_chain: String,
    pub merchant: Address,
    pub payer: Address,
    pub token: Address,
    pub amount: i128,
    pub status: CrossChainPledgeStatus,
    pub created_at: u64,
    pub updated_at: u64,
    pub memo: Option<String>,
}

/// A confirmed external-chain deposit recorded by an authorized bridge listener.
///
/// The `source_tx_id` is the 32-byte transaction hash on the origin chain and
/// doubles as the global idempotency key
/// (see [`BridgeKey::ProcessedBridgeDeposit`]).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BridgeDeposit {
    pub id: u64,
    pub source_chain: String,
    pub source_tx_id: BytesN<32>,
    pub listener: Address,
    pub token: Address,
    pub amount: i128,
    pub recipient: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CampaignCategory {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub active: bool,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CampaignTag {
    pub id: u64,
    pub name: String,
    pub creator: Address,
    pub timestamp: u64,
}

/// A fundraising campaign registered by a merchant, classified by category and
/// free-form tags. Stored under [`CampaignKey::Campaign`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Campaign {
    pub id: u64,
    pub merchant_id: u64,
    pub merchant: Address,
    pub title: String,
    pub description: String,
    pub category_id: u64,
    pub tags: Vec<u64>,
    pub goal_amount: i128,
    pub token: Address,
    pub deadline: u64,
    pub raised_amount: i128,
    pub active: bool,
    pub created_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CampaignFilter {
    pub is_active: Option<bool>,
    pub category_id: Option<u64>,
    pub tag_id: Option<u64>,
    pub merchant_id: Option<u64>,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum CampaignStatus {
    Active = 0,
    Ended = 1,
    Cancelled = 2,
}

/// A promotional campaign carrying a fee waiver and discount policy, with
/// participant staking, slashing and affiliate commissions.
/// Stored under [`CampaignKey::FeeCampaign`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeCampaign {
    pub id: u64,
    pub owner: Address,
    pub name: String,
    pub charity: bool,
    pub fee_waiver_bps: u32,
    pub discount_bps: u32,
    pub stake_required: i128,
    pub total_raised: i128,
    pub total_staked: i128,
    pub total_slashed: i128,
    pub total_commissions_paid: i128,
    pub active: bool,
    pub created_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CampaignParticipant {
    pub campaign_id: u64,
    pub participant: Address,
    pub contributed: i128,
    pub staked: i128,
    pub slashed: i128,
    pub commissions_paid: i128,
    /// Leaderboard ranking score: contributions plus live stake, less slashes.
    pub score: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CampaignAffiliate {
    pub campaign_id: u64,
    pub affiliate: Address,
    pub commission_bps: u32,
    pub total_paid: i128,
    pub active: bool,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum PledgeCampaignStatus {
    Active = 0,
    Executed = 1,
    Cancelled = 2,
}

/// An all-or-nothing crowdfunding campaign: contributors pledge before the
/// deadline and are refunded if the goal is not met.
/// Stored under [`CampaignKey::PledgeCampaign`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PledgeCampaign {
    pub id: u64,
    pub merchant_id: u64,
    pub merchant: Address,
    pub title: String,
    pub goal: i128,
    pub token: Address,
    pub deadline: u64,
    pub raised: i128,
    pub status: PledgeCampaignStatus,
    pub date_created: u64,
    /// True once `batch_refund` has run, so it cannot run twice.
    pub refunds_processed: bool,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum PledgeStatus {
    Active = 0,
    Refunded = 1,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pledge {
    pub id: u64,
    pub campaign_id: u64,
    pub contributor: Address,
    pub amount: i128,
    pub token: Address,
    pub status: PledgeStatus,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DonorInfo {
    pub donor: Address,
    pub total_donated: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackerCampaign {
    pub id: u64,
    pub merchant_id: u64,
    pub name: String,
    pub token: Address,
    pub deadline: u64,
    pub raised: i128,
    pub active: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackerPerk {
    pub name: String,
    pub description: String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackerRewardTier {
    pub name: String,
    pub description: String,
    /// Minimum cumulative pledge required to select this tier.
    pub min_pledge: i128,
    pub perks: Vec<BackerPerk>,
    /// Maximum number of backers allowed on this tier. `0` means unlimited.
    pub max_backers: u32,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum CommentStatus {
    Active = 0,
    Flagged = 1,
    Removed = 2,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackerComment {
    pub id: u64,
    pub crowdfund_id: u64,
    pub author: Address,
    pub content: String,
    pub status: CommentStatus,
    pub created_at: u64,
    pub updated_at: u64,
    pub flag_count: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommentFlag {
    pub comment_id: u64,
    pub flagger: Address,
    pub reason: String,
    pub flagged_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VestingTimeline {
    pub id: u64,
    pub name: String,
    pub cliff_duration: u64,
    pub vesting_duration: u64,
    /// Share unlocked at the cliff, in basis points (10_000 = 100%).
    pub unlock_percentage: i128,
    pub admin: Address,
    pub created_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VestingSchedule {
    pub timeline_id: u64,
    pub tranche_index: u64,
    pub unlock_amount: i128,
    pub unlock_timestamp: u64,
    pub released: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CrowdfundVestingConfig {
    pub crowdfund_id: u64,
    pub timeline_id: u64,
    pub total_vesting_amount: i128,
    pub configured_at: u64,
}

/// Lifecycle of a campaign creator's vesting schedule.
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum CreatorVestingStatus {
    /// Funds are vesting; the creator may release whatever has vested.
    Active = 0,
    /// Every vested token has been released; the schedule is finished.
    Completed = 1,
    /// The admin froze the schedule. Whatever had vested at that moment stays
    /// claimable; nothing further will ever vest.
    Revoked = 2,
}

/// A campaign creator's claim on the funds their campaign raised, released
/// gradually rather than all at once.
///
/// `creator` and `token` are denormalized from the campaign record so a release
/// needs one storage read instead of three (campaign -> merchant id ->
/// merchant). The extra 2 fields cost far less rent than the reads cost CPU on
/// every claim.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatorVesting {
    pub campaign_id: u64,
    /// Beneficiary; the merchant that owns the campaign.
    pub creator: Address,
    /// Token the campaign raised in, and that releases pay out in.
    pub token: Address,
    /// Total committed to the schedule. On revocation this is lowered to the
    /// amount that had vested at that instant, freezing the remainder.
    pub total_amount: i128,
    /// Cumulative amount already paid out. Never exceeds `total_amount`.
    pub released_amount: i128,
    /// Ledger timestamp the schedule is measured from.
    pub start_time: u64,
    /// Seconds after `start_time` before anything unlocks.
    pub cliff_duration: u64,
    /// Total seconds from `start_time` until fully vested.
    pub vesting_duration: u64,
    /// Share released in one lump at the cliff, in basis points (10_000 = 100%).
    /// The remainder vests linearly from the cliff to the end.
    pub initial_unlock_bps: u32,
    pub status: CreatorVestingStatus,
    pub created_at: u64,
    /// Timestamp of the most recent release; `0` if none yet.
    pub last_release_at: u64,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum VotingStatus {
    Active = 0,
    Passed = 1,
    Failed = 2,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum VoteDirection {
    Increase = 0,
    Decrease = 1,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HardCapVoting {
    pub crowdfund_id: u64,
    pub current_cap: i128,
    pub proposed_cap: i128,
    pub voting_start: u64,
    pub voting_end: u64,
    pub votes_for: u32,
    pub votes_against: u32,
    pub status: VotingStatus,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HardCapVote {
    pub crowdfund_id: u64,
    pub voter: Address,
    pub proposed_cap: i128,
    pub direction: VoteDirection,
    pub created_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DynamicHardCapConfig {
    pub crowdfund_id: u64,
    pub hard_cap: i128,
    pub voting_duration: u64,
    pub min_votes_required: u32,
    pub last_updated: u64,
}

/// Lifecycle state of a stretch goal milestone.
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum StretchGoalStatus {
    /// Created; the campaign has not yet raised `target_amount`.
    Pending = 0,
    /// The campaign reached `target_amount` and the goal is now live.
    Unlocked = 1,
    /// Retired by the owning merchant before it was unlocked.
    Cancelled = 2,
}

/// A funding milestone beyond a campaign's base goal, unlocked once the
/// campaign's cumulative raise reaches `target_amount`.
///
/// Stored under [`StretchKey::StretchGoal`]. The denormalized `reward_count`
/// and `total_reward_amount` counters let indexers and UIs report on a goal
/// without scanning every per-backer reward entry.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StretchGoal {
    pub id: u64,
    /// Campaign this milestone belongs to.
    pub campaign_id: u64,
    /// Merchant that owns the campaign; the only address allowed to manage it.
    pub merchant: Address,
    /// Cumulative campaign raise (in token base units) that unlocks this goal.
    pub target_amount: i128,
    pub description: String,
    pub reward_description: String,
    pub status: StretchGoalStatus,
    pub created_at: u64,
    /// Ledger timestamp when the goal was unlocked; `0` while pending.
    pub unlocked_at: u64,
    /// Number of backers granted a reward for this goal.
    pub reward_count: u32,
    /// Sum of all reward amounts granted for this goal.
    pub total_reward_amount: i128,
}

/// A reward granted to one backer for one unlocked stretch goal.
/// Stored under [`StretchKey::StretchGoalReward`], keyed by `(goal_id, backer)`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StretchGoalReward {
    pub goal_id: u64,
    pub campaign_id: u64,
    pub backer: Address,
    pub reward_amount: i128,
    pub claimed: bool,
    pub granted_at: u64,
    /// Ledger timestamp when the backer claimed; `0` while unclaimed.
    pub claimed_at: u64,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum FiatGoalStatus {
    /// Accepting contributions; the fiat target has not been met.
    Active = 0,
    /// The fiat target has been met. Still accepting contributions, so an
    /// overfunded campaign keeps accruing.
    Reached = 1,
    /// Wound down by the owning merchant or the admin; no further contributions
    /// are valued against it.
    Closed = 2,
}

/// A campaign funding target denominated in fiat rather than in the campaign's
/// token, tracked against oracle-valued contributions.
/// Stored under [`FiatGoalKey::CampaignFiatGoal`].
///
/// `goal_amount` and `raised_amount` are both minor units of `currency` scaled
/// by `decimals` (e.g. `1_000_000` with `decimals = 2` is $10,000.00). Each
/// contribution is valued once, at the oracle price of the moment it lands, and
/// that snapshot is what accrues — so a later price swing never revalues a
/// contribution that was already counted.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CampaignFiatGoal {
    pub campaign_id: u64,
    /// Merchant that owns the campaign; the only address allowed to manage the
    /// peg.
    pub merchant: Address,
    /// Token contributions are denominated in, and whose oracle prices them.
    pub token: Address,
    /// Quote currency passed to the oracle, e.g. `"USD"`.
    pub currency: String,
    /// Fiat target, in minor units scaled by `decimals`.
    pub goal_amount: i128,
    /// Number of fractional digits `goal_amount` and `raised_amount` carry.
    pub decimals: u32,
    /// Cumulative fiat value credited so far, each contribution snapshotted at
    /// the price it landed at.
    pub raised_amount: i128,
    /// Token base units folded into `raised_amount`.
    pub raised_tokens: i128,
    /// Number of contributions valued against this goal.
    pub contribution_count: u32,
    pub status: FiatGoalStatus,
    pub created_at: u64,
    /// Most recent oracle price used, scaled by the oracle's `price_decimals`.
    pub last_price: i128,
    /// Ledger timestamp `last_price` was read at.
    pub last_priced_at: u64,
    /// Ledger timestamp the target was met; `0` while unmet.
    pub reached_at: u64,
}

/// A live valuation of a fiat-pegged goal at the current oracle price.
///
/// Derived on demand and never stored: it exists so a UI can render "$4,200 of
/// $10,000 — 1,383 XLM to go" from one read-only call, without paying rent on a
/// figure that goes stale the moment the price moves.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiatGoalQuote {
    pub campaign_id: u64,
    pub token: Address,
    pub currency: String,
    pub goal_amount: i128,
    pub raised_amount: i128,
    /// Fiat still needed to meet the target; `0` once met.
    pub remaining_amount: i128,
    /// Current oracle price for one whole token, scaled by `price_decimals`.
    pub price: i128,
    pub price_decimals: u32,
    /// Token base units that would close `remaining_amount` at `price`, rounded
    /// up so contributing exactly this much always meets the target.
    pub tokens_required: i128,
    /// Progress toward the target in basis points, capped at 10_000.
    pub progress_bps: u32,
    pub status: FiatGoalStatus,
    pub quoted_at: u64,
}

/// Serialization a creator wants an export rendered as off-chain.
///
/// The contract does not build the file — it publishes the figures and the
/// format the creator asked for, and indexers render the bytes. Storing the
/// intent on-chain is what makes an export reproducible: anyone can rebuild the
/// same file from the same snapshot.
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ExportFormat {
    /// One header row plus one row of figures.
    Csv,
    /// A single JSON object.
    Json,
    /// Newline-delimited JSON, one object per export in a series.
    Ndjson,
}

/// Running contribution aggregate for one backer campaign, plus the cursor
/// marking how much of it previous exports have already covered.
///
/// Counters accrue from contributions recorded *after* this component shipped.
/// A campaign that raised funds before then keeps that raise in its own
/// `BackerCampaign::raised`, which stays authoritative; `tracked_raised` is the
/// slice this component actually observed. Exports carry both, so a consumer
/// can always tell the two apart rather than silently reading a short total as
/// the whole raise.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CampaignStats {
    pub campaign_id: u64,
    /// Contributions recorded since tracking began.
    pub pledge_count: u32,
    /// Distinct addresses that have contributed since tracking began.
    pub backer_count: u32,
    /// Token base units contributed since tracking began.
    pub tracked_raised: i128,
    /// Largest single contribution; `0` before the first one.
    pub largest_pledge: i128,
    /// Smallest single contribution; `0` before the first one.
    pub smallest_pledge: i128,
    pub first_pledge_at: u64,
    pub last_pledge_at: u64,
    // ── Export cursor ──
    /// Exports run against this campaign so far.
    pub export_count: u32,
    /// ID of the most recent export; `0` before the first one.
    pub last_export_id: u64,
    /// Ledger time of the most recent export; `0` before the first one.
    pub last_export_at: u64,
    /// Counters as of the last export. The difference against the live figures
    /// above is what the next export reports as its period delta, which is why
    /// two exports in the same ledger second still partition the data exactly.
    pub exported_pledge_count: u32,
    pub exported_backer_count: u32,
    pub exported_raised: i128,
}

/// An immutable analytics snapshot a creator exported for one campaign.
///
/// Consecutive exports partition the campaign's timeline: each one covers
/// everything since the previous one's `period_end`, so a creator can pull
/// incremental deltas rather than re-reading the whole history every time.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalyticsExport {
    pub id: u64,
    pub campaign_id: u64,
    /// The campaign's owning merchant, who authorized this export.
    pub creator: Address,
    pub merchant_id: u64,
    pub token: Address,
    pub format: ExportFormat,
    /// 1-based position in this campaign's export series.
    pub sequence: u32,
    /// Start of the window this export covers: the previous export's
    /// `period_end`, or `0` for the first export in a series.
    pub period_start: u64,
    /// End of the window, always the ledger time the export ran.
    pub period_end: u64,
    // ── Campaign context ──
    /// The campaign's own raise total, authoritative even for contributions
    /// taken before analytics tracking began.
    pub campaign_raised: i128,
    pub campaign_deadline: u64,
    pub campaign_active: bool,
    // ── Cumulative, as observed by analytics tracking ──
    pub total_raised: i128,
    pub pledge_count: u32,
    pub backer_count: u32,
    /// `total_raised / pledge_count`, truncated.
    pub average_pledge: i128,
    pub largest_pledge: i128,
    pub smallest_pledge: i128,
    pub first_pledge_at: u64,
    pub last_pledge_at: u64,
    // ── Delta since the previous export ──
    pub period_raised: i128,
    pub period_pledges: u32,
    pub period_backers: u32,
    pub created_at: u64,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum NftStatus {
    Active = 0,
    Burned = 1,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NftCollection {
    pub id: u64,
    pub merchant_id: u64,
    pub merchant: Address,
    pub name: String,
    pub base_uri: String,
    /// Maximum mintable supply. `0` means unlimited.
    pub max_supply: u64,
    pub minted: u64,
    pub royalty_bps: u32,
    pub active: bool,
    pub created_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Nft {
    pub id: u64,
    pub collection_id: u64,
    pub owner: Address,
    pub uri: String,
    pub status: NftStatus,
    pub minted_at: u64,
    /// Original mint recipient, used to gate reward claims.
    pub recipient: Address,
}

/// Lifecycle status of an identity-verification record.
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum VerificationStatus {
    /// No request has been submitted yet.
    Unverified = 0,
    /// Request submitted; awaiting reviewer action.
    Pending = 1,
    /// Request approved; subject is KYC-cleared.
    Approved = 2,
    /// Request rejected by a reviewer.
    Rejected = 3,
    /// Previously approved status revoked for compliance reasons.
    Suspended = 4,
}

/// The scope / purpose of a KYC verification request.
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum VerificationType {
    /// General identity check for individual users.
    Individual = 0,
    /// Enhanced check required before launching a crowdfunding campaign.
    CampaignCreator = 1,
    /// Lightweight check for campaign backers (when the campaign requires it).
    Backer = 2,
}

/// A single KYC verification request submitted by or on behalf of `subject`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KycRequest {
    /// Auto-incremented unique ID.
    pub id: u64,
    /// The address whose identity is being verified.
    pub subject: Address,
    /// What kind of verification is being requested.
    pub verification_type: VerificationType,
    /// Ledger timestamp when the request was created.
    pub submitted_at: u64,
    /// Ledger timestamp when a reviewer last acted on the request; 0 = never.
    pub reviewed_at: u64,
    /// Address of the reviewer who last acted; zero-address placeholder when
    /// not yet reviewed. Stored as `Option<Address>` to keep the field
    /// XDR-compatible inside a `#[contracttype]`.
    pub reviewer: Option<Address>,
    /// Current lifecycle status of the request.
    pub status: VerificationStatus,
    /// How many supporting documents were attached (informational).
    pub document_count: u32,
    /// Arbitrary off-chain metadata string (e.g. encrypted doc reference, IPFS CID).
    pub metadata: soroban_sdk::String,
}

/// Per-campaign KYC configuration maintained by campaign creators / reviewers.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CampaignKycStatus {
    /// The campaign this record belongs to.
    pub campaign_id: u64,
    /// Address of the campaign creator.
    pub creator: Address,
    /// Whether the campaign creator has been KYC-verified.
    pub kyc_status: VerificationStatus,
    /// When `true`, every backer must hold a valid KYC approval before
    /// their contribution is counted.
    pub min_backer_kyc_required: bool,
    /// Ledger timestamp when this record was first created.
    pub created_at: u64,
    /// Ledger timestamp when the campaign was KYC-verified (0 = not yet).
    pub verified_at: u64,
    /// Address of the reviewer who verified the campaign; `None` if not yet verified.
    pub verified_by: Option<Address>,
}

/// Backer-level KYC status snapshot cached per (campaign, backer) pair.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackerKycStatus {
    /// The backer's address.
    pub backer: Address,
    /// The backer's most recent KYC verification status.
    pub kyc_status: VerificationStatus,
    /// Running count of campaigns this backer has contributed to (across all campaigns).
    pub campaigns_backed: u64,
    /// Cumulative amount contributed across all campaigns (informational).
    pub total_backed_amount: i128,
    /// Ledger timestamp of the last time this record was refreshed.
    pub last_kyc_check: u64,
}
