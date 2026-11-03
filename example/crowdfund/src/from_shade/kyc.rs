//! Campaign KYC & Verification System (#324)
//!
//! Provides:
//! - KYC reviewer role management (admin only)
//! - Subject verification request lifecycle (submit → approve/reject → suspend)
//! - Per-campaign KYC configuration for creators
//! - Backer KYC enforcement helpers
//!
//! Security model
//! ──────────────
//! • `admin`-gated: grant/revoke reviewer roles, suspend verified subjects.
//! • `reviewer`-gated: approve/reject individual KYC requests, verify campaigns.
//! • `subject`-gated: submit own KYC request.
//! • Reentrancy guards on every state-mutating function.

use shade::components::core;
use shade::components::reentrancy;
use crate::from_shade::errors::ContractError;
use crate::from_shade::events;
use crate::from_shade::types::{BackerKycStatus, CampaignKycStatus, DataKey, KycRequest, VerificationStatus, VerificationType};
use soroban_sdk::{panic_with_error, Address, Env, String};

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Return the current KYC request counter (0 when no requests have been made).
fn get_request_count(env: &Env) -> u64 {
    env.storage()
        .persistent()
        .get(&DataKey::KycRequestCount)
        .unwrap_or(0)
}

/// Persist an incremented request counter and return the new ID.
fn next_request_id(env: &Env) -> u64 {
    let id = get_request_count(env) + 1;
    env.storage()
        .persistent()
        .set(&DataKey::KycRequestCount, &id);
    id
}

/// Load a KYC request or panic with `NotFound`.
fn load_request(env: &Env, request_id: u64) -> KycRequest {
    env.storage()
        .persistent()
        .get(&DataKey::KycRequest(request_id))
        .unwrap_or_else(|| panic_with_error!(env, ContractError::KycRequestNotFound))
}

/// Assert that `caller` is a registered KYC reviewer or the contract admin.
fn assert_reviewer(env: &Env, caller: &Address) {
    let admin = core::get_admin(env);
    if *caller == admin {
        return;
    }
    let is_reviewer: bool = env
        .storage()
        .persistent()
        .get(&DataKey::KycReviewer(caller.clone()))
        .unwrap_or(false);
    if !is_reviewer {
        panic_with_error!(env, shade::errors::ContractError::NotAuthorized);
    }
}

// ── Reviewer role management ─────────────────────────────────────────────────

/// Grant the KYC reviewer role to `reviewer`. Admin-only.
pub fn grant_kyc_reviewer(env: &Env, admin: &Address, reviewer: &Address) {
    reentrancy::enter(env);
    core::assert_admin(env, admin);

    env.storage()
        .persistent()
        .set(&DataKey::KycReviewer(reviewer.clone()), &true);

    events::publish_kyc_reviewer_granted_event(
        env,
        admin.clone(),
        reviewer.clone(),
        env.ledger().timestamp(),
    );
    reentrancy::exit(env);
}

/// Revoke the KYC reviewer role from `reviewer`. Admin-only.
pub fn revoke_kyc_reviewer(env: &Env, admin: &Address, reviewer: &Address) {
    reentrancy::enter(env);
    core::assert_admin(env, admin);

    env.storage()
        .persistent()
        .remove(&DataKey::KycReviewer(reviewer.clone()));

    events::publish_kyc_reviewer_revoked_event(
        env,
        admin.clone(),
        reviewer.clone(),
        env.ledger().timestamp(),
    );
    reentrancy::exit(env);
}

/// Return `true` if `address` is a registered KYC reviewer (or the admin).
pub fn is_kyc_reviewer(env: &Env, address: &Address) -> bool {
    if *address == core::get_admin(env) {
        return true;
    }
    env.storage()
        .persistent()
        .get(&DataKey::KycReviewer(address.clone()))
        .unwrap_or(false)
}

// ── KYC request lifecycle ─────────────────────────────────────────────────────

/// Submit a KYC verification request on behalf of `subject`.
///
/// The subject must call this themselves (`require_auth`).  A subject that
/// already holds an `Approved` status cannot resubmit unless they have been
/// `Suspended` or `Rejected` first.
pub fn submit_kyc_request(
    env: &Env,
    subject: &Address,
    verification_type: VerificationType,
    document_count: u32,
    metadata: &String,
) -> u64 {
    reentrancy::enter(env);
    subject.require_auth();

    // Prevent spam: do not allow a new submission when one is already Pending.
    let current_status: VerificationStatus = env
        .storage()
        .persistent()
        .get(&DataKey::KycStatus(subject.clone()))
        .unwrap_or(VerificationStatus::Unverified);

    if current_status == VerificationStatus::Pending {
        panic_with_error!(env, ContractError::KycRequestAlreadyPending);
    }
    if current_status == VerificationStatus::Approved {
        panic_with_error!(env, ContractError::KycAlreadyApproved);
    }

    let request_id = next_request_id(env);
    let request = KycRequest {
        id: request_id,
        subject: subject.clone(),
        verification_type,
        submitted_at: env.ledger().timestamp(),
        reviewed_at: 0,
        reviewer: None,
        status: VerificationStatus::Pending,
        document_count,
        metadata: metadata.clone(),
    };

    env.storage()
        .persistent()
        .set(&DataKey::KycRequest(request_id), &request);
    env.storage()
        .persistent()
        .set(&DataKey::KycStatus(subject.clone()), &VerificationStatus::Pending);

    events::publish_kyc_request_submitted_event(
        env,
        request_id,
        subject.clone(),
        verification_type,
        env.ledger().timestamp(),
    );

    reentrancy::exit(env);
    request_id
}

/// Approve a pending KYC request.
///
/// `expiration_days`: how many days from now the approval is valid.
/// Pass `0` for no expiration (approval valid indefinitely).
pub fn approve_kyc_request(
    env: &Env,
    reviewer: &Address,
    request_id: u64,
    expiration_days: u64,
) {
    reentrancy::enter(env);
    reviewer.require_auth();
    assert_reviewer(env, reviewer);

    let mut request = load_request(env, request_id);
    if request.status != VerificationStatus::Pending {
        panic_with_error!(env, ContractError::InvalidKycRequestStatus);
    }

    let now = env.ledger().timestamp();
    let expiration = if expiration_days == 0 {
        0_u64 // 0 == no expiration
    } else {
        now.saturating_add(expiration_days.saturating_mul(86_400))
    };

    request.status = VerificationStatus::Approved;
    request.reviewed_at = now;
    request.reviewer = Some(reviewer.clone());

    env.storage()
        .persistent()
        .set(&DataKey::KycRequest(request_id), &request);
    env.storage()
        .persistent()
        .set(&DataKey::KycStatus(request.subject.clone()), &VerificationStatus::Approved);
    env.storage()
        .persistent()
        .set(&DataKey::KycApprovedRequestId(request.subject.clone()), &request_id);
    env.storage()
        .persistent()
        .set(&DataKey::KycExpiration(request.subject.clone()), &expiration);

    events::publish_kyc_request_approved_event(
        env,
        request_id,
        request.subject.clone(),
        reviewer.clone(),
        expiration,
        env.ledger().timestamp(),
    );

    reentrancy::exit(env);
}

/// Reject a pending KYC request with an optional reason string.
pub fn reject_kyc_request(
    env: &Env,
    reviewer: &Address,
    request_id: u64,
    reason: &String,
) {
    reentrancy::enter(env);
    reviewer.require_auth();
    assert_reviewer(env, reviewer);

    let mut request = load_request(env, request_id);
    if request.status != VerificationStatus::Pending {
        panic_with_error!(env, ContractError::InvalidKycRequestStatus);
    }

    request.status = VerificationStatus::Rejected;
    request.reviewed_at = env.ledger().timestamp();
    request.reviewer = Some(reviewer.clone());

    env.storage()
        .persistent()
        .set(&DataKey::KycRequest(request_id), &request);
    env.storage()
        .persistent()
        .set(&DataKey::KycStatus(request.subject.clone()), &VerificationStatus::Rejected);

    events::publish_kyc_request_rejected_event(
        env,
        request_id,
        request.subject.clone(),
        reviewer.clone(),
        reason.clone(),
        env.ledger().timestamp(),
    );

    reentrancy::exit(env);
}

/// Suspend the KYC approval of a previously approved subject.
///
/// This is an admin-level compliance action (e.g. sanctions screening).
pub fn suspend_kyc(
    env: &Env,
    admin: &Address,
    subject: &Address,
    reason: &String,
) {
    reentrancy::enter(env);
    core::assert_admin(env, admin);

    let current: VerificationStatus = env
        .storage()
        .persistent()
        .get(&DataKey::KycStatus(subject.clone()))
        .unwrap_or(VerificationStatus::Unverified);

    if current != VerificationStatus::Approved {
        panic_with_error!(env, ContractError::InvalidKycRequestStatus);
    }

    env.storage()
        .persistent()
        .set(&DataKey::KycStatus(subject.clone()), &VerificationStatus::Suspended);

    events::publish_kyc_suspended_event(
        env,
        subject.clone(),
        admin.clone(),
        reason.clone(),
        env.ledger().timestamp(),
    );

    reentrancy::exit(env);
}

// ── KYC status reads ──────────────────────────────────────────────────────────

/// Return the current `VerificationStatus` for `subject`.
pub fn get_kyc_status(env: &Env, subject: &Address) -> VerificationStatus {
    env.storage()
        .persistent()
        .get(&DataKey::KycStatus(subject.clone()))
        .unwrap_or(VerificationStatus::Unverified)
}

/// Return `true` iff `subject` currently holds a valid (non-expired) KYC approval.
pub fn is_kyc_approved(env: &Env, subject: &Address) -> bool {
    let status: VerificationStatus = env
        .storage()
        .persistent()
        .get(&DataKey::KycStatus(subject.clone()))
        .unwrap_or(VerificationStatus::Unverified);

    if status != VerificationStatus::Approved {
        return false;
    }

    // Check expiration when set (0 means no expiry).
    let expiration: u64 = env
        .storage()
        .persistent()
        .get(&DataKey::KycExpiration(subject.clone()))
        .unwrap_or(0);

    expiration == 0 || env.ledger().timestamp() <= expiration
}

/// Fetch a KYC request by ID, panicking if it does not exist.
pub fn get_kyc_request(env: &Env, request_id: u64) -> KycRequest {
    load_request(env, request_id)
}

/// Total number of KYC requests ever submitted (monotonically increasing).
pub fn get_kyc_request_count(env: &Env) -> u64 {
    get_request_count(env)
}

// ── Campaign KYC configuration ────────────────────────────────────────────────

/// Register KYC configuration for an existing campaign.
///
/// The campaign creator (`creator`) must hold a valid KYC approval before
/// they can register their campaign for KYC tracking.
/// `require_backer_kyc` controls whether backers must also be KYC-verified.
pub fn register_campaign_kyc(
    env: &Env,
    creator: &Address,
    campaign_id: u64,
    require_backer_kyc: bool,
) {
    reentrancy::enter(env);
    creator.require_auth();

    if !is_kyc_approved(env, creator) {
        panic_with_error!(env, ContractError::KycNotApproved);
    }

    let key = DataKey::CampaignKycStatus(campaign_id);
    if env.storage().persistent().has(&key) {
        panic_with_error!(env, ContractError::CampaignKycAlreadyRegistered);
    }

    let record = CampaignKycStatus {
        campaign_id,
        creator: creator.clone(),
        kyc_status: VerificationStatus::Pending,
        min_backer_kyc_required: require_backer_kyc,
        created_at: env.ledger().timestamp(),
        verified_at: 0,
        verified_by: None,
    };

    env.storage().persistent().set(&key, &record);

    events::publish_campaign_kyc_registered_event(
        env,
        campaign_id,
        creator.clone(),
        require_backer_kyc,
        env.ledger().timestamp(),
    );

    reentrancy::exit(env);
}

/// Mark a campaign as KYC-verified. Reviewer-only.
pub fn verify_campaign_kyc(
    env: &Env,
    reviewer: &Address,
    campaign_id: u64,
) {
    reentrancy::enter(env);
    reviewer.require_auth();
    assert_reviewer(env, reviewer);

    let key = DataKey::CampaignKycStatus(campaign_id);
    let mut record: CampaignKycStatus = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(env, ContractError::CampaignKycNotFound));

    if record.kyc_status == VerificationStatus::Approved {
        panic_with_error!(env, ContractError::CampaignKycAlreadyVerified);
    }

    record.kyc_status = VerificationStatus::Approved;
    record.verified_at = env.ledger().timestamp();
    record.verified_by = Some(reviewer.clone());

    env.storage().persistent().set(&key, &record);

    events::publish_campaign_kyc_verified_event(
        env,
        campaign_id,
        record.creator.clone(),
        reviewer.clone(),
        env.ledger().timestamp(),
    );

    reentrancy::exit(env);
}

/// Fetch the KYC configuration record for a campaign.
pub fn get_campaign_kyc_status(env: &Env, campaign_id: u64) -> CampaignKycStatus {
    env.storage()
        .persistent()
        .get(&DataKey::CampaignKycStatus(campaign_id))
        .unwrap_or_else(|| panic_with_error!(env, ContractError::CampaignKycNotFound))
}

/// Return `true` iff the campaign has been KYC-verified.
pub fn is_campaign_kyc_verified(env: &Env, campaign_id: u64) -> bool {
    let record: Option<CampaignKycStatus> = env
        .storage()
        .persistent()
        .get(&DataKey::CampaignKycStatus(campaign_id));
    record
        .map(|r| r.kyc_status == VerificationStatus::Approved)
        .unwrap_or(false)
}

// ── Backer KYC helpers ────────────────────────────────────────────────────────

/// Record that `backer` has been verified for `campaign_id` and update their
/// running BackerKycStatus aggregate.
pub fn verify_backer_for_campaign(
    env: &Env,
    reviewer: &Address,
    campaign_id: u64,
    backer: &Address,
) {
    reentrancy::enter(env);
    reviewer.require_auth();
    assert_reviewer(env, reviewer);

    if !is_kyc_approved(env, backer) {
        panic_with_error!(env, ContractError::KycNotApproved);
    }

    // Mark backer as verified for this specific campaign.
    env.storage()
        .persistent()
        .set(&DataKey::CampaignBackerVerified(campaign_id, backer.clone()), &true);

    // Update or initialise the backer's aggregate status record.
    let backer_key = DataKey::BackerKycStatus(campaign_id, backer.clone());
    let mut backer_status: BackerKycStatus = env
        .storage()
        .persistent()
        .get(&backer_key)
        .unwrap_or(BackerKycStatus {
            backer: backer.clone(),
            kyc_status: VerificationStatus::Unverified,
            campaigns_backed: 0,
            total_backed_amount: 0,
            last_kyc_check: 0,
        });

    backer_status.kyc_status = VerificationStatus::Approved;
    backer_status.campaigns_backed = backer_status.campaigns_backed.saturating_add(1);
    backer_status.last_kyc_check = env.ledger().timestamp();

    env.storage().persistent().set(&backer_key, &backer_status);

    events::publish_backer_kyc_verified_event(
        env,
        campaign_id,
        backer.clone(),
        reviewer.clone(),
        env.ledger().timestamp(),
    );

    reentrancy::exit(env);
}

/// Return `true` if `backer` is verified for the given campaign.
pub fn is_backer_kyc_verified(env: &Env, campaign_id: u64, backer: &Address) -> bool {
    env.storage()
        .persistent()
        .get(&DataKey::CampaignBackerVerified(campaign_id, backer.clone()))
        .unwrap_or(false)
}

/// Fetch the backer's KYC status record for a campaign.
pub fn get_backer_kyc_status(env: &Env, campaign_id: u64, backer: &Address) -> BackerKycStatus {
    env.storage()
        .persistent()
        .get(&DataKey::BackerKycStatus(campaign_id, backer.clone()))
        .unwrap_or(BackerKycStatus {
            backer: backer.clone(),
            kyc_status: VerificationStatus::Unverified,
            campaigns_backed: 0,
            total_backed_amount: 0,
            last_kyc_check: 0,
        })
}
