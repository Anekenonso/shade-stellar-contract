//! Campaign analytics and creator data exports.
//!
//! A creator running a backer campaign needs their raise as *data*: how many
//! people backed it, how the average contribution moved, what came in since
//! they last looked. The chain holds every contribution already, but only as a
//! stream of individual pledges — answering "what is my average pledge" from it
//! means replaying the campaign's whole history.
//!
//! This component keeps that answer as it goes. Every contribution to a backer
//! campaign folds into a small running aggregate ([`CampaignStats`]), and a
//! creator can snapshot that aggregate at any time into an immutable
//! [`AnalyticsExport`] record which is also emitted as an event for off-chain
//! tooling to render as CSV or JSON.
//!
//! # What an export is, and is not
//!
//! The contract does not build a file. It publishes the *figures*, the format
//! the creator asked for, and the window they cover; indexers and UIs turn that
//! into bytes. Storing the intent on-chain is what makes an export reproducible
//! — anyone can rebuild the identical file from the same snapshot, and the
//! creator's authorization for it is on the ledger.
//!
//! Per-backer rows are deliberately not part of an export. Producing them
//! on-chain would need a backer roster that grows without bound and is walked
//! in full on every export, which is exactly the shape that makes a Soroban
//! call unaffordable at the size where the export starts to matter. The
//! per-pledge events this component and [`crate::from_shade::backer_rewards`]
//! emit already carry every row an indexer needs, so rows are reconstructed
//! off-chain and the on-chain cost stays flat in the number of backers.
//!
//! # Incremental exports
//!
//! Consecutive exports partition the campaign's timeline: each covers
//! everything since the previous one's `period_end`. The delta is computed from
//! a *counter* cursor stored alongside the aggregate, not from timestamps, so
//! two exports in the same ledger second still partition the data exactly —
//! the second sees the first's cursor and reports a zero delta rather than
//! double-counting the contributions they share a second with.
//!
//! # Invariants
//!
//! - Only the campaign's owning merchant may export it. Reads are public:
//!   backers being able to audit a creator's published figures is the point.
//! - Every counter in [`CampaignStats`] only ever grows, and an export never
//!   mutates them — it only advances the cursor. Exports are immutable once
//!   written and there is no update or delete path.
//! - `tracked_raised` counts what this component observed. A campaign's own
//!   `BackerCampaign::raised` stays authoritative and is carried on every
//!   export beside it, so a campaign that took contributions before this
//!   component shipped reports a short tracked total rather than a wrong
//!   authoritative one.
//!
//! # Storage
//!
//! Keys live in [`AnalyticsKey`], a dedicated enum, so this feature adds no
//! cases to the near-full `CampaignKey` (Soroban caps every enum at 50 cases).
//! The aggregate and its export cursor share one entry keyed by `campaign_id`,
//! so recording a contribution and running an export each cost a single
//! read-modify-write. Derived figures (averages, deltas) are computed on demand
//! rather than stored — nothing pays rent to hold a number the next
//! contribution invalidates.

use crate::from_shade::backer_rewards;
use crate::from_shade::errors::AnalyticsError;
use crate::from_shade::events;
use crate::from_shade::types::{AnalyticsExport, AnalyticsKey, BackerCampaign, CampaignStats, ExportFormat};
use soroban_sdk::{panic_with_error, Address, Env, Vec};

/// Upper bound on exports per campaign. Bounds the reverse-index entry so its
/// reads and writes stay within a predictable rent/CPU budget, and stops an
/// export loop from minting storage entries without limit.
const MAX_EXPORTS_PER_CAMPAIGN: u32 = 64;

// ── Aggregate ─────────────────────────────────────────────────────────────────

/// The campaign's aggregate, or a zeroed one if it has seen no contributions.
///
/// Absence is not an error: a campaign that has taken no pledges legitimately
/// has an all-zero aggregate, and returning that is more useful to a caller
/// than a panic it has to special-case.
fn load_stats(env: &Env, campaign_id: u64) -> CampaignStats {
    env.storage()
        .persistent()
        .get(&AnalyticsKey::CampaignStats(campaign_id))
        .unwrap_or(CampaignStats {
            campaign_id,
            pledge_count: 0,
            backer_count: 0,
            tracked_raised: 0,
            largest_pledge: 0,
            smallest_pledge: 0,
            first_pledge_at: 0,
            last_pledge_at: 0,
            export_count: 0,
            last_export_id: 0,
            last_export_at: 0,
            exported_pledge_count: 0,
            exported_backer_count: 0,
            exported_raised: 0,
        })
}

fn save_stats(env: &Env, stats: &CampaignStats) {
    env.storage()
        .persistent()
        .set(&AnalyticsKey::CampaignStats(stats.campaign_id), stats);
}

/// `tracked_raised / pledge_count`, truncated; `0` before the first
/// contribution.
fn average_pledge(stats: &CampaignStats) -> i128 {
    if stats.pledge_count == 0 {
        return 0;
    }
    stats.tracked_raised / i128::from(stats.pledge_count)
}

/// Folds one contribution into a campaign's running aggregate.
///
/// Called from the pledge path rather than exposed as an entrypoint: analytics
/// must reflect contributions the contract actually accepted, so there is
/// deliberately no way to write a figure in from outside. `is_new_backer` says
/// whether this address had contributed before — the caller already read that
/// to update its own state, so passing it here avoids a second storage read.
pub fn record_pledge(
    env: &Env,
    campaign_id: u64,
    backer: &Address,
    amount: i128,
    is_new_backer: bool,
) {
    let mut stats = load_stats(env, campaign_id);
    let now = env.ledger().timestamp();

    // A first contribution seeds both extremes; afterwards they only widen.
    if stats.pledge_count == 0 {
        stats.first_pledge_at = now;
        stats.largest_pledge = amount;
        stats.smallest_pledge = amount;
    } else {
        if amount > stats.largest_pledge {
            stats.largest_pledge = amount;
        }
        if amount < stats.smallest_pledge {
            stats.smallest_pledge = amount;
        }
    }

    stats.pledge_count = stats.pledge_count.saturating_add(1);
    if is_new_backer {
        stats.backer_count = stats.backer_count.saturating_add(1);
    }
    stats.tracked_raised = stats.tracked_raised.saturating_add(amount);
    stats.last_pledge_at = now;

    save_stats(env, &stats);

    events::publish_campaign_stats_updated_event(
        env,
        campaign_id,
        backer.clone(),
        amount,
        is_new_backer,
        &stats,
        average_pledge(&stats),
        now,
    );
}

/// A campaign's running aggregate. Public: backers can audit the same figures
/// the creator exports.
///
/// Panics if the campaign does not exist, so a typo'd ID reads as an error
/// rather than as a campaign with no activity.
pub fn get_campaign_stats(env: &Env, campaign_id: u64) -> CampaignStats {
    backer_rewards::get_backer_campaign(env, campaign_id);
    load_stats(env, campaign_id)
}

// ── Exports ───────────────────────────────────────────────────────────────────

fn get_export_count(env: &Env) -> u64 {
    env.storage()
        .persistent()
        .get(&AnalyticsKey::AnalyticsExportCount)
        .unwrap_or(0)
}

fn load_campaign_exports(env: &Env, campaign_id: u64) -> Vec<u64> {
    env.storage()
        .persistent()
        .get(&AnalyticsKey::CampaignExports(campaign_id))
        .unwrap_or_else(|| Vec::new(env))
}

/// Snapshots a campaign's analytics into an immutable export record and emits
/// it for off-chain rendering, returning the new export's ID.
///
/// Callable only by the campaign's owning merchant. The snapshot reports both
/// the cumulative figures and the delta since this campaign's previous export,
/// so a creator polling for updates gets an incremental feed rather than the
/// whole history each time.
pub fn export_campaign_analytics(
    env: &Env,
    creator: &Address,
    campaign_id: u64,
    format: ExportFormat,
) -> u64 {
    creator.require_auth();

    let campaign: BackerCampaign = assert_campaign_creator(env, campaign_id, creator);

    let mut stats = load_stats(env, campaign_id);
    // An export over an empty dataset would pay rent to say nothing. A campaign
    // that raised before this component shipped has no tracked pledges but is
    // not empty: its authoritative raise is still worth exporting, and refusing
    // it would lock every pre-existing campaign out of the feature for good.
    if stats.pledge_count == 0 && campaign.raised == 0 {
        panic_with_error!(env, AnalyticsError::NothingToExport);
    }

    let mut exports = load_campaign_exports(env, campaign_id);
    if exports.len() >= MAX_EXPORTS_PER_CAMPAIGN {
        panic_with_error!(env, AnalyticsError::TooManyExports);
    }

    let export_id = get_export_count(env) + 1;
    let now = env.ledger().timestamp();

    // Deltas come off the counter cursor, never off the clock: same-second
    // exports then partition the data exactly instead of double-counting the
    // contributions they share a second with.
    let export = AnalyticsExport {
        id: export_id,
        campaign_id,
        creator: creator.clone(),
        merchant_id: campaign.merchant_id,
        token: campaign.token.clone(),
        format,
        sequence: stats.export_count.saturating_add(1),
        period_start: stats.last_export_at,
        period_end: now,
        campaign_raised: campaign.raised,
        campaign_deadline: campaign.deadline,
        campaign_active: campaign.active,
        total_raised: stats.tracked_raised,
        pledge_count: stats.pledge_count,
        backer_count: stats.backer_count,
        average_pledge: average_pledge(&stats),
        largest_pledge: stats.largest_pledge,
        smallest_pledge: stats.smallest_pledge,
        first_pledge_at: stats.first_pledge_at,
        last_pledge_at: stats.last_pledge_at,
        period_raised: stats.tracked_raised - stats.exported_raised,
        period_pledges: stats.pledge_count - stats.exported_pledge_count,
        period_backers: stats.backer_count - stats.exported_backer_count,
        created_at: now,
    };

    env.storage()
        .persistent()
        .set(&AnalyticsKey::AnalyticsExport(export_id), &export);
    env.storage()
        .persistent()
        .set(&AnalyticsKey::AnalyticsExportCount, &export_id);

    exports.push_back(export_id);
    env.storage()
        .persistent()
        .set(&AnalyticsKey::CampaignExports(campaign_id), &exports);

    // Advance the cursor to what this export covered. The aggregate itself is
    // untouched — an export reads history, it does not reset it.
    stats.export_count = export.sequence;
    stats.last_export_id = export_id;
    stats.last_export_at = now;
    stats.exported_pledge_count = stats.pledge_count;
    stats.exported_backer_count = stats.backer_count;
    stats.exported_raised = stats.tracked_raised;
    save_stats(env, &stats);

    events::publish_analytics_export_event(env, &export);

    export_id
}

/// Asserts `creator` is the merchant that owns `campaign_id`, returning the
/// campaign.
///
/// Reports [`AnalyticsError::NotAnalyticsExportOwner`] rather than the generic
/// authorization error, so a creator who exported the wrong campaign ID sees
/// why.
fn assert_campaign_creator(env: &Env, campaign_id: u64, creator: &Address) -> BackerCampaign {
    let campaign = backer_rewards::get_backer_campaign(env, campaign_id);
    if !backer_rewards::is_backer_campaign_owner(env, &campaign, creator) {
        panic_with_error!(env, AnalyticsError::NotAnalyticsExportOwner);
    }
    campaign
}

/// A stored export snapshot.
pub fn get_analytics_export(env: &Env, export_id: u64) -> AnalyticsExport {
    env.storage()
        .persistent()
        .get(&AnalyticsKey::AnalyticsExport(export_id))
        .unwrap_or_else(|| panic_with_error!(env, AnalyticsError::AnalyticsExportNotFound))
}

/// Export IDs for a campaign, in the order they were run.
pub fn get_campaign_exports(env: &Env, campaign_id: u64) -> Vec<u64> {
    backer_rewards::get_backer_campaign(env, campaign_id);
    load_campaign_exports(env, campaign_id)
}

/// The most recent export for a campaign.
pub fn get_latest_campaign_export(env: &Env, campaign_id: u64) -> AnalyticsExport {
    let stats = get_campaign_stats(env, campaign_id);
    if stats.last_export_id == 0 {
        panic_with_error!(env, AnalyticsError::NoExportsYet);
    }
    get_analytics_export(env, stats.last_export_id)
}
