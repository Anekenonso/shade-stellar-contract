#![cfg(test)]
//! Campaign analytics exports: a creator snapshots their campaign's
//! contribution aggregate into an immutable, event-published record.

use shade::shade::{Shade, ShadeClient};
use crate::from_shade::types::{ExportFormat};
use soroban_sdk::testutils::{Address as _, Events as _, Ledger as _};
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{Address, Env, FromVal, String, Symbol};

/// Ledger time the fixture starts at. Non-zero so "never happened" (`0`) stays
/// distinguishable from "happened at the start".
const START: u64 = 1_000;
const DEADLINE: u64 = START + 86_400;
/// Minted to every backer the fixture creates.
const FUNDING: i128 = 10_000_000;

struct Fixture<'a> {
    env: Env,
    client: ShadeClient<'a>,
    admin: Address,
    token: Address,
    merchant: Address,
    campaign_id: u64,
}

impl Fixture<'_> {
    /// A funded address that has never contributed.
    fn new_backer(&self) -> Address {
        let backer = Address::generate(&self.env);
        StellarAssetClient::new(&self.env, &self.token).mint(&backer, &FUNDING);
        backer
    }

    /// Contributes `amount` from a brand-new backer and returns it.
    fn pledge(&self, amount: i128) -> Address {
        let backer = self.new_backer();
        self.client
            .pledge_to_campaign(&backer, &self.campaign_id, &amount);
        backer
    }

    fn set_time(&self, timestamp: u64) {
        self.env.ledger().with_mut(|l| l.timestamp = timestamp);
    }

    fn export(&self) -> u64 {
        self.client
            .export_campaign_analytics(&self.merchant, &self.campaign_id, &ExportFormat::Csv)
    }

    /// Topic of the most recently published event.
    fn last_event_topic(&self) -> Symbol {
        let events = self.env.events().all();
        let last = events.last().unwrap();
        assert_eq!(last.0, self.client.address);
        Symbol::from_val(&self.env, &last.1.get(0).unwrap())
    }
}

/// A registered merchant with an open, empty backer campaign.
fn setup() -> Fixture<'static> {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = START);

    let contract_id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    client.add_accepted_token(&admin, &token);

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    let campaign_id = client.create_backer_campaign(
        &merchant,
        &String::from_str(&env, "Open Hardware Rev 2"),
        &token,
        &DEADLINE,
    );

    Fixture {
        env,
        client,
        admin,
        token,
        merchant,
        campaign_id,
    }
}

// ── Aggregation ───────────────────────────────────────────────────────────────

#[test]
fn test_stats_start_empty() {
    let f = setup();

    let stats = f.client.get_campaign_stats(&f.campaign_id);
    assert_eq!(stats.campaign_id, f.campaign_id);
    assert_eq!(stats.pledge_count, 0);
    assert_eq!(stats.backer_count, 0);
    assert_eq!(stats.tracked_raised, 0);
    assert_eq!(stats.largest_pledge, 0);
    assert_eq!(stats.smallest_pledge, 0);
    assert_eq!(stats.first_pledge_at, 0);
    assert_eq!(stats.last_pledge_at, 0);
    assert_eq!(stats.export_count, 0);
    assert_eq!(stats.last_export_id, 0);
}

#[test]
fn test_first_pledge_seeds_both_extremes() {
    let f = setup();
    f.pledge(500);

    let stats = f.client.get_campaign_stats(&f.campaign_id);
    assert_eq!(stats.pledge_count, 1);
    assert_eq!(stats.backer_count, 1);
    assert_eq!(stats.tracked_raised, 500);
    assert_eq!(stats.largest_pledge, 500);
    assert_eq!(stats.smallest_pledge, 500);
    assert_eq!(stats.first_pledge_at, START);
    assert_eq!(stats.last_pledge_at, START);
}

#[test]
fn test_extremes_widen_across_pledges() {
    let f = setup();
    f.pledge(500);
    f.pledge(9_000);
    f.pledge(100);
    f.pledge(1_200);

    let stats = f.client.get_campaign_stats(&f.campaign_id);
    assert_eq!(stats.pledge_count, 4);
    assert_eq!(stats.tracked_raised, 10_800);
    assert_eq!(stats.largest_pledge, 9_000);
    assert_eq!(stats.smallest_pledge, 100);
}

#[test]
fn test_timestamps_track_first_and_last_pledge() {
    let f = setup();
    f.pledge(500);
    f.set_time(START + 3_600);
    f.pledge(700);

    let stats = f.client.get_campaign_stats(&f.campaign_id);
    assert_eq!(stats.first_pledge_at, START);
    assert_eq!(stats.last_pledge_at, START + 3_600);
}

#[test]
fn test_repeat_backer_counts_once() {
    let f = setup();
    let backer = f.pledge(500);
    f.client.pledge_to_campaign(&backer, &f.campaign_id, &300);
    f.client.pledge_to_campaign(&backer, &f.campaign_id, &200);

    let stats = f.client.get_campaign_stats(&f.campaign_id);
    // Three contributions, but only one distinct contributor.
    assert_eq!(stats.pledge_count, 3);
    assert_eq!(stats.backer_count, 1);
    assert_eq!(stats.tracked_raised, 1_000);
}

#[test]
fn test_pledge_emits_stats_event_with_running_aggregate() {
    let f = setup();
    f.pledge(500);

    assert_eq!(
        f.last_event_topic(),
        Symbol::new(&f.env, "campaign_stats_updated_event")
    );
}

#[test]
fn test_stats_are_isolated_per_campaign() {
    let f = setup();
    let other = f.client.create_backer_campaign(
        &f.merchant,
        &String::from_str(&f.env, "Second Run"),
        &f.token,
        &DEADLINE,
    );

    f.pledge(500);

    assert_eq!(f.client.get_campaign_stats(&f.campaign_id).pledge_count, 1);
    assert_eq!(f.client.get_campaign_stats(&other).pledge_count, 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #200)")] // CampaignNotFound
fn test_stats_for_unknown_campaign_reject() {
    let f = setup();
    f.client.get_campaign_stats(&999);
}

// ── Exporting ─────────────────────────────────────────────────────────────────

#[test]
fn test_export_snapshots_the_aggregate() {
    let f = setup();
    f.pledge(1_000);
    f.pledge(3_000);
    f.set_time(START + 60);

    let export_id = f.export();
    let export = f.client.get_analytics_export(&export_id);

    assert_eq!(export.id, export_id);
    assert_eq!(export.campaign_id, f.campaign_id);
    assert_eq!(export.creator, f.merchant);
    assert_eq!(export.merchant_id, 1);
    assert_eq!(export.token, f.token);
    assert_eq!(export.format, ExportFormat::Csv);
    assert_eq!(export.sequence, 1);
    // No previous export, so the window opens at the start of time.
    assert_eq!(export.period_start, 0);
    assert_eq!(export.period_end, START + 60);
    assert_eq!(export.created_at, START + 60);

    assert_eq!(export.total_raised, 4_000);
    assert_eq!(export.pledge_count, 2);
    assert_eq!(export.backer_count, 2);
    assert_eq!(export.average_pledge, 2_000);
    assert_eq!(export.largest_pledge, 3_000);
    assert_eq!(export.smallest_pledge, 1_000);
    assert_eq!(export.first_pledge_at, START);
    assert_eq!(export.last_pledge_at, START);

    // A first export covers everything, so its delta is the whole raise.
    assert_eq!(export.period_raised, 4_000);
    assert_eq!(export.period_pledges, 2);
    assert_eq!(export.period_backers, 2);
}

#[test]
fn test_export_carries_campaign_context() {
    let f = setup();
    f.pledge(2_500);

    let export = f.client.get_analytics_export(&f.export());
    assert_eq!(export.campaign_raised, 2_500);
    assert_eq!(export.campaign_deadline, DEADLINE);
    assert!(export.campaign_active);
}

#[test]
fn test_export_truncates_average_rather_than_rounding() {
    let f = setup();
    f.pledge(10);
    f.pledge(11);

    // 21 / 2 == 10.5, reported as 10 so the figure never overstates the raise.
    assert_eq!(
        f.client.get_analytics_export(&f.export()).average_pledge,
        10
    );
}

#[test]
fn test_export_records_requested_format() {
    let f = setup();
    f.pledge(500);

    let id = f
        .client
        .export_campaign_analytics(&f.merchant, &f.campaign_id, &ExportFormat::Ndjson);
    assert_eq!(
        f.client.get_analytics_export(&id).format,
        ExportFormat::Ndjson
    );
}

#[test]
fn test_export_emits_event() {
    let f = setup();
    f.pledge(500);
    f.export();

    assert_eq!(
        f.last_event_topic(),
        Symbol::new(&f.env, "analytics_export_event")
    );
}

#[test]
fn test_export_indexes_under_campaign() {
    let f = setup();
    f.pledge(500);

    let first = f.export();
    f.set_time(START + 10);
    let second = f.export();

    let exports = f.client.get_campaign_exports(&f.campaign_id);
    assert_eq!(exports.len(), 2);
    assert_eq!(exports.get(0).unwrap(), first);
    assert_eq!(exports.get(1).unwrap(), second);
    assert_eq!(
        f.client.get_latest_campaign_export(&f.campaign_id).id,
        second
    );
}

#[test]
fn test_export_advances_cursor_without_resetting_the_aggregate() {
    let f = setup();
    f.pledge(500);
    f.set_time(START + 60);
    let export_id = f.export();

    let stats = f.client.get_campaign_stats(&f.campaign_id);
    // Cumulative counters survive the export untouched...
    assert_eq!(stats.pledge_count, 1);
    assert_eq!(stats.tracked_raised, 500);
    // ...and the cursor now marks them all as covered.
    assert_eq!(stats.export_count, 1);
    assert_eq!(stats.last_export_id, export_id);
    assert_eq!(stats.last_export_at, START + 60);
    assert_eq!(stats.exported_pledge_count, 1);
    assert_eq!(stats.exported_backer_count, 1);
    assert_eq!(stats.exported_raised, 500);
}

#[test]
fn test_export_ids_are_global_across_campaigns() {
    let f = setup();
    f.pledge(500);
    let other = f.client.create_backer_campaign(
        &f.merchant,
        &String::from_str(&f.env, "Second Run"),
        &f.token,
        &DEADLINE,
    );
    let other_backer = f.new_backer();
    f.client.pledge_to_campaign(&other_backer, &other, &900);

    let first = f.export();
    let second = f
        .client
        .export_campaign_analytics(&f.merchant, &other, &ExportFormat::Json);

    assert_eq!(first, 1);
    assert_eq!(second, 2);
    // Each campaign still numbers its own series from one.
    assert_eq!(f.client.get_analytics_export(&second).sequence, 1);
}

// ── Incremental windows ───────────────────────────────────────────────────────

#[test]
fn test_second_export_reports_only_the_delta() {
    let f = setup();
    f.pledge(1_000);
    f.set_time(START + 100);
    f.export();

    f.set_time(START + 200);
    f.pledge(400);
    f.pledge(600);
    f.set_time(START + 300);
    let second = f.client.get_analytics_export(&f.export());

    assert_eq!(second.sequence, 2);
    // The window opens where the previous export closed.
    assert_eq!(second.period_start, START + 100);
    assert_eq!(second.period_end, START + 300);

    // Cumulative figures cover the whole campaign...
    assert_eq!(second.total_raised, 2_000);
    assert_eq!(second.pledge_count, 3);
    assert_eq!(second.backer_count, 3);
    // ...while the delta covers only what arrived since the first export.
    assert_eq!(second.period_raised, 1_000);
    assert_eq!(second.period_pledges, 2);
    assert_eq!(second.period_backers, 2);
}

#[test]
fn test_repeat_export_in_same_second_reports_zero_delta() {
    let f = setup();
    f.pledge(1_000);

    let first = f.client.get_analytics_export(&f.export());
    // Same ledger second, no intervening contribution. The delta comes off the
    // counter cursor rather than the clock, so the second export must not
    // re-report what the first already covered.
    let second = f.client.get_analytics_export(&f.export());

    assert_eq!(first.period_raised, 1_000);
    assert_eq!(second.period_raised, 0);
    assert_eq!(second.period_pledges, 0);
    assert_eq!(second.period_backers, 0);
    // A zero-width window is the honest description of what it covered.
    assert_eq!(second.period_start, second.period_end);
    // The cumulative view is identical in both.
    assert_eq!(second.total_raised, first.total_raised);
    assert_eq!(second.sequence, 2);
}

#[test]
fn test_deltas_across_a_series_sum_to_the_total() {
    let f = setup();
    let mut expected_raised = 0_i128;
    let mut summed_periods = 0_i128;

    for round in 0..5_u64 {
        f.set_time(START + round * 100);
        f.pledge(100 * i128::from(round + 1));
        expected_raised += 100 * i128::from(round + 1);

        let export = f.client.get_analytics_export(&f.export());
        summed_periods += export.period_raised;
    }

    assert_eq!(summed_periods, expected_raised);
    assert_eq!(
        f.client
            .get_latest_campaign_export(&f.campaign_id)
            .total_raised,
        expected_raised
    );
}

// ── Authorization and guards ──────────────────────────────────────────────────

#[test]
#[should_panic(expected = "Error(Contract, #301)")] // NotAnalyticsExportOwner
fn test_non_owner_cannot_export() {
    let f = setup();
    f.pledge(500);

    let intruder = Address::generate(&f.env);
    f.client
        .export_campaign_analytics(&intruder, &f.campaign_id, &ExportFormat::Csv);
}

#[test]
#[should_panic(expected = "Error(Contract, #301)")] // NotAnalyticsExportOwner
fn test_other_merchant_cannot_export() {
    let f = setup();
    f.pledge(500);

    let rival = Address::generate(&f.env);
    f.client.register_merchant(&rival);
    f.client
        .export_campaign_analytics(&rival, &f.campaign_id, &ExportFormat::Csv);
}

#[test]
#[should_panic(expected = "Error(Contract, #302)")] // NothingToExport
fn test_cannot_export_a_campaign_with_no_contributions() {
    let f = setup();
    f.export();
}

#[test]
#[should_panic(expected = "Error(Contract, #303)")] // TooManyExports
fn test_export_count_is_capped_per_campaign() {
    let f = setup();
    f.pledge(500);

    // 64 is the cap; the 65th must be refused.
    for round in 0..65_u64 {
        f.set_time(START + round);
        f.export();
    }
}

#[test]
#[should_panic(expected = "Error(Contract, #300)")] // AnalyticsExportNotFound
fn test_unknown_export_id_rejects() {
    let f = setup();
    f.client.get_analytics_export(&999);
}

#[test]
#[should_panic(expected = "Error(Contract, #304)")] // NoExportsYet
fn test_latest_export_before_any_export_rejects() {
    let f = setup();
    f.pledge(500);
    f.client.get_latest_campaign_export(&f.campaign_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #200)")] // CampaignNotFound
fn test_export_of_unknown_campaign_rejects() {
    let f = setup();
    f.client
        .export_campaign_analytics(&f.merchant, &999, &ExportFormat::Csv);
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")] // ContractPaused
fn test_export_blocked_while_paused() {
    let f = setup();
    f.pledge(500);
    f.client.pause(&f.admin);
    f.export();
}

#[test]
fn test_reads_stay_available_while_paused() {
    let f = setup();
    f.pledge(500);
    let export_id = f.export();
    f.client.pause(&f.admin);

    // Backers auditing a creator's published figures must not depend on the
    // contract being unpaused.
    assert_eq!(f.client.get_campaign_stats(&f.campaign_id).pledge_count, 1);
    assert_eq!(f.client.get_analytics_export(&export_id).id, export_id);
}
