#![cfg(test)]

use shade::shade::{Shade, ShadeClient};
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{Address, Env, String};

struct Fixture<'a> {
    env: Env,
    client: ShadeClient<'a>,
    admin: Address,
    token: Address,
}

fn setup() -> Fixture<'static> {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    client.add_accepted_token(&admin, &token_address);

    Fixture {
        env,
        client,
        admin,
        token: token_address,
    }
}

fn register_merchant(env: &Env, client: &ShadeClient) -> Address {
    let merchant = Address::generate(env);
    client.register_merchant(&merchant);
    merchant
}

fn future_date(env: &Env) -> u64 {
    env.ledger().timestamp() + 86_400 // 1 day from now
}

fn str(env: &Env, s: &str) -> String {
    String::from_str(env, s)
}

// ── #335 Campaign creation ────────────────────────────────────────────────────

#[test]
fn create_campaign_stores_all_fields() {
    let f = setup();
    let merchant = register_merchant(&f.env, &f.client);
    let end = future_date(&f.env);

    let id = f.client.create_campaign(
        &merchant,
        &str(&f.env, "Seed Round"),
        &str(&f.env, "Raising funds for launch"),
        &10_000i128,
        &f.token,
        &end,
    );

    let c = f.client.get_campaign(&id);
    assert_eq!(c.id, id);
    assert_eq!(c.merchant, merchant);
    assert_eq!(c.title, str(&f.env, "Seed Round"));
    assert_eq!(c.description, str(&f.env, "Raising funds for launch"));
    assert_eq!(c.goal_amount, 10_000);
    assert_eq!(c.token, f.token);
    assert_eq!(c.end_date, end);
}

#[test]
fn create_campaign_increments_count() {
    let f = setup();
    let merchant = register_merchant(&f.env, &f.client);
    let end = future_date(&f.env);

    let id1 = f.client.create_campaign(
        &merchant,
        &str(&f.env, "Camp A"),
        &str(&f.env, "desc"),
        &0i128,
        &f.token,
        &end,
    );
    let id2 = f.client.create_campaign(
        &merchant,
        &str(&f.env, "Camp B"),
        &str(&f.env, "desc"),
        &0i128,
        &f.token,
        &end,
    );
    assert_eq!(id2, id1 + 1);
}

#[test]
fn create_campaign_open_ended_zero_goal() {
    let f = setup();
    let merchant = register_merchant(&f.env, &f.client);

    let id = f.client.create_campaign(
        &merchant,
        &str(&f.env, "Awareness"),
        &str(&f.env, "No funding goal"),
        &0i128,
        &f.token,
        &future_date(&f.env),
    );

    assert_eq!(f.client.get_campaign(&id).goal_amount, 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #58)")] // InvalidCampaignEndDate
fn create_campaign_rejects_past_end_date() {
    let f = setup();
    let merchant = register_merchant(&f.env, &f.client);

    f.client.create_campaign(
        &merchant,
        &str(&f.env, "X"),
        &str(&f.env, "desc"),
        &0i128,
        &f.token,
        &0u64, // timestamp 0 is in the past
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")] // InvalidAmount
fn create_campaign_rejects_negative_goal() {
    let f = setup();
    let merchant = register_merchant(&f.env, &f.client);

    f.client.create_campaign(
        &merchant,
        &str(&f.env, "X"),
        &str(&f.env, "desc"),
        &-1i128,
        &f.token,
        &future_date(&f.env),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #12)")] // TokenNotAccepted
fn create_campaign_rejects_unaccepted_token() {
    let f = setup();
    let merchant = register_merchant(&f.env, &f.client);
    let bad_token = Address::generate(&f.env);

    f.client.create_campaign(
        &merchant,
        &str(&f.env, "X"),
        &str(&f.env, "desc"),
        &0i128,
        &bad_token,
        &future_date(&f.env),
    );
}

// ── #335 Merchant campaign index ──────────────────────────────────────────────

#[test]
fn get_merchant_campaigns_returns_all_created() {
    let f = setup();
    let merchant = register_merchant(&f.env, &f.client);
    let end = future_date(&f.env);

    let id1 = f.client.create_campaign(
        &merchant,
        &str(&f.env, "A"),
        &str(&f.env, "d"),
        &0i128,
        &f.token,
        &end,
    );
    let id2 = f.client.create_campaign(
        &merchant,
        &str(&f.env, "B"),
        &str(&f.env, "d"),
        &0i128,
        &f.token,
        &end,
    );

    let ids = f.client.get_merchant_campaigns(&merchant);
    assert_eq!(ids.len(), 2);
    assert_eq!(ids.get(0).unwrap(), id1);
    assert_eq!(ids.get(1).unwrap(), id2);
}

#[test]
fn get_merchant_campaigns_empty_before_creation() {
    let f = setup();
    let merchant = register_merchant(&f.env, &f.client);
    assert_eq!(f.client.get_merchant_campaigns(&merchant).len(), 0);
}

// ── #335 Campaign update ──────────────────────────────────────────────────────

#[test]
fn update_campaign_changes_fields() {
    let f = setup();
    let merchant = register_merchant(&f.env, &f.client);
    let id = f.client.create_campaign(
        &merchant,
        &str(&f.env, "Old title"),
        &str(&f.env, "Old desc"),
        &0i128,
        &f.token,
        &future_date(&f.env),
    );

    let new_end = future_date(&f.env) + 86_400;
    f.client.update_campaign(
        &merchant,
        &id,
        &str(&f.env, "New title"),
        &str(&f.env, "New desc"),
        &new_end,
    );

    let c = f.client.get_campaign(&id);
    assert_eq!(c.title, str(&f.env, "New title"));
    assert_eq!(c.description, str(&f.env, "New desc"));
    assert_eq!(c.end_date, new_end);
}

#[test]
#[should_panic(expected = "Error(Contract, #59)")] // NotCampaignMerchant
fn update_campaign_rejects_wrong_merchant() {
    let f = setup();
    let merchant = register_merchant(&f.env, &f.client);
    let other = register_merchant(&f.env, &f.client);

    let id = f.client.create_campaign(
        &merchant,
        &str(&f.env, "T"),
        &str(&f.env, "d"),
        &0i128,
        &f.token,
        &future_date(&f.env),
    );

    f.client.update_campaign(
        &other,
        &id,
        &str(&f.env, "T"),
        &str(&f.env, "d"),
        &future_date(&f.env),
    );
}

// ── #335 Campaign cancellation ────────────────────────────────────────────────

#[test]
fn cancel_campaign_by_merchant_sets_status() {
    let f = setup();
    let merchant = register_merchant(&f.env, &f.client);
    let id = f.client.create_campaign(
        &merchant,
        &str(&f.env, "T"),
        &str(&f.env, "d"),
        &0i128,
        &f.token,
        &future_date(&f.env),
    );

    f.client.cancel_campaign(&merchant, &id);

    let c = f.client.get_campaign(&id);
    assert_eq!(c.status, crate::from_shade::types::CampaignStatus::Cancelled);
}

#[test]
fn cancel_campaign_by_admin_succeeds() {
    let f = setup();
    let merchant = register_merchant(&f.env, &f.client);
    let id = f.client.create_campaign(
        &merchant,
        &str(&f.env, "T"),
        &str(&f.env, "d"),
        &0i128,
        &f.token,
        &future_date(&f.env),
    );

    // Admin should be able to cancel any campaign.
    f.client.cancel_campaign(&f.admin, &id);
    assert_eq!(
        f.client.get_campaign(&id).status,
        crate::from_shade::types::CampaignStatus::Cancelled
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #56)")] // CampaignNotActive
fn cancel_already_cancelled_campaign_panics() {
    let f = setup();
    let merchant = register_merchant(&f.env, &f.client);
    let id = f.client.create_campaign(
        &merchant,
        &str(&f.env, "T"),
        &str(&f.env, "d"),
        &0i128,
        &f.token,
        &future_date(&f.env),
    );

    f.client.cancel_campaign(&merchant, &id);
    f.client.cancel_campaign(&merchant, &id); // second cancel must panic
}

#[test]
#[should_panic(expected = "Error(Contract, #59)")] // NotCampaignMerchant
fn cancel_campaign_by_unrelated_address_panics() {
    let f = setup();
    let merchant = register_merchant(&f.env, &f.client);
    let other = Address::generate(&f.env);
    let id = f.client.create_campaign(
        &merchant,
        &str(&f.env, "T"),
        &str(&f.env, "d"),
        &0i128,
        &f.token,
        &future_date(&f.env),
    );

    f.client.cancel_campaign(&other, &id);
}

// ── #335 Campaign end ─────────────────────────────────────────────────────────

#[test]
fn end_campaign_sets_ended_status() {
    let f = setup();
    let merchant = register_merchant(&f.env, &f.client);
    let id = f.client.create_campaign(
        &merchant,
        &str(&f.env, "T"),
        &str(&f.env, "d"),
        &0i128,
        &f.token,
        &future_date(&f.env),
    );

    f.client.end_campaign(&merchant, &id);

    assert_eq!(
        f.client.get_campaign(&id).status,
        crate::from_shade::types::CampaignStatus::Ended
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #57)")] // CampaignEnded
fn end_already_ended_campaign_panics() {
    let f = setup();
    let merchant = register_merchant(&f.env, &f.client);
    let id = f.client.create_campaign(
        &merchant,
        &str(&f.env, "T"),
        &str(&f.env, "d"),
        &0i128,
        &f.token,
        &future_date(&f.env),
    );

    f.client.end_campaign(&merchant, &id);
    f.client.end_campaign(&merchant, &id); // must panic
}

// ── #335 Campaign announcements ───────────────────────────────────────────────

#[test]
fn post_announcement_stores_content_and_returns_id() {
    let f = setup();
    let merchant = register_merchant(&f.env, &f.client);
    let campaign_id = f.client.create_campaign(
        &merchant,
        &str(&f.env, "Camp"),
        &str(&f.env, "desc"),
        &0i128,
        &f.token,
        &future_date(&f.env),
    );

    let ann_id = f.client.post_campaign_announcement(
        &merchant,
        &campaign_id,
        &str(&f.env, "Milestone reached!"),
        &str(&f.env, "We hit 50% of our goal."),
    );

    let anns = f.client.get_campaign_announcements(&campaign_id);
    assert_eq!(anns.len(), 1);
    let a = anns.get(0).unwrap();
    assert_eq!(a.id, ann_id);
    assert_eq!(a.campaign_id, campaign_id);
    assert_eq!(a.title, str(&f.env, "Milestone reached!"));
    assert_eq!(a.content, str(&f.env, "We hit 50% of our goal."));
}

#[test]
fn post_multiple_announcements_preserves_order() {
    let f = setup();
    let merchant = register_merchant(&f.env, &f.client);
    let campaign_id = f.client.create_campaign(
        &merchant,
        &str(&f.env, "Camp"),
        &str(&f.env, "desc"),
        &0i128,
        &f.token,
        &future_date(&f.env),
    );

    let ann1 = f.client.post_campaign_announcement(
        &merchant,
        &campaign_id,
        &str(&f.env, "Update 1"),
        &str(&f.env, "First update"),
    );
    let ann2 = f.client.post_campaign_announcement(
        &merchant,
        &campaign_id,
        &str(&f.env, "Update 2"),
        &str(&f.env, "Second update"),
    );

    assert_eq!(ann2, ann1 + 1);
    let anns = f.client.get_campaign_announcements(&campaign_id);
    assert_eq!(anns.len(), 2);
    assert_eq!(anns.get(0).unwrap().id, ann1);
    assert_eq!(anns.get(1).unwrap().id, ann2);
}

#[test]
fn get_campaign_announcements_empty_before_posts() {
    let f = setup();
    let merchant = register_merchant(&f.env, &f.client);
    let campaign_id = f.client.create_campaign(
        &merchant,
        &str(&f.env, "T"),
        &str(&f.env, "d"),
        &0i128,
        &f.token,
        &future_date(&f.env),
    );

    assert_eq!(f.client.get_campaign_announcements(&campaign_id).len(), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #56)")] // CampaignNotActive
fn post_announcement_on_cancelled_campaign_panics() {
    let f = setup();
    let merchant = register_merchant(&f.env, &f.client);
    let id = f.client.create_campaign(
        &merchant,
        &str(&f.env, "T"),
        &str(&f.env, "d"),
        &0i128,
        &f.token,
        &future_date(&f.env),
    );

    f.client.cancel_campaign(&merchant, &id);
    f.client.post_campaign_announcement(
        &merchant,
        &id,
        &str(&f.env, "Late news"),
        &str(&f.env, "content"),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #57)")] // CampaignEnded
fn post_announcement_on_ended_campaign_panics() {
    let f = setup();
    let merchant = register_merchant(&f.env, &f.client);
    let id = f.client.create_campaign(
        &merchant,
        &str(&f.env, "T"),
        &str(&f.env, "d"),
        &0i128,
        &f.token,
        &future_date(&f.env),
    );

    f.client.end_campaign(&merchant, &id);
    f.client.post_campaign_announcement(
        &merchant,
        &id,
        &str(&f.env, "News"),
        &str(&f.env, "content"),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #59)")] // NotCampaignMerchant
fn post_announcement_by_wrong_merchant_panics() {
    let f = setup();
    let merchant = register_merchant(&f.env, &f.client);
    let other = register_merchant(&f.env, &f.client);
    let id = f.client.create_campaign(
        &merchant,
        &str(&f.env, "T"),
        &str(&f.env, "d"),
        &0i128,
        &f.token,
        &future_date(&f.env),
    );

    f.client.post_campaign_announcement(
        &other,
        &id,
        &str(&f.env, "Fake news"),
        &str(&f.env, "content"),
    );
}

// ── #335 Campaigns across merchants are isolated ──────────────────────────────

#[test]
fn campaigns_are_isolated_per_merchant() {
    let f = setup();
    let merchant_a = register_merchant(&f.env, &f.client);
    let merchant_b = register_merchant(&f.env, &f.client);

    f.client.create_campaign(
        &merchant_a,
        &str(&f.env, "A's Camp"),
        &str(&f.env, "d"),
        &0i128,
        &f.token,
        &future_date(&f.env),
    );
    f.client.create_campaign(
        &merchant_b,
        &str(&f.env, "B's Camp"),
        &str(&f.env, "d"),
        &0i128,
        &f.token,
        &future_date(&f.env),
    );

    assert_eq!(f.client.get_merchant_campaigns(&merchant_a).len(), 1);
    assert_eq!(f.client.get_merchant_campaigns(&merchant_b).len(), 1);
}

// ── #335 get_campaign rejects unknown ID ──────────────────────────────────────

#[test]
#[should_panic(expected = "Error(Contract, #55)")] // CampaignNotFound
fn get_campaign_unknown_id_panics() {
    let f = setup();
    f.client.get_campaign(&999u64);
}
