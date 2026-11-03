#![no_std]

mod errors;
#[cfg(test)]
mod tests;

use errors::CrowdfundError;
use soroban_sdk::{
    contract, contractclient, contractevent, contractimpl, contracttype, panic_with_error, token,
    vec, Address, BytesN, Env, String, Vec,
};

#[allow(dead_code)]
#[contractclient(name = "InvoicePaymentClient")]
trait InvoicePayment {
    fn pay_invoice(env: Env, payer: Address, invoice_id: u64);
}

#[allow(dead_code)]
#[contractclient(name = "MerchantAccountRefundClient")]
trait MerchantAccountRefund {
    fn refund(env: Env, token: Address, amount: i128, to: Address);
}

#[contractevent]
pub struct CampaignExecutedEvent {
    pub amount: i128,
}

#[contractevent]
pub struct RefundClaimedEvent {
    pub contributor: Address,
    pub amount: i128,
}

#[contractevent]
pub struct StretchGoalReachedEvent {
    pub milestone_index: u32,
    pub threshold: i128,
}

#[contractevent]
pub struct DiscountAppliedEvent {
    pub contributor: Address,
    pub original_amount: i128,
    pub discounted_amount: i128,
    pub discount_bps: u32,
}

#[contractevent]
pub struct PricingWindowSetEvent {
    pub tier_index: u32,
    pub start: u64,
    pub end: u64,
    pub discount_bps: u32,
}

#[contractevent]
pub struct RewardFulfilledEvent {
    pub backer: Address,
}

#[contracttype]
#[derive(Clone)]
pub struct RewardTier {
    pub min_pledge: i128,
    pub name: String,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscountTier {
    pub start: u64,
    pub end: u64,
    pub discount_bps: u32,
}

#[contractevent]
pub struct RewardTierSelectedEvent {
    pub contributor: Address,
    pub tier_index: u32,
}

#[contractevent]
pub struct MilestoneUnlockedEvent {
    pub index: u32,
}

#[contractevent]
pub struct MilestoneReleasedEvent {
    pub index: u32,
    pub amount: i128,
}

#[contractevent]
pub struct MilestoneVoteCastEvent {
    pub index: u32,
    pub voter: Address,
    pub approve: bool,
    pub weight: i128,
}

#[contractevent]
pub struct MatchingPoolFundedEvent {
    pub sponsor: Address,
    pub amount: i128,
}

#[contractevent]
pub struct MatchAppliedEvent {
    pub contributor: Address,
    pub matched_amount: i128,
}

#[contractevent]
pub struct PledgeCommentAddedEvent {
    pub contributor: Address,
    pub comment: String,
}

// ── Affiliate / referral events (#349) ───────────────────────────────────────

#[contractevent]
pub struct AffiliateRegisteredEvent {
    pub affiliate: Address,
    pub code_hash: BytesN<32>,
    pub commission_bps: u32,
}

#[contractevent]
pub struct ReferralUsedEvent {
    pub contributor: Address,
    pub affiliate: Address,
    pub code_hash: BytesN<32>,
    pub contribution_amount: i128,
    pub commission_amount: i128,
}

#[contractevent]
pub struct PledgeReceivedEvent {
    pub contributor: Address,
    pub amount: i128,
}

#[contractevent]
pub struct AffiliateAccruedEvent {
    pub affiliate: Address,
    pub contributor: Address,
    pub commission: i128,
}

#[contractevent]
pub struct AffiliateClaimedEvent {
    pub affiliate: Address,
    pub amount: i128,
}

#[contractevent]
pub struct BatchRefundProcessedEvent {
    pub total_refunded: i128,
    pub contributor_count: u32,
}

// ── Social recovery / guardian events ────────────────────────────────────────

/// Emitted when the organizer (re)configures the guardian set. Carries the
/// guardian count and threshold rather than the addresses so the event stays a
/// fixed size regardless of set size; read `get_guardians` for the members.
#[contractevent]
pub struct GuardiansSetEvent {
    pub organizer: Address,
    pub guardian_count: u32,
    pub threshold: u32,
}

/// Emitted when a guardian nominates a new organizer, opening a recovery.
#[contractevent]
pub struct RecoveryInitiatedEvent {
    pub initiator: Address,
    pub nominee: Address,
}

/// Emitted on each guardian approval, carrying progress toward the threshold so
/// an indexer can show "2 of 3 approved" without extra reads.
#[contractevent]
pub struct RecoveryApprovedEvent {
    pub guardian: Address,
    pub approvals: u32,
    pub threshold: u32,
}

/// Emitted when the approval threshold is met and the organizer is replaced.
#[contractevent]
pub struct RecoveryExecutedEvent {
    pub old_organizer: Address,
    pub new_organizer: Address,
}

/// Emitted when the sitting organizer cancels a pending recovery.
#[contractevent]
pub struct RecoveryCancelledEvent {
    pub organizer: Address,
    pub nominee: Address,
}

// ── KYC gating events ────────────────────────────────────────────────────────

/// Emitted when the organizer turns the KYC contribution gate on or off.
#[contractevent]
pub struct KycRequirementSetEvent {
    pub organizer: Address,
    pub required: bool,
}

/// Emitted when the organizer records or revokes a contributor's verification.
#[contractevent]
pub struct KycVerificationSetEvent {
    pub organizer: Address,
    pub contributor: Address,
    pub verified: bool,
}

/// Highly-detailed on-chain checkpoint of a campaign's aggregate statistics,
/// emitted by the organizer to drive off-chain indexing and UI dashboards.
#[contractevent]
pub struct CampaignStatsSnapshotEvent {
    pub caller: Address,
    pub goal: i128,
    pub raised: i128,
    pub total_matched: i128,
    pub matching_pool_balance: i128,
    pub contributor_count: u32,
    pub largest_pledge: i128,
    pub percent_funded_bps: u32,
    pub goal_reached: bool,
    pub is_ended: bool,
    pub timestamp: u64,
}

/// Aggregate, read-only statistics describing the current state of a campaign.
/// All amounts are in token base units; `percent_funded_bps` is in basis
/// points (10_000 = 100 %) and is saturated to `u32::MAX` for extreme
/// overfunding. `seconds_remaining` is 0 once the deadline has passed.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CampaignStats {
    pub goal: i128,
    pub raised: i128,
    pub total_matched: i128,
    pub matching_pool_balance: i128,
    pub contributor_count: u32,
    pub average_pledge: i128,
    pub largest_pledge: i128,
    pub largest_backer: Option<Address>,
    pub percent_funded_bps: u32,
    pub deadline: u64,
    pub seconds_remaining: u64,
    pub is_ended: bool,
    pub goal_reached: bool,
    pub executed: bool,
}

/// Gamification achievement a backer can earn. Each kind has on-chain
/// eligibility rules verified at award time (see `assert_badge_eligible`).
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum BadgeKind {
    /// The very first backer to pledge to the campaign.
    FirstBacker = 0,
    /// A backer within the first `EarlyBackerLimit` contributors.
    EarlyBacker = 1,
    /// A backer whose total pledge meets the `WhaleThreshold`.
    Whale = 2,
    /// A backer of a campaign that reached its funding goal.
    GoalGetter = 3,
}

/// Emitted whenever a backer earns a badge — carries full structural metadata
/// for off-chain indexing and UI achievement feeds.
#[contractevent]
pub struct BadgeAwardedEvent {
    pub backer: Address,
    pub kind: BadgeKind,
    pub awarded_by: Address,
    pub awarded_at: u64,
    pub badge_count: u32,
}

/// Emitted when the organizer (re)configures badge eligibility thresholds.
#[contractevent]
pub struct BadgeConfigSetEvent {
    pub organizer: Address,
    pub whale_threshold: i128,
    pub early_backer_limit: u32,
}

#[contracttype]
enum DataKey {
    Organizer,
    Token,
    Goal,
    Deadline,
    Raised,
    Executed,
    Pledge(Address),
    StretchGoals,
    StretchTriggered(u32),
    RewardFulfilled(Address),
    RewardTiers,
    SelectedTier(Address),
    MilestonePercentages,
    MilestoneUnlocked(u32),
    MilestoneReleased(u32),
    MilestoneApprovalWeight(u32),
    MilestoneRejectionWeight(u32),
    MilestoneVote(u32, Address),
    ShadeGateway,
    MerchantId,
    MerchantAccount,
    Contributors,
    RefundProcessed,
    MatchingPool,
    TotalMatched,
    Badge(Address, u32),
    BadgeCount(Address),
    EarlyBackerLimit,
    WhaleThreshold,
    DiscountTiers,
    // Public comment attached to a contributor pledge.
    // ── Affiliate / referral tracking (#349) ────────────────────────────────
    // Commission rate (in basis points) paid to affiliates from the raised pool.
    AffiliateCommissionBps,
    // Referral code (hash) → affiliate address.
    ReferralCodeOwner(BytesN<32>),
    // Affiliate address → referral code (hash).
    AffiliateCode(Address),
    // Per-code count of unique contributors who used the code.
    ReferralCount(BytesN<32>),
    // Cumulative commission earned by the affiliate for a given code.
    ReferralEarnings(BytesN<32>),
    // Tracks which referral code (if any) a contributor used.
    PledgeReferral(Address),
    PledgeComment(Address),
    DiscountTiers,
    DiscountApplied(Address),
    // ── Sponsor matching ────────────────────────────────────────────────────
    // Cumulative amount paid out of the sponsor matching pool.
    TotalMatched,
    // ── Badges / gamification ───────────────────────────────────────────────
    // Awarded badge → ledger timestamp it was earned at.
    Badge(Address, BadgeKind),
    // Number of distinct badges a backer holds.
    BadgeCount(Address),
    // Minimum total pledge that qualifies for the `Whale` badge.
    WhaleThreshold,
    // How many of the first contributors qualify for the `EarlyBacker` badge.
    EarlyBackerLimit,
    // ── Social recovery / guardians ─────────────────────────────────────────
    // Addresses authorised to recover the organizer account.
    Guardians,
    // Number of guardian approvals required to execute a recovery.
    GuardianThreshold,
    // Organizer address nominated by the pending recovery, if any. Its presence
    // is the "a recovery is pending" flag.
    RecoveryNominee,
    // Whether a given guardian has approved the pending recovery.
    RecoveryApproval(Address),
    // Approvals collected for the pending recovery.
    RecoveryApprovalCount,
    // ── KYC gating ──────────────────────────────────────────────────────────
    // When true, contributors must be KYC-verified before pledging.
    KycRequired,
    // Whether a specific contributor has been marked KYC-verified.
    KycVerified(Address),
}

#[contract]
pub struct CrowdfundContract;

#[contractimpl]
impl CrowdfundContract {
    const MAX_COMMENT_BYTES: u32 = 280;

    pub fn init_campaign(env: Env, organizer: Address, token: Address, goal: i128, deadline: u64) {
        if env.storage().persistent().has(&DataKey::Organizer) {
            panic_with_error!(&env, CrowdfundError::AlreadyInitialized);
        }
        if goal <= 0 {
            panic_with_error!(&env, CrowdfundError::InvalidGoal);
        }
        if deadline <= env.ledger().timestamp() {
            panic_with_error!(&env, CrowdfundError::InvalidDeadline);
        }

        env.storage()
            .persistent()
            .set(&DataKey::Organizer, &organizer);
        env.storage().persistent().set(&DataKey::Token, &token);
        env.storage().persistent().set(&DataKey::Goal, &goal);
        env.storage()
            .persistent()
            .set(&DataKey::Deadline, &deadline);
        env.storage().persistent().set(&DataKey::Raised, &0_i128);
        env.storage().persistent().set(&DataKey::Executed, &false);
        env.storage()
            .persistent()
            .set(&DataKey::RefundProcessed, &false);
        env.storage()
            .persistent()
            .set(&DataKey::Contributors, &Vec::<Address>::new(&env));
    }

    pub fn set_shade_gateway(env: Env, shade_gateway: Address) {
        let organizer: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Organizer)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));
        organizer.require_auth();
        if env.storage().persistent().has(&DataKey::ShadeGateway) {
            panic_with_error!(&env, CrowdfundError::AlreadyInitialized);
        }
        env.storage()
            .persistent()
            .set(&DataKey::ShadeGateway, &shade_gateway);
    }

    pub fn set_merchant_id(env: Env, merchant_id: u64) {
        let organizer: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Organizer)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));
        organizer.require_auth();
        if env.storage().persistent().has(&DataKey::MerchantId) {
            panic_with_error!(&env, CrowdfundError::AlreadyInitialized);
        }
        env.storage()
            .persistent()
            .set(&DataKey::MerchantId, &merchant_id);
    }

    pub fn set_merchant_account(env: Env, merchant_account: Address) {
        let organizer: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Organizer)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));
        organizer.require_auth();
        if env.storage().persistent().has(&DataKey::MerchantAccount) {
            panic_with_error!(&env, CrowdfundError::AlreadyInitialized);
        }
        env.storage()
            .persistent()
            .set(&DataKey::MerchantAccount, &merchant_account);
    }

    pub fn pledge(env: Env, contributor: Address, amount: i128, invoice_id: u64) {
        contributor.require_auth();
        if amount <= 0 {
            panic_with_error!(&env, CrowdfundError::InvalidAmount);
        }

        let deadline: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::Deadline)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));
        if env.ledger().timestamp() > deadline {
            panic_with_error!(&env, CrowdfundError::CampaignEnded);
        }
        if env
            .storage()
            .persistent()
            .get(&DataKey::Executed)
            .unwrap_or(false)
        {
            panic_with_error!(&env, CrowdfundError::AlreadyExecuted);
        }

        // Check KYC requirements
        if Self::is_kyc_required(env.clone())
            && !Self::is_kyc_verified(env.clone(), contributor.clone())
        {
            panic_with_error!(&env, CrowdfundError::KYCRequired);
        }

        let shade_gateway: Address = env
            .storage()
            .persistent()
            .get(&DataKey::ShadeGateway)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::ShadeGatewayNotSet));

        let discount_bps: u32 = {
            let now = env.ledger().timestamp();
            if let Some(tiers) = env
                .storage()
                .persistent()
                .get::<_, Vec<DiscountTier>>(&DataKey::DiscountTiers)
            {
                let mut disc = 0u32;
                for tier in tiers.iter() {
                    if now >= tier.start && now <= tier.end {
                        disc = tier.discount_bps;
                        break;
                    }
                }
                disc
            } else {
                0u32
            }
        };

        let discounted_amount = amount * (10_000i128 - discount_bps as i128) / 10_000i128;

        let token_addr: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Token)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));

        InvoicePaymentClient::new(&env, &shade_gateway).pay_invoice(&contributor, &invoice_id);

        let merchant_account: Address = env
            .storage()
            .persistent()
            .get(&DataKey::MerchantAccount)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::MerchantAccountNotSet));
        MerchantAccountRefundClient::new(&env, &merchant_account).refund(
            &token_addr,
            &amount,
            &env.current_contract_address(),
        );

        let new_raised =
            Self::apply_pledge_with_matching(&env, contributor.clone(), discounted_amount);

        let prev: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Pledge(contributor.clone()))
            .unwrap_or(0);
        env.storage().persistent().set(
            &DataKey::Pledge(contributor.clone()),
            &prev.saturating_add(amount),
        );

        DiscountAppliedEvent {
            contributor: contributor.clone(),
            original_amount: amount,
            discounted_amount,
            discount_bps,
        }
        .publish(&env);

        Self::track_contributor(&env, contributor.clone());
        Self::check_stretch_goals(&env, new_raised);
        PledgeReceivedEvent {
            contributor,
            amount,
        }
        .publish(&env);
    }

    pub fn contribute(env: Env, contributor: Address, amount: i128) {
        contributor.require_auth();

        if amount <= 0 {
            panic_with_error!(&env, CrowdfundError::InvalidAmount);
        }

        let deadline: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::Deadline)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));

        if env.ledger().timestamp() > deadline {
            panic_with_error!(&env, CrowdfundError::CampaignEnded);
        }

        // Check KYC requirements
        if Self::is_kyc_required(env.clone())
            && !Self::is_kyc_verified(env.clone(), contributor.clone())
        {
            panic_with_error!(&env, CrowdfundError::KYCRequired);
        }

        let token_addr: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Token)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));

        let contract_addr = env.current_contract_address();
        token::TokenClient::new(&env, &token_addr).transfer(&contributor, &contract_addr, &amount);

        let new_raised = Self::apply_pledge_with_matching(&env, contributor.clone(), amount);

        Self::track_contributor(&env, contributor);
        Self::check_stretch_goals(&env, new_raised);
    }

    pub fn fund_matching_pool(env: Env, sponsor: Address, amount: i128) {
        sponsor.require_auth();
        if amount <= 0 {
            panic_with_error!(&env, CrowdfundError::InvalidAmount);
        }

        let token_addr: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Token)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));
        let contract_addr = env.current_contract_address();
        token::TokenClient::new(&env, &token_addr).transfer(&sponsor, &contract_addr, &amount);

        let current: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::MatchingPool)
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&DataKey::MatchingPool, &current.saturating_add(amount));
        MatchingPoolFundedEvent { sponsor, amount }.publish(&env);
    }

    pub fn leave_comment(env: Env, contributor: Address, comment: String) {
        contributor.require_auth();
        let pledge: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Pledge(contributor.clone()))
            .unwrap_or(0);
        if pledge <= 0 {
            panic_with_error!(&env, CrowdfundError::NoPledge);
        }
        if comment.len() > Self::MAX_COMMENT_BYTES {
            panic_with_error!(&env, CrowdfundError::CommentTooLong);
        }

        env.storage()
            .persistent()
            .set(&DataKey::PledgeComment(contributor.clone()), &comment);
        PledgeCommentAddedEvent {
            contributor,
            comment,
        }
        .publish(&env);
    }

    pub fn get_comment(env: Env, contributor: Address) -> Option<String> {
        env.storage()
            .persistent()
            .get(&DataKey::PledgeComment(contributor))
    }

    pub fn matching_pool_balance(env: Env) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::MatchingPool)
            .unwrap_or(0)
    }

    // ── Affiliate / referral tracking (#349) ────────────────────────────────

    /// Set the campaign-wide commission rate that affiliates earn on
    /// contributions made through their referral links. Must be called by the
    /// organizer. `commission_bps` is in basis points (max 10 000 = 100 %).
    pub fn set_affiliate_commission(env: Env, commission_bps: u32) {
        let organizer: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Organizer)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));
        organizer.require_auth();

        if commission_bps > 10_000 {
            panic_with_error!(&env, CrowdfundError::InvalidCommissionBps);
        }

        env.storage()
            .persistent()
            .set(&DataKey::AffiliateCommissionBps, &commission_bps);
    }

    /// Register `affiliate` with a unique referral `code_hash`.
    ///
    /// - Only the organizer may add affiliates.
    /// - Each `code_hash` can only be owned by one affiliate.
    /// - An affiliate address can hold only one code; re-registering the same
    ///   affiliate with a new code replaces the old mapping.
    pub fn register_affiliate(env: Env, affiliate: Address, code_hash: BytesN<32>) {
        let organizer: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Organizer)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));
        organizer.require_auth();

        // Ensure the code is not already owned by a *different* affiliate.
        if let Some(existing_owner) = env
            .storage()
            .persistent()
            .get::<_, Address>(&DataKey::ReferralCodeOwner(code_hash.clone()))
        {
            if existing_owner != affiliate {
                panic_with_error!(&env, CrowdfundError::ReferralCodeAlreadyTaken);
            }
        }

        let commission_bps: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::AffiliateCommissionBps)
            .unwrap_or(0);

        env.storage()
            .persistent()
            .set(&DataKey::ReferralCodeOwner(code_hash.clone()), &affiliate);
        env.storage()
            .persistent()
            .set(&DataKey::AffiliateCode(affiliate.clone()), &code_hash);

        AffiliateRegisteredEvent {
            affiliate,
            code_hash,
            commission_bps,
        }
        .publish(&env);
    }

    /// Contribute `amount` tokens through a referral link identified by
    /// `code_hash`. Behaves like `contribute` but additionally:
    ///
    /// 1. Validates the referral code and that the contributor has not already
    ///    used a code for this campaign.
    /// 2. Pays the affiliate's commission (based on `AffiliateCommissionBps`)
    ///    from the campaign pool immediately after the contribution is recorded.
    /// 3. Increments the referral count and cumulative earnings for that code.
    pub fn contribute_with_referral(
        env: Env,
        contributor: Address,
        amount: i128,
        code_hash: BytesN<32>,
    ) {
        contributor.require_auth();

        if amount <= 0 {
            panic_with_error!(&env, CrowdfundError::InvalidAmount);
        }

        let deadline: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::Deadline)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));

        if env.ledger().timestamp() > deadline {
            panic_with_error!(&env, CrowdfundError::CampaignEnded);
        }

        // Validate referral code.
        let affiliate: Address = env
            .storage()
            .persistent()
            .get::<_, Address>(&DataKey::ReferralCodeOwner(code_hash.clone()))
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::ReferralCodeNotFound));

        // Each contributor may use at most one referral code per campaign.
        if env
            .storage()
            .persistent()
            .has(&DataKey::PledgeReferral(contributor.clone()))
        {
            panic_with_error!(&env, CrowdfundError::ReferralAlreadyUsed);
        }

        let token_addr: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Token)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));

        let contract_addr = env.current_contract_address();
        token::TokenClient::new(&env, &token_addr).transfer(&contributor, &contract_addr, &amount);

        let new_raised = Self::apply_pledge_with_matching(&env, contributor.clone(), amount);

        // Record that this contributor used this code.
        env.storage()
            .persistent()
            .set(&DataKey::PledgeReferral(contributor.clone()), &code_hash);

        // Increment referral count for this code.
        let count: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::ReferralCount(code_hash.clone()))
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&DataKey::ReferralCount(code_hash.clone()), &(count + 1));

        // Calculate and immediately pay commission from campaign balance.
        let commission_bps: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::AffiliateCommissionBps)
            .unwrap_or(0);

        let commission_amount: i128 = if commission_bps > 0 {
            amount * (commission_bps as i128) / 10_000
        } else {
            0
        };

        if commission_amount > 0 {
            token::TokenClient::new(&env, &token_addr).transfer(
                &contract_addr,
                &affiliate,
                &commission_amount,
            );

            // Reduce the raised counter so the commission is not counted
            // as funds the organizer can withdraw.
            let current_raised: i128 = env
                .storage()
                .persistent()
                .get(&DataKey::Raised)
                .unwrap_or(0);
            env.storage().persistent().set(
                &DataKey::Raised,
                &current_raised.saturating_sub(commission_amount),
            );

            // Accumulate affiliate earnings.
            let prev_earnings: i128 = env
                .storage()
                .persistent()
                .get(&DataKey::ReferralEarnings(code_hash.clone()))
                .unwrap_or(0);
            env.storage().persistent().set(
                &DataKey::ReferralEarnings(code_hash.clone()),
                &prev_earnings.saturating_add(commission_amount),
            );
        }

        ReferralUsedEvent {
            contributor: contributor.clone(),
            affiliate,
            code_hash,
            contribution_amount: amount,
            commission_amount,
        }
        .publish(&env);

        Self::track_contributor(&env, contributor);
        Self::check_stretch_goals(&env, new_raised);
    }

    /// Returns the number of unique contributors who used the given referral code.
    pub fn get_referral_count(env: Env, code_hash: BytesN<32>) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::ReferralCount(code_hash))
            .unwrap_or(0)
    }

    /// Returns the total commission (in token base units) earned by the
    /// affiliate who owns the given referral code.
    pub fn get_referral_earnings(env: Env, code_hash: BytesN<32>) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::ReferralEarnings(code_hash))
            .unwrap_or(0)
    }

    /// Returns the referral code hash registered for the given affiliate address,
    /// or `None` if the address has not been registered as an affiliate.
    pub fn get_referral_code(env: Env, affiliate: Address) -> Option<BytesN<32>> {
        env.storage()
            .persistent()
            .get(&DataKey::AffiliateCode(affiliate))
    }

    /// Returns the referral code hash used by a contributor, or `None` if the
    /// contributor did not use a referral code for this campaign.
    pub fn get_contributor_referral(env: Env, contributor: Address) -> Option<BytesN<32>> {
        env.storage()
            .persistent()
            .get(&DataKey::PledgeReferral(contributor))
    }

    /// Returns the affiliate commission rate configured for this campaign, in
    /// basis points (0 if not set).
    pub fn get_affiliate_commission_bps(env: Env) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::AffiliateCommissionBps)
            .unwrap_or(0)
    }

    /// Withdraw funds to the organizer after deadline if goal was met (#303).
    pub fn execute_campaign(env: Env) {
        let organizer: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Organizer)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));

        organizer.require_auth();

        let deadline: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::Deadline)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));

        if env.ledger().timestamp() <= deadline {
            panic_with_error!(&env, CrowdfundError::CampaignNotEnded);
        }

        let goal: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Goal)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));

        let raised: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Raised)
            .unwrap_or(0);

        if raised < goal {
            panic_with_error!(&env, CrowdfundError::GoalNotReached);
        }

        let executed: bool = env
            .storage()
            .persistent()
            .get(&DataKey::Executed)
            .unwrap_or(false);

        if executed {
            panic_with_error!(&env, CrowdfundError::AlreadyExecuted);
        }

        // Milestone mode: use release_milestone instead.
        if env
            .storage()
            .persistent()
            .has(&DataKey::MilestonePercentages)
        {
            panic_with_error!(&env, CrowdfundError::MilestonesActive);
        }

        env.storage().persistent().set(&DataKey::Executed, &true);

        let token_addr: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Token)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));

        let contract_addr = env.current_contract_address();
        token::TokenClient::new(&env, &token_addr).transfer(&contract_addr, &organizer, &raised);

        CampaignExecutedEvent { amount: raised }.publish(&env);
    }

    pub fn claim_refund(env: Env, contributor: Address) {
        contributor.require_auth();

        let deadline: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::Deadline)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));

        if env.ledger().timestamp() <= deadline {
            panic_with_error!(&env, CrowdfundError::CampaignNotEnded);
        }

        let goal: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Goal)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));

        let raised: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Raised)
            .unwrap_or(0);

        if raised >= goal {
            panic_with_error!(&env, CrowdfundError::GoalReached);
        }

        let pledge: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Pledge(contributor.clone()))
            .unwrap_or(0);

        if pledge == 0 {
            panic_with_error!(&env, CrowdfundError::NoPledge);
        }

        env.storage()
            .persistent()
            .set(&DataKey::Pledge(contributor.clone()), &0_i128);

        let token_addr: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Token)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));

        let contract_addr = env.current_contract_address();
        token::TokenClient::new(&env, &token_addr).transfer(&contract_addr, &contributor, &pledge);

        RefundClaimedEvent {
            contributor: contributor.clone(),
            amount: pledge,
        }
        .publish(&env);
    }

    pub fn batch_refund(env: Env) {
        let deadline: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::Deadline)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));
        if env.ledger().timestamp() <= deadline {
            panic_with_error!(&env, CrowdfundError::CampaignNotEnded);
        }

        let goal: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Goal)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));
        let raised: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Raised)
            .unwrap_or(0);
        if raised >= goal {
            panic_with_error!(&env, CrowdfundError::GoalReached);
        }

        if env
            .storage()
            .persistent()
            .get(&DataKey::RefundProcessed)
            .unwrap_or(false)
        {
            panic_with_error!(&env, CrowdfundError::RefundAlreadyProcessed);
        }
        env.storage()
            .persistent()
            .set(&DataKey::RefundProcessed, &true);

        let token_addr: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Token)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));
        let token_client = token::TokenClient::new(&env, &token_addr);
        let contract_addr = env.current_contract_address();

        let contributors: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Contributors)
            .unwrap_or_else(|| Vec::new(&env));
        let count = contributors.len();
        let mut total_refunded: i128 = 0;

        for contributor in contributors.iter() {
            let pledge: i128 = env
                .storage()
                .persistent()
                .get(&DataKey::Pledge(contributor.clone()))
                .unwrap_or(0);
            if pledge > 0 {
                env.storage()
                    .persistent()
                    .set(&DataKey::Pledge(contributor.clone()), &0_i128);
                token_client.transfer(&contract_addr, &contributor, &pledge);
                total_refunded = total_refunded.saturating_add(pledge);
            }
        }

        BatchRefundProcessedEvent {
            total_refunded,
            contributor_count: count,
        }
        .publish(&env);
    }

    pub fn set_stretch_goals(env: Env, milestones: Vec<i128>) {
        let organizer: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Organizer)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));

        organizer.require_auth();

        let mut prev = 0_i128;
        for m in milestones.iter() {
            if m <= prev {
                panic_with_error!(&env, CrowdfundError::InvalidGoal);
            }
            prev = m;
        }

        env.storage()
            .persistent()
            .set(&DataKey::StretchGoals, &milestones);

        // No `StretchGoalReachedEvent` here: configuring a milestone is not
        // reaching it. `check_stretch_goals` emits that once `raised` actually
        // crosses each threshold.
    }

    pub fn fulfill_reward(env: Env, backer: Address) {
        let organizer: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Organizer)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));

        organizer.require_auth();

        if env
            .storage()
            .persistent()
            .get(&DataKey::RewardFulfilled(backer.clone()))
            .unwrap_or(false)
        {
            panic_with_error!(&env, CrowdfundError::AlreadyFulfilled);
        }

        env.storage()
            .persistent()
            .set(&DataKey::RewardFulfilled(backer.clone()), &true);

        RewardFulfilledEvent { backer }.publish(&env);
    }

    pub fn is_fulfilled(env: Env, backer: Address) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::RewardFulfilled(backer))
            .unwrap_or(false)
    }

    pub fn set_reward_tiers(env: Env, tiers: Vec<RewardTier>) {
        let organizer: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Organizer)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));

        organizer.require_auth();

        let mut prev = 0_i128;
        for tier in tiers.iter() {
            if tier.min_pledge <= prev {
                panic_with_error!(&env, CrowdfundError::InvalidGoal);
            }
            prev = tier.min_pledge;
        }

        env.storage()
            .persistent()
            .set(&DataKey::RewardTiers, &tiers);
    }

    pub fn select_reward_tier(env: Env, contributor: Address, tier_index: u32) {
        contributor.require_auth();

        let tiers: Vec<RewardTier> = env
            .storage()
            .persistent()
            .get(&DataKey::RewardTiers)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));

        let tier = tiers
            .get(tier_index)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::InvalidTier));

        let pledge: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Pledge(contributor.clone()))
            .unwrap_or(0);

        if pledge < tier.min_pledge {
            panic_with_error!(&env, CrowdfundError::PledgeBelowTierMinimum);
        }

        env.storage()
            .persistent()
            .set(&DataKey::SelectedTier(contributor.clone()), &tier_index);

        RewardTierSelectedEvent {
            contributor,
            tier_index,
        }
        .publish(&env);
    }

    pub fn get_selected_tier(env: Env, contributor: Address) -> Option<u32> {
        env.storage()
            .persistent()
            .get(&DataKey::SelectedTier(contributor))
    }

    pub fn set_milestones(env: Env, percentages: Vec<u32>) {
        let organizer: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Organizer)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));

        organizer.require_auth();

        let mut sum: u32 = 0;
        for p in percentages.iter() {
            if p == 0 {
                panic_with_error!(&env, CrowdfundError::InvalidMilestonePercentages);
            }
            sum = sum.saturating_add(p);
        }
        if sum != 10_000 || percentages.is_empty() {
            panic_with_error!(&env, CrowdfundError::InvalidMilestonePercentages);
        }

        env.storage()
            .persistent()
            .set(&DataKey::MilestonePercentages, &percentages);
    }

    pub fn unlock_milestone(env: Env, index: u32) {
        let organizer: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Organizer)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));

        organizer.require_auth();

        let percentages: Vec<u32> = env
            .storage()
            .persistent()
            .get(&DataKey::MilestonePercentages)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::MilestonesNotSet));

        if index >= percentages.len() {
            panic_with_error!(&env, CrowdfundError::InvalidMilestonePercentages);
        }

        env.storage()
            .persistent()
            .set(&DataKey::MilestoneUnlocked(index), &true);

        MilestoneUnlockedEvent { index }.publish(&env);
    }

    pub fn vote_milestone(env: Env, voter: Address, index: u32, approve: bool) {
        voter.require_auth();

        let percentages: Vec<u32> = env
            .storage()
            .persistent()
            .get(&DataKey::MilestonePercentages)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::MilestonesNotSet));

        if index >= percentages.len() {
            panic_with_error!(&env, CrowdfundError::InvalidMilestonePercentages);
        }

        let weight: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Pledge(voter.clone()))
            .unwrap_or(0);

        if weight <= 0 {
            panic_with_error!(&env, CrowdfundError::NotBacker);
        }

        let vote_key = DataKey::MilestoneVote(index, voter.clone());
        if env.storage().persistent().has(&vote_key) {
            panic_with_error!(&env, CrowdfundError::MilestoneVoteAlreadyCast);
        }

        let tally_key = if approve {
            DataKey::MilestoneApprovalWeight(index)
        } else {
            DataKey::MilestoneRejectionWeight(index)
        };

        let current: i128 = env.storage().persistent().get(&tally_key).unwrap_or(0);
        env.storage()
            .persistent()
            .set(&tally_key, &current.saturating_add(weight));
        env.storage().persistent().set(&vote_key, &approve);

        MilestoneVoteCastEvent {
            index,
            voter,
            approve,
            weight,
        }
        .publish(&env);
    }

    /// Release the proportional funds for an unlocked, unreleased milestone to the organizer.
    /// Can only be called after the campaign deadline and goal is met.
    pub fn release_milestone(env: Env, index: u32) {
        let organizer: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Organizer)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));

        organizer.require_auth();

        let deadline: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::Deadline)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));

        if env.ledger().timestamp() <= deadline {
            panic_with_error!(&env, CrowdfundError::CampaignNotEnded);
        }

        let goal: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Goal)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));

        let raised: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Raised)
            .unwrap_or(0);

        if raised < goal {
            panic_with_error!(&env, CrowdfundError::GoalNotReached);
        }

        let percentages: Vec<u32> = env
            .storage()
            .persistent()
            .get(&DataKey::MilestonePercentages)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::MilestonesNotSet));

        if index >= percentages.len() {
            panic_with_error!(&env, CrowdfundError::InvalidMilestonePercentages);
        }

        let unlocked: bool = env
            .storage()
            .persistent()
            .get(&DataKey::MilestoneUnlocked(index))
            .unwrap_or(false);

        if !unlocked {
            panic_with_error!(&env, CrowdfundError::MilestoneNotUnlocked);
        }

        let released: bool = env
            .storage()
            .persistent()
            .get(&DataKey::MilestoneReleased(index))
            .unwrap_or(false);

        if released {
            panic_with_error!(&env, CrowdfundError::MilestoneAlreadyReleased);
        }

        let approval_weight: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::MilestoneApprovalWeight(index))
            .unwrap_or(0);

        if approval_weight <= raised / 2 {
            panic_with_error!(&env, CrowdfundError::MilestoneNotApproved);
        }

        let bps = percentages.get(index).unwrap() as i128;
        let amount = raised * bps / 10_000;

        env.storage()
            .persistent()
            .set(&DataKey::MilestoneReleased(index), &true);

        let token_addr: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Token)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));

        let contract_addr = env.current_contract_address();
        token::TokenClient::new(&env, &token_addr).transfer(&contract_addr, &organizer, &amount);

        MilestoneReleasedEvent { index, amount }.publish(&env);
    }

    /// Returns the pledge amount recorded for a given contributor.
    pub fn pledge_of(env: Env, contributor: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Pledge(contributor))
            .unwrap_or(0)
    }

    // ── Social recovery (#366) ────────────────────────────────────────────────

    /// Configure (or replace) the guardian set and approval threshold used for
    /// social recovery of the organizer account. Organizer-only. Replacing the
    /// guardian set while a recovery is pending is rejected to avoid a
    /// guardian being silently dropped mid-vote.
    pub fn set_guardians(env: Env, organizer: Address, guardians: Vec<Address>, threshold: u32) {
        let stored_organizer: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Organizer)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));

        if organizer != stored_organizer {
            panic_with_error!(&env, CrowdfundError::NotInitialized);
        }
        organizer.require_auth();

        if env.storage().persistent().has(&DataKey::RecoveryNominee) {
            panic_with_error!(&env, CrowdfundError::RecoveryAlreadyPending);
        }

        if threshold == 0 || threshold > guardians.len() {
            panic_with_error!(&env, CrowdfundError::InvalidThreshold);
        }

        for i in 0..guardians.len() {
            let g = guardians.get(i).unwrap();
            for j in (i + 1)..guardians.len() {
                if g == guardians.get(j).unwrap() {
                    panic_with_error!(&env, CrowdfundError::DuplicateGuardian);
                }
            }
        }

        env.storage()
            .persistent()
            .set(&DataKey::Guardians, &guardians);
        env.storage()
            .persistent()
            .set(&DataKey::GuardianThreshold, &threshold);

        GuardiansSetEvent {
            organizer,
            guardian_count: guardians.len(),
            threshold,
        }
        .publish(&env);
    }

    /// A guardian nominates a new organizer address and casts the first
    /// approval. Fails if a recovery is already pending. If the threshold is
    /// 1, this immediately executes the recovery.
    pub fn initiate_recovery(env: Env, guardian: Address, new_organizer: Address) {
        guardian.require_auth();
        Self::require_guardian(&env, &guardian);

        if env.storage().persistent().has(&DataKey::RecoveryNominee) {
            panic_with_error!(&env, CrowdfundError::RecoveryAlreadyPending);
        }

        env.storage()
            .persistent()
            .set(&DataKey::RecoveryNominee, &new_organizer);

        RecoveryInitiatedEvent {
            initiator: guardian.clone(),
            nominee: new_organizer,
        }
        .publish(&env);

        Self::record_approval_and_maybe_execute(&env, guardian);
    }

    /// A guardian approves the pending recovery. Once the configured
    /// threshold of approvals is reached, the recovery executes immediately
    /// and the nominee becomes the new organizer.
    pub fn approve_recovery(env: Env, guardian: Address) {
        guardian.require_auth();
        Self::require_guardian(&env, &guardian);

        if !env.storage().persistent().has(&DataKey::RecoveryNominee) {
            panic_with_error!(&env, CrowdfundError::NoPendingRecovery);
        }

        Self::record_approval_and_maybe_execute(&env, guardian);
    }

    fn record_approval_and_maybe_execute(env: &Env, guardian: Address) {
        let approval_key = DataKey::RecoveryApproval(guardian.clone());
        if env
            .storage()
            .persistent()
            .get(&approval_key)
            .unwrap_or(false)
        {
            panic_with_error!(env, CrowdfundError::AlreadyApprovedRecovery);
        }
        env.storage().persistent().set(&approval_key, &true);

        let threshold: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::GuardianThreshold)
            .unwrap_or_else(|| panic_with_error!(env, CrowdfundError::GuardiansNotSet));

        let approvals: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::RecoveryApprovalCount)
            .unwrap_or(0)
            + 1;
        env.storage()
            .persistent()
            .set(&DataKey::RecoveryApprovalCount, &approvals);

        RecoveryApprovedEvent {
            guardian,
            approvals,
            threshold,
        }
        .publish(env);

        if approvals >= threshold {
            let nominee: Address = env
                .storage()
                .persistent()
                .get(&DataKey::RecoveryNominee)
                .unwrap_or_else(|| panic_with_error!(env, CrowdfundError::NoPendingRecovery));
            let old_organizer: Address = env
                .storage()
                .persistent()
                .get(&DataKey::Organizer)
                .unwrap_or_else(|| panic_with_error!(env, CrowdfundError::NotInitialized));

            env.storage()
                .persistent()
                .set(&DataKey::Organizer, &nominee);

            Self::clear_pending_recovery(env);

            RecoveryExecutedEvent {
                old_organizer,
                new_organizer: nominee,
            }
            .publish(env);
        }
    }

    /// The current organizer can cancel a pending recovery (e.g. if it was
    /// not actually compromised). Requires organizer auth so a quorum of
    /// guardians cannot be silently overridden by anyone else.
    pub fn cancel_recovery(env: Env, organizer: Address) {
        let stored_organizer: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Organizer)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));

        if organizer != stored_organizer {
            panic_with_error!(&env, CrowdfundError::NotInitialized);
        }
        organizer.require_auth();

        let nominee: Address = env
            .storage()
            .persistent()
            .get(&DataKey::RecoveryNominee)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NoPendingRecovery));

        Self::clear_pending_recovery(&env);

        RecoveryCancelledEvent { organizer, nominee }.publish(&env);
    }

    pub fn get_guardians(env: Env) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::Guardians)
            .unwrap_or_else(|| Vec::new(&env))
    }

    pub fn get_guardian_threshold(env: Env) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::GuardianThreshold)
            .unwrap_or(0)
    }

    pub fn is_guardian(env: Env, address: Address) -> bool {
        let guardians: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Guardians)
            .unwrap_or_else(|| Vec::new(&env));
        guardians.iter().any(|g| g == address)
    }

    pub fn get_pending_recovery(env: Env) -> Option<Address> {
        env.storage().persistent().get(&DataKey::RecoveryNominee)
    }

    pub fn get_recovery_approval_count(env: Env) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::RecoveryApprovalCount)
            .unwrap_or(0)
    }

    fn require_guardian(env: &Env, guardian: &Address) {
        let guardians: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Guardians)
            .unwrap_or_else(|| panic_with_error!(env, CrowdfundError::GuardiansNotSet));
        if !guardians.iter().any(|g| &g == guardian) {
            panic_with_error!(env, CrowdfundError::NotGuardian);
        }
    }

    fn clear_pending_recovery(env: &Env) {
        let guardians: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Guardians)
            .unwrap_or_else(|| Vec::new(env));
        for guardian in guardians.iter() {
            env.storage()
                .persistent()
                .remove(&DataKey::RecoveryApproval(guardian));
        }
        env.storage().persistent().remove(&DataKey::RecoveryNominee);
        env.storage()
            .persistent()
            .remove(&DataKey::RecoveryApprovalCount);
    }

    // ── Read-only accessors ───────────────────────────────────────────────────

    pub fn goal(env: Env) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Goal)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized))
    }

    pub fn deadline(env: Env) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::Deadline)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized))
    }

    pub fn raised(env: Env) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Raised)
            .unwrap_or(0)
    }

    pub fn organizer(env: Env) -> Address {
        env.storage()
            .persistent()
            .get(&DataKey::Organizer)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized))
    }

    pub fn is_executed(env: Env) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::Executed)
            .unwrap_or(false)
    }

    /// Returns `true` when the raised amount has reached or exceeded the goal.
    pub fn goal_reached(env: Env) -> bool {
        let goal: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Goal)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));
        let raised: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Raised)
            .unwrap_or(0);
        raised >= goal
    }

    // ── Deep campaign statistics (read-only views) ────────────────────────────

    /// Public, read-only aggregate statistics for the campaign.
    ///
    /// Safe to call by anyone (no auth) and free of state mutation, so it is
    /// concurrency-safe: concurrent callers observe a consistent snapshot of
    /// the persisted ledger state. Panics with `NotInitialized` if no campaign
    /// has been created.
    pub fn get_campaign_stats(env: Env) -> CampaignStats {
        Self::compute_stats(&env)
    }

    /// Organizer-only ranked list of backers by total pledge (descending),
    /// truncated to `limit` entries. Exposes per-backer amounts, so it requires
    /// the organizer's authorization (role-based check).
    pub fn get_backer_leaderboard(env: Env, caller: Address, limit: u32) -> Vec<(Address, i128)> {
        Self::require_organizer(&env, &caller);

        let contributors: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Contributors)
            .unwrap_or_else(|| Vec::new(&env));

        // Collect (backer, pledge) pairs with a positive pledge.
        let mut pairs: Vec<(Address, i128)> = Vec::new(&env);
        for c in contributors.iter() {
            let pledge: i128 = env
                .storage()
                .persistent()
                .get(&DataKey::Pledge(c.clone()))
                .unwrap_or(0);
            if pledge > 0 {
                pairs.push_back((c, pledge));
            }
        }

        // Partial selection sort: pull the top `limit` pledges to the front.
        let n = pairs.len();
        let take = if limit < n { limit } else { n };
        let mut result: Vec<(Address, i128)> = Vec::new(&env);
        for i in 0..take {
            let mut max_idx = i;
            let mut max_amt = pairs.get(i).unwrap().1;
            for j in (i + 1)..n {
                let amt = pairs.get(j).unwrap().1;
                if amt > max_amt {
                    max_amt = amt;
                    max_idx = j;
                }
            }
            if max_idx != i {
                let a = pairs.get(i).unwrap();
                let b = pairs.get(max_idx).unwrap();
                pairs.set(i, b);
                pairs.set(max_idx, a);
            }
            result.push_back(pairs.get(i).unwrap());
        }
        result
    }

    // ── KYC gating ───────────────────────────────────────────────────────────
    //
    // `contribute` and `contribute_with_referral` gate on these two predicates.
    // KYC is opt-in: `is_kyc_required` defaults to false, so the gate is inert
    // until the organizer turns it on.

    /// Organizer-only. When enabled, contributors must be KYC-verified before
    /// they can pledge.
    pub fn set_kyc_required(env: Env, organizer: Address, required: bool) {
        Self::require_organizer(&env, &organizer);
        env.storage()
            .persistent()
            .set(&DataKey::KycRequired, &required);
        KycRequirementSetEvent {
            organizer,
            required,
        }
        .publish(&env);
    }

    /// Whether this campaign gates contributions on KYC verification.
    pub fn is_kyc_required(env: Env) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::KycRequired)
            .unwrap_or(false)
    }

    /// Organizer-only. Record (or revoke) a contributor's KYC verification.
    /// The organizer is trusted to have performed verification off-chain; only
    /// the resulting decision is stored on-chain.
    pub fn set_kyc_verified(env: Env, organizer: Address, contributor: Address, verified: bool) {
        Self::require_organizer(&env, &organizer);
        env.storage()
            .persistent()
            .set(&DataKey::KycVerified(contributor.clone()), &verified);
        KycVerificationSetEvent {
            organizer,
            contributor,
            verified,
        }
        .publish(&env);
    }

    /// Whether `contributor` has been marked KYC-verified on this campaign.
    pub fn is_kyc_verified(env: Env, contributor: Address) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::KycVerified(contributor))
            .unwrap_or(false)
    }

    /// Organizer-only action that publishes a detailed `CampaignStatsSnapshotEvent`
    /// for off-chain indexers/UIs and returns the current statistics. The call
    /// reads (does not alter) campaign state, so it is safe under concurrency.
    pub fn snapshot_campaign_stats(env: Env, caller: Address) -> CampaignStats {
        Self::require_organizer(&env, &caller);
        let stats = Self::compute_stats(&env);
        CampaignStatsSnapshotEvent {
            caller,
            goal: stats.goal,
            raised: stats.raised,
            total_matched: stats.total_matched,
            matching_pool_balance: stats.matching_pool_balance,
            contributor_count: stats.contributor_count,
            largest_pledge: stats.largest_pledge,
            percent_funded_bps: stats.percent_funded_bps,
            goal_reached: stats.goal_reached,
            is_ended: stats.is_ended,
            timestamp: env.ledger().timestamp(),
        }
        .publish(&env);
        stats
    }

    // ── Gamification: badges & achievements ───────────────────────────────────

    /// Configure (or update) badge eligibility thresholds. Organizer-only.
    /// `whale_threshold` is the minimum total pledge for the Whale badge;
    /// `early_backer_limit` is how many of the first contributors qualify for
    /// the EarlyBacker badge.
    pub fn set_badge_config(
        env: Env,
        organizer: Address,
        whale_threshold: i128,
        early_backer_limit: u32,
    ) {
        Self::require_organizer(&env, &organizer);
        if whale_threshold <= 0 {
            panic_with_error!(&env, CrowdfundError::InvalidAmount);
        }
        env.storage()
            .persistent()
            .set(&DataKey::WhaleThreshold, &whale_threshold);
        env.storage()
            .persistent()
            .set(&DataKey::EarlyBackerLimit, &early_backer_limit);
        BadgeConfigSetEvent {
            organizer,
            whale_threshold,
            early_backer_limit,
        }
        .publish(&env);
    }

    /// Award `kind` to `backer` after verifying the badge's on-chain eligibility
    /// rules. Callable by the backer themselves (self-claim) or by the organizer
    /// (role-based check). Idempotent guard: a backer cannot earn the same badge
    /// twice, so repeated/concurrent calls converge safely.
    pub fn award_badge(env: Env, caller: Address, backer: Address, kind: BadgeKind) {
        caller.require_auth();

        let organizer: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Organizer)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));
        if caller != backer && caller != organizer {
            panic_with_error!(&env, CrowdfundError::NotAuthorized);
        }

        if env
            .storage()
            .persistent()
            .has(&DataKey::Badge(backer.clone(), kind))
        {
            panic_with_error!(&env, CrowdfundError::BadgeAlreadyAwarded);
        }

        Self::assert_badge_eligible(&env, &backer, kind);

        let now = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&DataKey::Badge(backer.clone(), kind), &now);

        let prev_count: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::BadgeCount(backer.clone()))
            .unwrap_or(0);
        let badge_count = prev_count.saturating_add(1);
        env.storage()
            .persistent()
            .set(&DataKey::BadgeCount(backer.clone()), &badge_count);

        BadgeAwardedEvent {
            backer,
            kind,
            awarded_by: caller,
            awarded_at: now,
            badge_count,
        }
        .publish(&env);
    }

    /// Whether `backer` holds the given badge.
    pub fn has_badge(env: Env, backer: Address, kind: BadgeKind) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::Badge(backer, kind))
    }

    /// The ledger timestamp at which `backer` earned `kind`, if owned.
    pub fn badge_awarded_at(env: Env, backer: Address, kind: BadgeKind) -> Option<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::Badge(backer, kind))
    }

    /// Number of distinct badges `backer` has earned.
    pub fn badge_count(env: Env, backer: Address) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::BadgeCount(backer))
            .unwrap_or(0)
    }

    /// The full set of badges `backer` currently holds.
    pub fn get_backer_badges(env: Env, backer: Address) -> Vec<BadgeKind> {
        let mut owned: Vec<BadgeKind> = Vec::new(&env);
        for kind in [
            BadgeKind::FirstBacker,
            BadgeKind::EarlyBacker,
            BadgeKind::Whale,
            BadgeKind::GoalGetter,
        ] {
            if env
                .storage()
                .persistent()
                .has(&DataKey::Badge(backer.clone(), kind))
            {
                owned.push_back(kind);
            }
        }
        owned
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    /// Verify that `backer` satisfies the eligibility rules for `kind`,
    /// panicking with `BadgeNotEligible` (or `BadgeConfigNotSet`) otherwise.
    fn assert_badge_eligible(env: &Env, backer: &Address, kind: BadgeKind) {
        // Every badge requires the holder to be an actual backer.
        let pledge: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Pledge(backer.clone()))
            .unwrap_or(0);
        if pledge <= 0 {
            panic_with_error!(env, CrowdfundError::BadgeNotEligible);
        }

        match kind {
            BadgeKind::FirstBacker => {
                let contributors: Vec<Address> = env
                    .storage()
                    .persistent()
                    .get(&DataKey::Contributors)
                    .unwrap_or_else(|| Vec::new(env));
                let is_first = matches!(contributors.first(), Some(first) if first == *backer);
                if !is_first {
                    panic_with_error!(env, CrowdfundError::BadgeNotEligible);
                }
            }
            BadgeKind::EarlyBacker => {
                let limit: u32 = env
                    .storage()
                    .persistent()
                    .get(&DataKey::EarlyBackerLimit)
                    .unwrap_or_else(|| panic_with_error!(env, CrowdfundError::BadgeConfigNotSet));
                let contributors: Vec<Address> = env
                    .storage()
                    .persistent()
                    .get(&DataKey::Contributors)
                    .unwrap_or_else(|| Vec::new(env));
                let mut eligible = false;
                for (i, c) in contributors.iter().enumerate() {
                    if c == *backer {
                        eligible = (i as u32) < limit;
                        break;
                    }
                }
                if !eligible {
                    panic_with_error!(env, CrowdfundError::BadgeNotEligible);
                }
            }
            BadgeKind::Whale => {
                let threshold: i128 = env
                    .storage()
                    .persistent()
                    .get(&DataKey::WhaleThreshold)
                    .unwrap_or_else(|| panic_with_error!(env, CrowdfundError::BadgeConfigNotSet));
                if pledge < threshold {
                    panic_with_error!(env, CrowdfundError::BadgeNotEligible);
                }
            }
            BadgeKind::GoalGetter => {
                let goal: i128 = env
                    .storage()
                    .persistent()
                    .get(&DataKey::Goal)
                    .unwrap_or_else(|| panic_with_error!(env, CrowdfundError::NotInitialized));
                let raised: i128 = env
                    .storage()
                    .persistent()
                    .get(&DataKey::Raised)
                    .unwrap_or(0);
                if raised < goal {
                    panic_with_error!(env, CrowdfundError::BadgeNotEligible);
                }
            }
        }
    }

    /// Require that `caller` is the campaign organizer (authenticated).
    fn require_organizer(env: &Env, caller: &Address) {
        caller.require_auth();
        let organizer: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Organizer)
            .unwrap_or_else(|| panic_with_error!(env, CrowdfundError::NotInitialized));
        if *caller != organizer {
            panic_with_error!(env, CrowdfundError::NotAuthorized);
        }
    }

    /// Build the aggregate `CampaignStats` from persisted state.
    fn compute_stats(env: &Env) -> CampaignStats {
        let goal: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Goal)
            .unwrap_or_else(|| panic_with_error!(env, CrowdfundError::NotInitialized));
        let deadline: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::Deadline)
            .unwrap_or_else(|| panic_with_error!(env, CrowdfundError::NotInitialized));

        let raised: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Raised)
            .unwrap_or(0);
        let total_matched: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::TotalMatched)
            .unwrap_or(0);
        let matching_pool_balance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::MatchingPool)
            .unwrap_or(0);
        let executed: bool = env
            .storage()
            .persistent()
            .get(&DataKey::Executed)
            .unwrap_or(false);

        let contributors: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Contributors)
            .unwrap_or_else(|| Vec::new(env));
        let contributor_count = contributors.len();

        // Largest pledge and its backer (read-only scan; accurate regardless of
        // how individual pledges were accumulated).
        let mut largest_pledge: i128 = 0;
        let mut largest_backer: Option<Address> = None;
        for c in contributors.iter() {
            let pledge: i128 = env
                .storage()
                .persistent()
                .get(&DataKey::Pledge(c.clone()))
                .unwrap_or(0);
            if pledge > largest_pledge {
                largest_pledge = pledge;
                largest_backer = Some(c);
            }
        }

        let average_pledge = if contributor_count > 0 {
            raised / contributor_count as i128
        } else {
            0
        };

        let percent_funded_bps = if goal > 0 {
            let bps = raised.saturating_mul(10_000) / goal;
            bps.clamp(0, u32::MAX as i128) as u32
        } else {
            0
        };

        let now = env.ledger().timestamp();
        let is_ended = now > deadline;
        let seconds_remaining = if is_ended { 0 } else { deadline - now };
        let goal_reached = raised >= goal;

        CampaignStats {
            goal,
            raised,
            total_matched,
            matching_pool_balance,
            contributor_count,
            average_pledge,
            largest_pledge,
            largest_backer,
            percent_funded_bps,
            deadline,
            seconds_remaining,
            is_ended,
            goal_reached,
            executed,
        }
    }

    /// Emit a `stretch / reached` event for each milestone crossed by `new_raised`
    /// that has not already been triggered.
    fn track_contributor(env: &Env, contributor: Address) {
        let mut contributors: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Contributors)
            .unwrap_or_else(|| Vec::new(env));
        for c in contributors.iter() {
            if c == contributor {
                return;
            }
        }
        contributors.push_back(contributor);
        env.storage()
            .persistent()
            .set(&DataKey::Contributors, &contributors);
    }

    fn check_stretch_goals(env: &Env, new_raised: i128) {
        let milestones: Vec<i128> = env
            .storage()
            .persistent()
            .get(&DataKey::StretchGoals)
            .unwrap_or_else(|| vec![env]);

        for (idx, threshold) in milestones.iter().enumerate() {
            let idx_u32 = idx as u32;
            let already: bool = env
                .storage()
                .persistent()
                .get(&DataKey::StretchTriggered(idx_u32))
                .unwrap_or(false);

            if !already && new_raised >= threshold {
                env.storage()
                    .persistent()
                    .set(&DataKey::StretchTriggered(idx_u32), &true);
                StretchGoalReachedEvent {
                    milestone_index: idx_u32,
                    threshold,
                }
                .publish(env);
            }
        }
    }

    fn apply_pledge_with_matching(env: &Env, contributor: Address, amount: i128) -> i128 {
        let matching_pool: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::MatchingPool)
            .unwrap_or(0);
        let matched_amount = if matching_pool >= amount {
            amount
        } else {
            matching_pool
        };
        if matched_amount > 0 {
            env.storage().persistent().set(
                &DataKey::MatchingPool,
                &matching_pool.saturating_sub(matched_amount),
            );
            let total_matched: i128 = env
                .storage()
                .persistent()
                .get(&DataKey::TotalMatched)
                .unwrap_or(0);
            env.storage().persistent().set(
                &DataKey::TotalMatched,
                &total_matched.saturating_add(matched_amount),
            );
            MatchAppliedEvent {
                contributor: contributor.clone(),
                matched_amount,
            }
            .publish(env);
        }

        let effective_amount = amount.saturating_add(matched_amount);
        let raised: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Raised)
            .unwrap_or(0);
        let new_raised = raised.saturating_add(effective_amount);
        env.storage()
            .persistent()
            .set(&DataKey::Raised, &new_raised);

        let prev_pledge: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Pledge(contributor.clone()))
            .unwrap_or(0);
        env.storage().persistent().set(
            &DataKey::Pledge(contributor),
            &prev_pledge.saturating_add(effective_amount),
        );

        new_raised
    }
}
