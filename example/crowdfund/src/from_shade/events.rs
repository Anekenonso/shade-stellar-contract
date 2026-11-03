use soroban_sdk::{contractevent, Address, BytesN, Env, String, Vec};
use super::types::*;
use shade::types::*;

#[contractevent]
pub struct CrossChainPledgeCreatedEvent {
    pub pledge_id: u64,
    pub source_chain: String,
    pub source_pledge_id: u64,
    pub merchant: Address,
    pub payer: Address,
    pub token: Address,
    pub amount: i128,
    pub timestamp: u64,
}

pub fn publish_cross_chain_pledge_created(env: &Env, pledge: &crate::types::CrossChainPledge) {
    CrossChainPledgeCreatedEvent {
        pledge_id: pledge.id,
        source_chain: pledge.source_chain.clone(),
        source_pledge_id: pledge.source_pledge_id,
        merchant: pledge.merchant.clone(),
        payer: pledge.payer.clone(),
        token: pledge.token.clone(),
        amount: pledge.amount,
        timestamp: pledge.created_at,
    }
    .publish(env);
}

#[contractevent]
pub struct CrossChainPledgeUpdatedEvent {
    pub pledge_id: u64,
    pub status: crate::types::CrossChainPledgeStatus,
    pub timestamp: u64,
}

pub fn publish_cross_chain_pledge_updated(env: &Env, pledge: &crate::types::CrossChainPledge) {
    CrossChainPledgeUpdatedEvent {
        pledge_id: pledge.id,
        status: pledge.status.clone(),
        timestamp: pledge.updated_at,
    }
    .publish(env);
}

#[contractevent]
pub struct LeaderboardUpdatedEvent {
    pub campaign_id: u64,
    pub donor: Address,
    pub amount: i128,
    pub new_total: i128,
    pub timestamp: u64,
}

pub fn publish_leaderboard_updated_event(
    env: &Env,
    campaign_id: u64,
    donor: Address,
    amount: i128,
    new_total: i128,
    timestamp: u64,
) {
    LeaderboardUpdatedEvent {
        campaign_id,
        donor,
        amount,
        new_total,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct CampaignCategoryCreatedEvent {
    pub category_id: u64,
    pub admin: Address,
    pub name: String,
    pub description: String,
    pub timestamp: u64,
}

pub fn publish_campaign_category_created_event(
    env: &Env,
    category_id: u64,
    admin: Address,
    name: String,
    description: String,
    timestamp: u64,
) {
    CampaignCategoryCreatedEvent {
        category_id,
        admin,
        name,
        description,
        timestamp,
    }
    .publish(env)
}

#[contractevent]
pub struct CampaignCategoryUpdatedEvent {
    pub category_id: u64,
    pub admin: Address,
    pub name: String,
    pub description: String,
    pub active: bool,
    pub timestamp: u64,
}

pub fn publish_campaign_category_updated_event(
    env: &Env,
    category_id: u64,
    admin: Address,
    name: String,
    description: String,
    active: bool,
    timestamp: u64,
) {
    CampaignCategoryUpdatedEvent {
        category_id,
        admin,
        name,
        description,
        active,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct CampaignTagCreatedEvent {
    pub tag_id: u64,
    pub creator: Address,
    pub name: String,
    pub timestamp: u64,
}

pub fn publish_campaign_tag_created_event(
    env: &Env,
    tag_id: u64,
    creator: Address,
    name: String,
    timestamp: u64,
) {
    CampaignTagCreatedEvent {
        tag_id,
        creator,
        name,
        timestamp,
    }
    .publish(env);
}

#[allow(clippy::too_many_arguments)]
#[contractevent]
pub struct CampaignCreatedEvent {
    pub campaign_id: u64,
    pub merchant: Address,
    pub merchant_id: u64,
    pub title: String,
    pub description: String,
    pub category_id: u64,
    pub tags: Vec<u64>,
    pub goal_amount: i128,
    pub token: Address,
    pub deadline: u64,
    pub timestamp: u64,
}

#[allow(clippy::too_many_arguments)]
pub fn publish_campaign_created_event(
    env: &Env,
    campaign_id: u64,
    merchant: Address,
    merchant_id: u64,
    title: String,
    description: String,
    category_id: u64,
    tags: Vec<u64>,
    goal_amount: i128,
    token: Address,
    deadline: u64,
    timestamp: u64,
) {
    CampaignCreatedEvent {
        campaign_id,
        merchant,
        merchant_id,
        title,
        description,
        category_id,
        tags,
        goal_amount,
        token,
        deadline,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct CampaignUpdatedEvent {
    pub campaign_id: u64,
    pub merchant: Address,
    pub title: String,
    pub description: String,
    pub timestamp: u64,
}

pub fn publish_campaign_updated_event(
    env: &Env,
    campaign_id: u64,
    merchant: Address,
    title: String,
    description: String,
    timestamp: u64,
) {
    CampaignUpdatedEvent {
        campaign_id,
        merchant,
        title,
        description,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct CampaignStatusChangedEvent {
    pub campaign_id: u64,
    pub merchant: Address,
    pub active: bool,
    pub timestamp: u64,
}

pub fn publish_campaign_status_changed_event(
    env: &Env,
    campaign_id: u64,
    merchant: Address,
    active: bool,
    timestamp: u64,
) {
    CampaignStatusChangedEvent {
        campaign_id,
        merchant,
        active,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct CampaignTagAddedEvent {
    pub campaign_id: u64,
    pub merchant: Address,
    pub tag_id: u64,
    pub timestamp: u64,
}

pub fn publish_campaign_tag_added_event(
    env: &Env,
    campaign_id: u64,
    merchant: Address,
    tag_id: u64,
    timestamp: u64,
) {
    CampaignTagAddedEvent {
        campaign_id,
        merchant,
        tag_id,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct CampaignTagRemovedEvent {
    pub campaign_id: u64,
    pub merchant: Address,
    pub tag_id: u64,
    pub timestamp: u64,
}

pub fn publish_campaign_tag_removed_event(
    env: &Env,
    campaign_id: u64,
    merchant: Address,
    tag_id: u64,
    timestamp: u64,
) {
    CampaignTagRemovedEvent {
        campaign_id,
        merchant,
        tag_id,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct CampaignContributionEvent {
    pub campaign_id: u64,
    pub contributor: Address,
    pub amount: i128,
    pub raised_amount: i128,
    pub goal_amount: i128,
    pub timestamp: u64,
}

pub fn publish_campaign_contribution_event(
    env: &Env,
    campaign_id: u64,
    contributor: Address,
    amount: i128,
    raised_amount: i128,
    goal_amount: i128,
    timestamp: u64,
) {
    CampaignContributionEvent {
        campaign_id,
        contributor,
        amount,
        raised_amount,
        goal_amount,
        timestamp,
    }
    .publish(env);
}

/// Emitted when a merchant defines a new stretch goal on a campaign.
#[contractevent]
pub struct StretchGoalCreatedEvent {
    pub goal_id: u64,
    pub campaign_id: u64,
    /// Campaign owner; the only address permitted to manage this goal.
    pub merchant: Address,
    /// Cumulative campaign raise that unlocks this goal.
    pub target_amount: i128,
    /// The campaign's base funding goal, for computing the stretch delta.
    pub base_goal_amount: i128,
    /// Number of goals on the campaign after this one was added.
    pub goal_count: u32,
    pub timestamp: u64,
}

pub fn publish_stretch_goal_created_event(
    env: &Env,
    goal_id: u64,
    campaign_id: u64,
    merchant: Address,
    target_amount: i128,
    base_goal_amount: i128,
    goal_count: u32,
    timestamp: u64,
) {
    StretchGoalCreatedEvent {
        goal_id,
        campaign_id,
        merchant,
        target_amount,
        base_goal_amount,
        goal_count,
        timestamp,
    }
    .publish(env);
}

/// Emitted when a campaign's raise reaches a goal's target and it goes live.
#[contractevent]
pub struct StretchGoalUnlockedEvent {
    pub goal_id: u64,
    pub campaign_id: u64,
    pub merchant: Address,
    pub target_amount: i128,
    /// Campaign raise at the moment of unlocking; always >= target_amount.
    pub raised_amount: i128,
    pub timestamp: u64,
}

pub fn publish_stretch_goal_unlocked_event(
    env: &Env,
    goal_id: u64,
    campaign_id: u64,
    merchant: Address,
    target_amount: i128,
    raised_amount: i128,
    timestamp: u64,
) {
    StretchGoalUnlockedEvent {
        goal_id,
        campaign_id,
        merchant,
        target_amount,
        raised_amount,
        timestamp,
    }
    .publish(env);
}

/// Emitted when a merchant retires a goal before it was unlocked.
#[contractevent]
pub struct StretchGoalCancelledEvent {
    pub goal_id: u64,
    pub campaign_id: u64,
    pub merchant: Address,
    pub timestamp: u64,
}

pub fn publish_stretch_goal_cancelled_event(
    env: &Env,
    goal_id: u64,
    campaign_id: u64,
    merchant: Address,
    timestamp: u64,
) {
    StretchGoalCancelledEvent {
        goal_id,
        campaign_id,
        merchant,
        timestamp,
    }
    .publish(env);
}

/// Emitted when a backer is granted a reward for an unlocked goal.
#[contractevent]
pub struct StretchRewardGrantedEvent {
    pub goal_id: u64,
    pub campaign_id: u64,
    pub backer: Address,
    pub reward_amount: i128,
    /// Number of backers rewarded on this goal after this grant.
    pub reward_count: u32,
    /// Sum of all reward amounts granted on this goal after this grant.
    pub total_reward_amount: i128,
    pub timestamp: u64,
}

pub fn publish_stretch_reward_granted_event(
    env: &Env,
    goal_id: u64,
    campaign_id: u64,
    backer: Address,
    reward_amount: i128,
    reward_count: u32,
    total_reward_amount: i128,
    timestamp: u64,
) {
    StretchRewardGrantedEvent {
        goal_id,
        campaign_id,
        backer,
        reward_amount,
        reward_count,
        total_reward_amount,
        timestamp,
    }
    .publish(env);
}

/// Emitted when a backer claims their granted reward.
#[contractevent]
pub struct StretchRewardClaimedEvent {
    pub goal_id: u64,
    pub campaign_id: u64,
    pub backer: Address,
    pub reward_amount: i128,
    pub timestamp: u64,
}

pub fn publish_stretch_reward_claimed_event(
    env: &Env,
    goal_id: u64,
    campaign_id: u64,
    backer: Address,
    reward_amount: i128,
    timestamp: u64,
) {
    StretchRewardClaimedEvent {
        goal_id,
        campaign_id,
        backer,
        reward_amount,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct AffiliateCommissionPaidEvent {
    pub campaign_id: u64,
    pub affiliate_address: Address,
    pub amount: i128,
    pub total_paid: i128,
    pub timestamp: u64,
}

pub fn publish_affiliate_commission_paid_event(
    env: &Env,
    campaign_id: u64,
    affiliate_address: Address,
    amount: i128,
    total_paid: i128,
    timestamp: u64,
) {
    AffiliateCommissionPaidEvent {
        campaign_id,
        affiliate_address,
        amount,
        total_paid,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct AffiliateRegisteredEvent {
    pub campaign_id: u64,
    pub affiliate_address: Address,
    pub commission_bps: u32,
    pub timestamp: u64,
}

pub fn publish_affiliate_registered_event(
    env: &Env,
    campaign_id: u64,
    affiliate_address: Address,
    commission_bps: u32,
    timestamp: u64,
) {
    AffiliateRegisteredEvent {
        campaign_id,
        affiliate_address,
        commission_bps,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct BackerCampaignCreatedEvent {
    pub campaign_id: u64,
    pub merchant_addr: Address,
    pub merchant_id: u64,
    pub name: String,
    pub token: Address,
    pub deadline: u64,
    pub timestamp: u64,
}

pub fn publish_backer_campaign_created_event(
    env: &Env,
    campaign_id: u64,
    merchant_addr: Address,
    merchant_id: u64,
    name: String,
    token: Address,
    deadline: u64,
    timestamp: u64,
) {
    BackerCampaignCreatedEvent {
        campaign_id,
        merchant_addr,
        merchant_id,
        name,
        token,
        deadline,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct BackerCommentCreatedEvent {
    pub comment_id: u64,
    pub crowdfund_id: u64,
    pub author: Address,
    pub content_len: u64,
    pub now: u64,
}

pub fn publish_backer_comment_created_event(
    env: &Env,
    comment_id: u64,
    crowdfund_id: u64,
    author: Address,
    content_len: u64,
    now: u64,
) {
    BackerCommentCreatedEvent {
        comment_id,
        crowdfund_id,
        author,
        content_len,
        now,
    }
    .publish(env);
}

#[contractevent]
pub struct BackerCommentFlaggedEvent {
    pub comment_id: u64,
    pub flagger: Address,
    pub reason_len: u64,
    pub flag_count: u32,
    pub now: u64,
}

pub fn publish_backer_comment_flagged_event(
    env: &Env,
    comment_id: u64,
    flagger: Address,
    reason_len: u64,
    flag_count: u32,
    now: u64,
) {
    BackerCommentFlaggedEvent {
        comment_id,
        flagger,
        reason_len,
        flag_count,
        now,
    }
    .publish(env);
}

#[contractevent]
pub struct BackerCommentRemovedEvent {
    pub comment_id: u64,
    pub crowdfund_id: u64,
    pub moderator: Address,
    pub now: u64,
}

pub fn publish_backer_comment_removed_event(
    env: &Env,
    comment_id: u64,
    crowdfund_id: u64,
    moderator: Address,
    now: u64,
) {
    BackerCommentRemovedEvent {
        comment_id,
        crowdfund_id,
        moderator,
        now,
    }
    .publish(env);
}

#[contractevent]
pub struct BackerPerkClaimedEvent {
    pub campaign_id: u64,
    pub backer: Address,
    pub tier_index: u32,
    pub perk_index: u32,
    pub name: String,
    pub timestamp: u64,
}

pub fn publish_backer_perk_claimed_event(
    env: &Env,
    campaign_id: u64,
    backer: Address,
    tier_index: u32,
    perk_index: u32,
    name: String,
    timestamp: u64,
) {
    BackerPerkClaimedEvent {
        campaign_id,
        backer,
        tier_index,
        perk_index,
        name,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct BackerPledgeRecordedEvent {
    pub campaign_id: u64,
    pub backer: Address,
    pub amount: i128,
    pub new_pledge: i128,
    pub timestamp: u64,
}

pub fn publish_backer_pledge_recorded_event(
    env: &Env,
    campaign_id: u64,
    backer: Address,
    amount: i128,
    new_pledge: i128,
    timestamp: u64,
) {
    BackerPledgeRecordedEvent {
        campaign_id,
        backer,
        amount,
        new_pledge,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct BackerRewardFulfilledEvent {
    pub campaign_id: u64,
    pub merchant_addr: Address,
    pub backer: Address,
    pub tier_index: u32,
    pub pledge: i128,
    pub timestamp: u64,
}

pub fn publish_backer_reward_fulfilled_event(
    env: &Env,
    campaign_id: u64,
    merchant_addr: Address,
    backer: Address,
    tier_index: u32,
    pledge: i128,
    timestamp: u64,
) {
    BackerRewardFulfilledEvent {
        campaign_id,
        merchant_addr,
        backer,
        tier_index,
        pledge,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct BackerTierSelectedEvent {
    pub campaign_id: u64,
    pub backer: Address,
    pub tier_index: u32,
    pub min_pledge: i128,
    pub tier_len: u32,
    pub timestamp: u64,
}

pub fn publish_backer_reward_tier_selected_event(
    env: &Env,
    campaign_id: u64,
    backer: Address,
    tier_index: u32,
    min_pledge: i128,
    tier_len: u32,
    timestamp: u64,
) {
    BackerTierSelectedEvent {
        campaign_id,
        backer,
        tier_index,
        min_pledge,
        tier_len,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct BackerRewardTiersSetEvent {
    pub campaign_id: u64,
    pub merchant_addr: Address,
    pub tiers_len: u32,
    pub timestamp: u64,
}

pub fn publish_backer_reward_tiers_set_event(
    env: &Env,
    campaign_id: u64,
    merchant_addr: Address,
    tiers_len: u32,
    timestamp: u64,
) {
    BackerRewardTiersSetEvent {
        campaign_id,
        merchant_addr,
        tiers_len,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct CampaignBatchRefundedEvent {
    pub campaign_id: u64,
    pub total_refunded: i128,
    pub count: u32,
    pub timestamp: u64,
}

pub fn publish_campaign_batch_refunded_event(
    env: &Env,
    campaign_id: u64,
    total_refunded: i128,
    count: u32,
    timestamp: u64,
) {
    CampaignBatchRefundedEvent {
        campaign_id,
        total_refunded,
        count,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct CampaignCancelledEvent {
    pub campaign_id: u64,
    pub merchant_address: Address,
    pub timestamp: u64,
}

pub fn publish_campaign_cancelled_event(
    env: &Env,
    campaign_id: u64,
    merchant_address: Address,
    timestamp: u64,
) {
    CampaignCancelledEvent {
        campaign_id,
        merchant_address,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct CampaignContribRecordedEvent {
    pub campaign_id: u64,
    pub caller: Address,
    pub amount: i128,
    pub total_raised: i128,
    pub timestamp: u64,
}

pub fn publish_campaign_contribution_recorded_event(
    env: &Env,
    campaign_id: u64,
    caller: Address,
    amount: i128,
    total_raised: i128,
    timestamp: u64,
) {
    CampaignContribRecordedEvent {
        campaign_id,
        caller,
        amount,
        total_raised,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct CampaignExecutedEvent {
    pub campaign_id: u64,
    pub merchant_address: Address,
    pub raised: i128,
    pub timestamp: u64,
}

pub fn publish_campaign_executed_event(
    env: &Env,
    campaign_id: u64,
    merchant_address: Address,
    raised: i128,
    timestamp: u64,
) {
    CampaignExecutedEvent {
        campaign_id,
        merchant_address,
        raised,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct CampaignFeePolicySetEvent {
    pub campaign_id: u64,
    pub caller: Address,
    pub fee_waiver_bps: u32,
    pub discount_bps: u32,
    pub timestamp: u64,
}

pub fn publish_campaign_fee_policy_configured_event(
    env: &Env,
    campaign_id: u64,
    caller: Address,
    fee_waiver_bps: u32,
    discount_bps: u32,
    timestamp: u64,
) {
    CampaignFeePolicySetEvent {
        campaign_id,
        caller,
        fee_waiver_bps,
        discount_bps,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct CampaignSlashedEvent {
    pub campaign_id: u64,
    pub participant_address: Address,
    pub amount: i128,
    pub staked: i128,
    pub timestamp: u64,
}

pub fn publish_campaign_slashed_event(
    env: &Env,
    campaign_id: u64,
    participant_address: Address,
    amount: i128,
    staked: i128,
    timestamp: u64,
) {
    CampaignSlashedEvent {
        campaign_id,
        participant_address,
        amount,
        staked,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct CampaignStakedEvent {
    pub campaign_id: u64,
    pub caller: Address,
    pub amount: i128,
    pub staked: i128,
    pub timestamp: u64,
}

pub fn publish_campaign_staked_event(
    env: &Env,
    campaign_id: u64,
    caller: Address,
    amount: i128,
    staked: i128,
    timestamp: u64,
) {
    CampaignStakedEvent {
        campaign_id,
        caller,
        amount,
        staked,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct CommentModerationAppliedEvent {
    pub comment_id: u64,
    pub moderator: Address,
    pub action: String,
    pub now: u64,
}

pub fn publish_comment_moderation_applied_event(
    env: &Env,
    comment_id: u64,
    moderator: Address,
    action: String,
    now: u64,
) {
    CommentModerationAppliedEvent {
        comment_id,
        moderator,
        action,
        now,
    }
    .publish(env);
}

#[contractevent]
pub struct CrowdfundVestingSetEvent {
    pub crowdfund_id: u64,
    pub timeline_id: u64,
    pub total_vesting_amount: i128,
    pub admin: Address,
    pub now: u64,
}

pub fn publish_crowdfund_vesting_configured_event(
    env: &Env,
    crowdfund_id: u64,
    timeline_id: u64,
    total_vesting_amount: i128,
    admin: Address,
    now: u64,
) {
    CrowdfundVestingSetEvent {
        crowdfund_id,
        timeline_id,
        total_vesting_amount,
        admin,
        now,
    }
    .publish(env);
}

#[contractevent]
pub struct DynamicHardCapUpdatedEvent {
    pub crowdfund_id: u64,
    pub proposed_cap: i128,
    pub current_cap: i128,
    pub now: u64,
}

pub fn publish_dynamic_hard_cap_updated_event(
    env: &Env,
    crowdfund_id: u64,
    proposed_cap: i128,
    current_cap: i128,
    now: u64,
) {
    DynamicHardCapUpdatedEvent {
        crowdfund_id,
        proposed_cap,
        current_cap,
        now,
    }
    .publish(env);
}

#[contractevent]
pub struct HardCapVotedEvent {
    pub crowdfund_id: u64,
    pub voter: Address,
    pub proposed_cap: i128,
    pub now: u64,
}

pub fn publish_hard_cap_voted_event(
    env: &Env,
    crowdfund_id: u64,
    voter: Address,
    proposed_cap: i128,
    now: u64,
) {
    HardCapVotedEvent {
        crowdfund_id,
        voter,
        proposed_cap,
        now,
    }
    .publish(env);
}

#[contractevent]
pub struct HardCapVotingFinalizedEvent {
    pub crowdfund_id: u64,
    pub votes_for: u32,
    pub votes_against: u32,
    pub votes_passed: bool,
    pub current_cap: i128,
    pub now: u64,
}

pub fn publish_hard_cap_voting_finalized_event(
    env: &Env,
    crowdfund_id: u64,
    votes_for: u32,
    votes_against: u32,
    votes_passed: bool,
    current_cap: i128,
    now: u64,
) {
    HardCapVotingFinalizedEvent {
        crowdfund_id,
        votes_for,
        votes_against,
        votes_passed,
        current_cap,
        now,
    }
    .publish(env);
}

#[contractevent]
pub struct HardCapVotingInitiatedEvent {
    pub crowdfund_id: u64,
    pub proposed_cap: i128,
    pub voting_duration: u64,
    pub voting_end: u64,
    pub now: u64,
}

pub fn publish_hard_cap_voting_initiated_event(
    env: &Env,
    crowdfund_id: u64,
    proposed_cap: i128,
    voting_duration: u64,
    voting_end: u64,
    now: u64,
) {
    HardCapVotingInitiatedEvent {
        crowdfund_id,
        proposed_cap,
        voting_duration,
        voting_end,
        now,
    }
    .publish(env);
}

#[contractevent]
pub struct NftBatchMintedEvent {
    pub collection_id: u64,
    pub merchant_id: u64,
    pub count: u32,
    pub timestamp: u64,
}

pub fn publish_nft_batch_minted_event(
    env: &Env,
    collection_id: u64,
    merchant_id: u64,
    count: u32,
    timestamp: u64,
) {
    NftBatchMintedEvent {
        collection_id,
        merchant_id,
        count,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct NftBurnedEvent {
    pub nft_id: u64,
    pub collection_id: u64,
    pub owner: Address,
    pub timestamp: u64,
}

pub fn publish_nft_burned_event(
    env: &Env,
    nft_id: u64,
    collection_id: u64,
    owner: Address,
    timestamp: u64,
) {
    NftBurnedEvent {
        nft_id,
        collection_id,
        owner,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct NftCollectionCreatedEvent {
    pub id: u64,
    pub merchant_id: u64,
    pub merchant_addr: Address,
    pub name: String,
    pub base_uri: String,
    pub max_supply: u64,
    pub royalty_bps: u32,
    pub timestamp: u64,
}

pub fn publish_nft_collection_created_event(
    env: &Env,
    id: u64,
    merchant_id: u64,
    merchant_addr: Address,
    name: String,
    base_uri: String,
    max_supply: u64,
    royalty_bps: u32,
    timestamp: u64,
) {
    NftCollectionCreatedEvent {
        id,
        merchant_id,
        merchant_addr,
        name,
        base_uri,
        max_supply,
        royalty_bps,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct NftCollectionDeactivatedEvent {
    pub collection_id: u64,
    pub merchant_addr: Address,
    pub timestamp: u64,
}

pub fn publish_nft_collection_deactivated_event(
    env: &Env,
    collection_id: u64,
    merchant_addr: Address,
    timestamp: u64,
) {
    NftCollectionDeactivatedEvent {
        collection_id,
        merchant_addr,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct NftMintedEvent {
    pub nft_id: u64,
    pub collection_id: u64,
    pub merchant_id: u64,
    pub recipient: Address,
    pub token_uri: String,
    pub timestamp: u64,
}

pub fn publish_nft_minted_event(
    env: &Env,
    nft_id: u64,
    collection_id: u64,
    merchant_id: u64,
    recipient: Address,
    token_uri: String,
    timestamp: u64,
) {
    NftMintedEvent {
        nft_id,
        collection_id,
        merchant_id,
        recipient,
        token_uri,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct NftRewardClaimedEvent {
    pub nft_id: u64,
    pub collection_id: u64,
    pub claimer: Address,
    pub timestamp: u64,
}

pub fn publish_nft_reward_claimed_event(
    env: &Env,
    nft_id: u64,
    collection_id: u64,
    claimer: Address,
    timestamp: u64,
) {
    NftRewardClaimedEvent {
        nft_id,
        collection_id,
        claimer,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct NftTransferredEvent {
    pub nft_id: u64,
    pub collection_id: u64,
    pub from: Address,
    pub to: Address,
    pub timestamp: u64,
}

pub fn publish_nft_transferred_event(
    env: &Env,
    nft_id: u64,
    collection_id: u64,
    from: Address,
    to: Address,
    timestamp: u64,
) {
    NftTransferredEvent {
        nft_id,
        collection_id,
        from,
        to,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct PledgeMadeEvent {
    pub pledge_id: u64,
    pub campaign_id: u64,
    pub contributor: Address,
    pub amount: i128,
    pub token: Address,
    pub timestamp: u64,
}

pub fn publish_pledge_made_event(
    env: &Env,
    pledge_id: u64,
    campaign_id: u64,
    contributor: Address,
    amount: i128,
    token: Address,
    timestamp: u64,
) {
    PledgeMadeEvent {
        pledge_id,
        campaign_id,
        contributor,
        amount,
        token,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct PledgeRefundedEvent {
    pub pledge_id: u64,
    pub campaign_id: u64,
    pub contributor: Address,
    pub amount: i128,
    pub token: Address,
    pub timestamp: u64,
}

pub fn publish_pledge_refunded_event(
    env: &Env,
    pledge_id: u64,
    campaign_id: u64,
    contributor: Address,
    amount: i128,
    token: Address,
    timestamp: u64,
) {
    PledgeRefundedEvent {
        pledge_id,
        campaign_id,
        contributor,
        amount,
        token,
        timestamp,
    }
    .publish(env);
}

#[contractevent]
pub struct VestingScheduleReleasedEvent {
    pub timeline_id: u64,
    pub tranche_index: u64,
    pub unlock_amount: i128,
    pub now: u64,
}

pub fn publish_vesting_schedule_released_event(
    env: &Env,
    timeline_id: u64,
    tranche_index: u64,
    unlock_amount: i128,
    now: u64,
) {
    VestingScheduleReleasedEvent {
        timeline_id,
        tranche_index,
        unlock_amount,
        now,
    }
    .publish(env);
}

#[contractevent]
pub struct VestingTimelineCreatedEvent {
    pub timeline_id: u64,
    pub name: String,
    pub cliff_duration: u64,
    pub vesting_duration: u64,
    pub admin: Address,
    pub now: u64,
}

pub fn publish_vesting_timeline_created_event(
    env: &Env,
    timeline_id: u64,
    name: String,
    cliff_duration: u64,
    vesting_duration: u64,
    admin: Address,
    now: u64,
) {
    VestingTimelineCreatedEvent {
        timeline_id,
        name,
        cliff_duration,
        vesting_duration,
        admin,
        now,
    }
    .publish(env);
}

#[contractevent]
pub struct VestingTimelineUpdatedEvent {
    pub timeline_id: u64,
    pub cliff_duration: u64,
    pub vesting_duration: u64,
    pub admin: Address,
    pub now: u64,
}

pub fn publish_vesting_timeline_updated_event(
    env: &Env,
    timeline_id: u64,
    cliff_duration: u64,
    vesting_duration: u64,
    admin: Address,
    now: u64,
) {
    VestingTimelineUpdatedEvent {
        timeline_id,
        cliff_duration,
        vesting_duration,
        admin,
        now,
    }
    .publish(env);
}

/// Emitted when a campaign creator commits raised funds to a vesting schedule.
#[contractevent]
pub struct CreatorVestingCreatedEvent {
    pub campaign_id: u64,
    /// Beneficiary of the schedule; the campaign's owning merchant.
    pub creator: Address,
    pub token: Address,
    pub total_amount: i128,
    pub start_time: u64,
    /// Absolute timestamp of the cliff: `start_time + cliff_duration`.
    pub cliff_timestamp: u64,
    /// Absolute timestamp of full vesting: `start_time + vesting_duration`.
    pub end_timestamp: u64,
    pub initial_unlock_bps: u32,
    /// Tokens the cliff releases in one lump; the rest vests linearly after it.
    pub initial_unlock_amount: i128,
    /// The campaign's raise at creation time, for context on the commitment.
    pub campaign_raised: i128,
    pub timestamp: u64,
}

pub fn publish_creator_vesting_created_event(
    env: &Env,
    campaign_id: u64,
    creator: Address,
    token: Address,
    total_amount: i128,
    start_time: u64,
    cliff_timestamp: u64,
    end_timestamp: u64,
    initial_unlock_bps: u32,
    initial_unlock_amount: i128,
    campaign_raised: i128,
    timestamp: u64,
) {
    CreatorVestingCreatedEvent {
        campaign_id,
        creator,
        token,
        total_amount,
        start_time,
        cliff_timestamp,
        end_timestamp,
        initial_unlock_bps,
        initial_unlock_amount,
        campaign_raised,
        timestamp,
    }
    .publish(env);
}

/// Emitted on every payout from a creator's vesting schedule.
#[contractevent]
pub struct CreatorVestingReleasedEvent {
    pub campaign_id: u64,
    pub creator: Address,
    pub token: Address,
    /// Amount transferred by this release.
    pub amount: i128,
    /// Cumulative amount released across every payout so far.
    pub total_released: i128,
    /// Committed total that has vested as of this release.
    pub vested_to_date: i128,
    /// Still to be paid out: `total_amount - total_released`.
    pub remaining_amount: i128,
    /// True when this release drained the schedule.
    pub completed: bool,
    pub timestamp: u64,
}

pub fn publish_creator_vesting_released_event(
    env: &Env,
    campaign_id: u64,
    creator: Address,
    token: Address,
    amount: i128,
    total_released: i128,
    vested_to_date: i128,
    remaining_amount: i128,
    completed: bool,
    timestamp: u64,
) {
    CreatorVestingReleasedEvent {
        campaign_id,
        creator,
        token,
        amount,
        total_released,
        vested_to_date,
        remaining_amount,
        completed,
        timestamp,
    }
    .publish(env);
}

/// Emitted when the admin freezes a schedule. The vested-but-unreleased balance
/// stays claimable by the creator; `forfeited_amount` never vests.
#[contractevent]
pub struct CreatorVestingRevokedEvent {
    pub campaign_id: u64,
    pub creator: Address,
    /// Contract admin that revoked the schedule.
    pub admin: Address,
    /// Amount that had vested at revocation; the schedule's new total.
    pub vested_amount: i128,
    /// Vested but not yet paid out; the creator may still claim this.
    pub unreleased_amount: i128,
    /// Amount that will now never vest.
    pub forfeited_amount: i128,
    pub timestamp: u64,
}

pub fn publish_creator_vesting_revoked_event(
    env: &Env,
    campaign_id: u64,
    creator: Address,
    admin: Address,
    vested_amount: i128,
    unreleased_amount: i128,
    forfeited_amount: i128,
    timestamp: u64,
) {
    CreatorVestingRevokedEvent {
        campaign_id,
        creator,
        admin,
        vested_amount,
        unreleased_amount,
        forfeited_amount,
        timestamp,
    }
    .publish(env);
}

/// Emitted when a fee-policy / staking campaign is created (see [`crate::types::FeeCampaign`]).
#[contractevent]
pub struct FeeCampaignCreatedEvent {
    pub campaign_id: u64,
    pub owner: Address,
    pub name: String,
    pub charity: bool,
    pub fee_waiver_bps: u32,
    pub discount_bps: u32,
    pub timestamp: u64,
}

pub fn publish_fee_campaign_created_event(
    env: &Env,
    campaign_id: u64,
    owner: Address,
    name: String,
    charity: bool,
    fee_waiver_bps: u32,
    discount_bps: u32,
    timestamp: u64,
) {
    FeeCampaignCreatedEvent {
        campaign_id,
        owner,
        name,
        charity,
        fee_waiver_bps,
        discount_bps,
        timestamp,
    }
    .publish(env);
}

/// Emitted when an all-or-nothing pledge campaign is created
/// (see [`crate::types::PledgeCampaign`]).
#[contractevent]
pub struct PledgeCampaignCreatedEvent {
    pub campaign_id: u64,
    pub merchant: Address,
    pub merchant_id: u64,
    pub title: String,
    pub goal: i128,
    pub token: Address,
    pub deadline: u64,
    pub timestamp: u64,
}

pub fn publish_pledge_campaign_created_event(
    env: &Env,
    campaign_id: u64,
    merchant: Address,
    merchant_id: u64,
    title: String,
    goal: i128,
    token: Address,
    deadline: u64,
    timestamp: u64,
) {
    PledgeCampaignCreatedEvent {
        campaign_id,
        merchant,
        merchant_id,
        title,
        goal,
        token,
        deadline,
        timestamp,
    }
    .publish(env);
}

/// Emitted when a merchant pegs a campaign's funding target to a fiat currency
/// (see [`crate::types::CampaignFiatGoal`]).
///
/// Carries the oracle and the seed price alongside the target so an indexer can
/// render the campaign, and reproduce every later valuation, from this one event.
#[contractevent]
pub struct CampaignFiatGoalSetEvent {
    pub campaign_id: u64,
    pub merchant: Address,
    pub token: Address,
    pub currency: String,
    pub goal_amount: i128,
    pub decimals: u32,
    pub oracle: Address,
    pub price: i128,
    pub price_decimals: u32,
    /// Token base units the target is worth at `price`, for display only — the
    /// figure moves with the price and is never stored.
    pub token_goal_estimate: i128,
    pub deadline: u64,
    pub timestamp: u64,
}

pub fn publish_campaign_fiat_goal_set_event(
    env: &Env,
    campaign_id: u64,
    merchant: Address,
    token: Address,
    currency: String,
    goal_amount: i128,
    decimals: u32,
    oracle: Address,
    price: i128,
    price_decimals: u32,
    token_goal_estimate: i128,
    deadline: u64,
    timestamp: u64,
) {
    CampaignFiatGoalSetEvent {
        campaign_id,
        merchant,
        token,
        currency,
        goal_amount,
        decimals,
        oracle,
        price,
        price_decimals,
        token_goal_estimate,
        deadline,
        timestamp,
    }
    .publish(env);
}

/// Emitted for every contribution valued against a fiat-pegged goal.
///
/// `price` is the snapshot the contribution was credited at, so the pair
/// (`token_amount`, `fiat_amount`) is independently verifiable after the fact.
#[contractevent]
pub struct FiatContributionEvent {
    pub campaign_id: u64,
    pub contributor: Address,
    pub token: Address,
    pub token_amount: i128,
    pub fiat_amount: i128,
    pub currency: String,
    pub price: i128,
    pub price_decimals: u32,
    pub raised_amount: i128,
    pub goal_amount: i128,
    pub progress_bps: u32,
    pub timestamp: u64,
}

pub fn publish_fiat_contribution_event(
    env: &Env,
    campaign_id: u64,
    contributor: Address,
    token: Address,
    token_amount: i128,
    fiat_amount: i128,
    currency: String,
    price: i128,
    price_decimals: u32,
    raised_amount: i128,
    goal_amount: i128,
    progress_bps: u32,
    timestamp: u64,
) {
    FiatContributionEvent {
        campaign_id,
        contributor,
        token,
        token_amount,
        fiat_amount,
        currency,
        price,
        price_decimals,
        raised_amount,
        goal_amount,
        progress_bps,
        timestamp,
    }
    .publish(env);
}

/// Emitted once, on the contribution that first carries a fiat-pegged goal to
/// its target.
#[contractevent]
pub struct FiatGoalReachedEvent {
    pub campaign_id: u64,
    pub merchant: Address,
    pub currency: String,
    pub goal_amount: i128,
    pub raised_amount: i128,
    /// Token base units it took to get there. Differs from what the target was
    /// originally worth whenever the price moved during the raise.
    pub raised_tokens: i128,
    pub contribution_count: u32,
    pub timestamp: u64,
}

pub fn publish_fiat_goal_reached_event(
    env: &Env,
    campaign_id: u64,
    merchant: Address,
    currency: String,
    goal_amount: i128,
    raised_amount: i128,
    raised_tokens: i128,
    contribution_count: u32,
    timestamp: u64,
) {
    FiatGoalReachedEvent {
        campaign_id,
        merchant,
        currency,
        goal_amount,
        raised_amount,
        raised_tokens,
        contribution_count,
        timestamp,
    }
    .publish(env);
}

/// Emitted when the owning merchant re-reads the oracle to publish a fresh
/// valuation of a fiat-pegged goal, for indexers that track the shortfall
/// between contributions.
#[contractevent]
pub struct FiatGoalQuoteEvent {
    pub campaign_id: u64,
    pub token: Address,
    pub currency: String,
    pub price: i128,
    pub price_decimals: u32,
    pub raised_amount: i128,
    pub goal_amount: i128,
    pub remaining_amount: i128,
    pub tokens_required: i128,
    pub progress_bps: u32,
    pub timestamp: u64,
}

pub fn publish_fiat_goal_quote_event(
    env: &Env,
    campaign_id: u64,
    token: Address,
    currency: String,
    price: i128,
    price_decimals: u32,
    raised_amount: i128,
    goal_amount: i128,
    remaining_amount: i128,
    tokens_required: i128,
    progress_bps: u32,
    timestamp: u64,
) {
    FiatGoalQuoteEvent {
        campaign_id,
        token,
        currency,
        price,
        price_decimals,
        raised_amount,
        goal_amount,
        remaining_amount,
        tokens_required,
        progress_bps,
        timestamp,
    }
    .publish(env);
}

/// Emitted when a fiat-pegged goal is wound down. `goal_reached` records
/// whether it made its target, which is what decides fulfilment off-chain.
#[contractevent]
pub struct FiatGoalClosedEvent {
    pub campaign_id: u64,
    pub caller: Address,
    pub currency: String,
    pub goal_amount: i128,
    pub raised_amount: i128,
    pub raised_tokens: i128,
    pub progress_bps: u32,
    pub goal_reached: bool,
    pub timestamp: u64,
}

pub fn publish_fiat_goal_closed_event(
    env: &Env,
    campaign_id: u64,
    caller: Address,
    currency: String,
    goal_amount: i128,
    raised_amount: i128,
    raised_tokens: i128,
    progress_bps: u32,
    goal_reached: bool,
    timestamp: u64,
) {
    FiatGoalClosedEvent {
        campaign_id,
        caller,
        currency,
        goal_amount,
        raised_amount,
        raised_tokens,
        progress_bps,
        goal_reached,
        timestamp,
    }
    .publish(env);
}

/// Emitted on every contribution an analytics-tracked campaign receives.
///
/// Carries the running aggregate rather than just the contribution, so an
/// indexer can keep a campaign's dashboard current from this event alone
/// without replaying the whole pledge history or reading contract state.
#[contractevent]
pub struct CampaignStatsUpdatedEvent {
    pub campaign_id: u64,
    pub backer: Address,
    pub amount: i128,
    /// Whether this contribution came from an address that had never
    /// contributed before, which is what moved `backer_count`.
    pub is_new_backer: bool,
    pub pledge_count: u32,
    pub backer_count: u32,
    pub tracked_raised: i128,
    pub average_pledge: i128,
    pub largest_pledge: i128,
    pub smallest_pledge: i128,
    pub timestamp: u64,
}

pub fn publish_campaign_stats_updated_event(
    env: &Env,
    campaign_id: u64,
    backer: Address,
    amount: i128,
    is_new_backer: bool,
    stats: &CampaignStats,
    average_pledge: i128,
    timestamp: u64,
) {
    CampaignStatsUpdatedEvent {
        campaign_id,
        backer,
        amount,
        is_new_backer,
        pledge_count: stats.pledge_count,
        backer_count: stats.backer_count,
        tracked_raised: stats.tracked_raised,
        average_pledge,
        largest_pledge: stats.largest_pledge,
        smallest_pledge: stats.smallest_pledge,
        timestamp,
    }
    .publish(env);
}

/// Emitted when a creator exports their campaign's analytics.
///
/// Mirrors the stored [`AnalyticsExport`] field for field: the record is the
/// export, and the event is how off-chain tooling picks it up without polling.
/// It carries the full structural context an indexer needs to render a file —
/// which campaign and creator, the requested `format`, the export's position in
/// its series, the window it covers, the cumulative figures, and the delta
/// since the previous export — so no follow-up contract read is required.
#[contractevent]
pub struct AnalyticsExportEvent {
    pub export_id: u64,
    pub campaign_id: u64,
    pub creator: Address,
    pub merchant_id: u64,
    pub token: Address,
    pub format: ExportFormat,
    pub sequence: u32,
    pub period_start: u64,
    pub period_end: u64,
    pub campaign_raised: i128,
    pub campaign_deadline: u64,
    pub campaign_active: bool,
    pub total_raised: i128,
    pub pledge_count: u32,
    pub backer_count: u32,
    pub average_pledge: i128,
    pub largest_pledge: i128,
    pub smallest_pledge: i128,
    pub first_pledge_at: u64,
    pub last_pledge_at: u64,
    pub period_raised: i128,
    pub period_pledges: u32,
    pub period_backers: u32,
    pub timestamp: u64,
}

/// Takes the export record itself rather than two dozen positional arguments:
/// at this field count a mistyped call site would silently transpose figures
/// that no compiler check would catch.
pub fn publish_analytics_export_event(env: &Env, export: &AnalyticsExport) {
    AnalyticsExportEvent {
        export_id: export.id,
        campaign_id: export.campaign_id,
        creator: export.creator.clone(),
        merchant_id: export.merchant_id,
        token: export.token.clone(),
        format: export.format,
        sequence: export.sequence,
        period_start: export.period_start,
        period_end: export.period_end,
        campaign_raised: export.campaign_raised,
        campaign_deadline: export.campaign_deadline,
        campaign_active: export.campaign_active,
        total_raised: export.total_raised,
        pledge_count: export.pledge_count,
        backer_count: export.backer_count,
        average_pledge: export.average_pledge,
        largest_pledge: export.largest_pledge,
        smallest_pledge: export.smallest_pledge,
        first_pledge_at: export.first_pledge_at,
        last_pledge_at: export.last_pledge_at,
        period_raised: export.period_raised,
        period_pledges: export.period_pledges,
        period_backers: export.period_backers,
        timestamp: export.created_at,
    }
    .publish(env);
}
