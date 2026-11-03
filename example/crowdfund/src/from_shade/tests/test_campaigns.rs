#![cfg(test)]

use shade::shade::{Shade, ShadeClient};
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{Address, Env, String, Vec};

struct CampaignFixture<'a> {
    env: Env,
    client: ShadeClient<'a>,
    admin: Address,
    token: Address,
    merchant: Address,
}

fn setup() -> CampaignFixture<'static> {
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

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    CampaignFixture {
        env,
        client,
        admin,
        token: token_address,
        merchant,
    }
}

fn future_deadline(env: &Env) -> u64 {
    env.ledger().timestamp() + 86_400
}

// ── Categories (#352) ─────────────────────────────────────────────────────────

#[test]
fn create_campaign_category_stores_all_fields() {
    let f = setup();
    let id = f.client.create_campaign_category(
        &f.admin,
        &String::from_str(&f.env, "Technology"),
        &String::from_str(&f.env, "Tech campaigns"),
    );
    let cat = f.client.get_campaign_category(&id);
    assert_eq!(cat.id, id);
    assert_eq!(cat.name, String::from_str(&f.env, "Technology"));
    assert_eq!(cat.description, String::from_str(&f.env, "Tech campaigns"));
    assert!(cat.active);
}

#[test]
fn get_campaign_categories_returns_all() {
    let f = setup();
    f.client.create_campaign_category(
        &f.admin,
        &String::from_str(&f.env, "Tech"),
        &String::from_str(&f.env, "Tech desc"),
    );
    f.client.create_campaign_category(
        &f.admin,
        &String::from_str(&f.env, "Art"),
        &String::from_str(&f.env, "Art desc"),
    );
    f.client.create_campaign_category(
        &f.admin,
        &String::from_str(&f.env, "Music"),
        &String::from_str(&f.env, "Music desc"),
    );
    let cats = f.client.get_campaign_categories();
    assert_eq!(cats.len(), 3);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")] // NotAuthorized
fn create_campaign_category_rejects_non_admin() {
    let f = setup();
    let imposter = Address::generate(&f.env);
    f.client.create_campaign_category(
        &imposter,
        &String::from_str(&f.env, "Tech"),
        &String::from_str(&f.env, "desc"),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #202)")] // CampaignCategoryAlreadyExists
fn create_campaign_category_rejects_duplicate_name() {
    let f = setup();
    f.client.create_campaign_category(
        &f.admin,
        &String::from_str(&f.env, "Tech"),
        &String::from_str(&f.env, "desc"),
    );
    f.client.create_campaign_category(
        &f.admin,
        &String::from_str(&f.env, "Tech"),
        &String::from_str(&f.env, "other"),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #33)")] // InvalidDescription (empty)
fn create_campaign_category_rejects_empty_name() {
    let f = setup();
    f.client.create_campaign_category(
        &f.admin,
        &String::from_str(&f.env, ""),
        &String::from_str(&f.env, "desc"),
    );
}

#[test]
fn update_campaign_category_changes_fields() {
    let f = setup();
    let id = f.client.create_campaign_category(
        &f.admin,
        &String::from_str(&f.env, "Tech"),
        &String::from_str(&f.env, "desc"),
    );
    f.client.update_campaign_category(
        &f.admin,
        &id,
        &Some(String::from_str(&f.env, "Technology")),
        &Some(String::from_str(&f.env, "new desc")),
        &Some(false),
    );
    let cat = f.client.get_campaign_category(&id);
    assert_eq!(cat.name, String::from_str(&f.env, "Technology"));
    assert_eq!(cat.description, String::from_str(&f.env, "new desc"));
    assert!(!cat.active);
}

#[test]
#[should_panic(expected = "Error(Contract, #201)")] // CampaignCategoryNotFound
fn get_campaign_category_rejects_missing() {
    let f = setup();
    f.client.get_campaign_category(&999);
}

// ── Tags (#352) ───────────────────────────────────────────────────────────────

#[test]
fn create_campaign_tag_by_merchant() {
    let f = setup();
    let id = f
        .client
        .create_campaign_tag(&f.merchant, &String::from_str(&f.env, "rust"));
    let tag = f.client.get_campaign_tag(&id);
    assert_eq!(tag.name, String::from_str(&f.env, "rust"));
    assert_eq!(tag.creator, f.merchant);
}

#[test]
fn create_campaign_tag_by_admin() {
    let f = setup();
    let id = f
        .client
        .create_campaign_tag(&f.admin, &String::from_str(&f.env, "platform"));
    let tag = f.client.get_campaign_tag(&id);
    assert_eq!(tag.creator, f.admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")] // NotAuthorized
fn create_campaign_tag_rejects_unregistered_user() {
    let f = setup();
    let imposter = Address::generate(&f.env);
    f.client
        .create_campaign_tag(&imposter, &String::from_str(&f.env, "x"));
}

#[test]
#[should_panic(expected = "Error(Contract, #205)")] // CampaignTagAlreadyExists
fn create_campaign_tag_rejects_duplicate_name() {
    let f = setup();
    f.client
        .create_campaign_tag(&f.merchant, &String::from_str(&f.env, "rust"));
    f.client
        .create_campaign_tag(&f.merchant, &String::from_str(&f.env, "rust"));
}

#[test]
fn get_campaign_tags_returns_all() {
    let f = setup();
    f.client
        .create_campaign_tag(&f.merchant, &String::from_str(&f.env, "a"));
    f.client
        .create_campaign_tag(&f.merchant, &String::from_str(&f.env, "b"));
    let tags = f.client.get_campaign_tags();
    assert_eq!(tags.len(), 2);
}

// ── Campaigns (#352) ─────────────────────────────────────────────────────────

#[test]
fn create_campaign_stores_all_fields() {
    let f = setup();
    let cat_id = f.client.create_campaign_category(
        &f.admin,
        &String::from_str(&f.env, "Tech"),
        &String::from_str(&f.env, "desc"),
    );
    let tag_a = f
        .client
        .create_campaign_tag(&f.merchant, &String::from_str(&f.env, "rust"));
    let tag_b = f
        .client
        .create_campaign_tag(&f.merchant, &String::from_str(&f.env, "sdk"));

    let id = f.client.create_campaign(
        &f.merchant,
        &String::from_str(&f.env, "Shade SDK"),
        &String::from_str(&f.env, "Reusable Soroban toolkit"),
        &cat_id,
        &Vec::from_array(&f.env, [tag_a, tag_b]),
        &10_000i128,
        &f.token,
        &future_deadline(&f.env),
    );

    let campaign = f.client.get_campaign(&id);
    assert_eq!(campaign.id, id);
    assert_eq!(campaign.title, String::from_str(&f.env, "Shade SDK"));
    assert_eq!(campaign.category_id, cat_id);
    assert_eq!(campaign.tags.len(), 2);
    assert_eq!(campaign.goal_amount, 10_000);
    assert_eq!(campaign.raised_amount, 0);
    assert!(campaign.active);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")] // MerchantNotFound
fn create_campaign_rejects_non_merchant() {
    let f = setup();
    let imposter = Address::generate(&f.env);
    let cat_id = f.client.create_campaign_category(
        &f.admin,
        &String::from_str(&f.env, "Tech"),
        &String::from_str(&f.env, "desc"),
    );
    f.client.create_campaign(
        &imposter,
        &String::from_str(&f.env, "X"),
        &String::from_str(&f.env, "x"),
        &cat_id,
        &Vec::new(&f.env),
        &1i128,
        &f.token,
        &future_deadline(&f.env),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #206)")] // InvalidCampaignGoal
fn create_campaign_rejects_zero_goal() {
    let f = setup();
    let cat_id = f.client.create_campaign_category(
        &f.admin,
        &String::from_str(&f.env, "Tech"),
        &String::from_str(&f.env, "desc"),
    );
    f.client.create_campaign(
        &f.merchant,
        &String::from_str(&f.env, "X"),
        &String::from_str(&f.env, "x"),
        &cat_id,
        &Vec::new(&f.env),
        &0i128,
        &f.token,
        &future_deadline(&f.env),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #207)")] // InvalidCampaignDeadline
fn create_campaign_rejects_past_deadline() {
    let f = setup();
    let cat_id = f.client.create_campaign_category(
        &f.admin,
        &String::from_str(&f.env, "Tech"),
        &String::from_str(&f.env, "desc"),
    );
    f.client.create_campaign(
        &f.merchant,
        &String::from_str(&f.env, "X"),
        &String::from_str(&f.env, "x"),
        &cat_id,
        &Vec::new(&f.env),
        &1i128,
        &f.token,
        &0u64,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #203)")] // CampaignCategoryInactive
fn create_campaign_rejects_inactive_category() {
    let f = setup();
    let cat_id = f.client.create_campaign_category(
        &f.admin,
        &String::from_str(&f.env, "Tech"),
        &String::from_str(&f.env, "desc"),
    );
    f.client
        .update_campaign_category(&f.admin, &cat_id, &None, &None, &Some(false));
    f.client.create_campaign(
        &f.merchant,
        &String::from_str(&f.env, "X"),
        &String::from_str(&f.env, "x"),
        &cat_id,
        &Vec::new(&f.env),
        &1i128,
        &f.token,
        &future_deadline(&f.env),
    );
}

#[test]
fn update_campaign_changes_title_and_description() {
    let f = setup();
    let cat_id = f.client.create_campaign_category(
        &f.admin,
        &String::from_str(&f.env, "Tech"),
        &String::from_str(&f.env, "desc"),
    );
    let id = f.client.create_campaign(
        &f.merchant,
        &String::from_str(&f.env, "Old title"),
        &String::from_str(&f.env, "Old desc"),
        &cat_id,
        &Vec::new(&f.env),
        &1i128,
        &f.token,
        &future_deadline(&f.env),
    );
    f.client.update_campaign(
        &f.merchant,
        &id,
        &Some(String::from_str(&f.env, "New title")),
        &Some(String::from_str(&f.env, "New desc")),
    );
    let c = f.client.get_campaign(&id);
    assert_eq!(c.title, String::from_str(&f.env, "New title"));
    assert_eq!(c.description, String::from_str(&f.env, "New desc"));
}

#[test]
#[should_panic(expected = "Error(Contract, #209)")] // NotCampaignMerchant
fn update_campaign_rejects_non_owner() {
    let f = setup();
    let cat_id = f.client.create_campaign_category(
        &f.admin,
        &String::from_str(&f.env, "Tech"),
        &String::from_str(&f.env, "desc"),
    );
    let id = f.client.create_campaign(
        &f.merchant,
        &String::from_str(&f.env, "X"),
        &String::from_str(&f.env, "x"),
        &cat_id,
        &Vec::new(&f.env),
        &1i128,
        &f.token,
        &future_deadline(&f.env),
    );
    let imposter = Address::generate(&f.env);
    // Register imposter as a merchant so the MerchantNotFound guard doesn't
    // fire first.
    f.client.register_merchant(&imposter);
    f.client.update_campaign(
        &imposter,
        &id,
        &Some(String::from_str(&f.env, "Hacked")),
        &None,
    );
}

#[test]
fn add_and_remove_campaign_tag_updates_indices() {
    let f = setup();
    let cat_id = f.client.create_campaign_category(
        &f.admin,
        &String::from_str(&f.env, "Tech"),
        &String::from_str(&f.env, "desc"),
    );
    let tag_one = f
        .client
        .create_campaign_tag(&f.merchant, &String::from_str(&f.env, "alpha"));
    let tag_two = f
        .client
        .create_campaign_tag(&f.merchant, &String::from_str(&f.env, "beta"));

    let id = f.client.create_campaign(
        &f.merchant,
        &String::from_str(&f.env, "C"),
        &String::from_str(&f.env, "d"),
        &cat_id,
        &Vec::from_array(&f.env, [tag_one]),
        &1i128,
        &f.token,
        &future_deadline(&f.env),
    );

    // Add a new tag.
    f.client.add_campaign_tag(&f.merchant, &id, &tag_two);
    let c = f.client.get_campaign(&id);
    assert_eq!(c.tags.len(), 2);

    // Adding the same tag again is a no-op.
    f.client.add_campaign_tag(&f.merchant, &id, &tag_two);
    let c2 = f.client.get_campaign(&id);
    assert_eq!(c2.tags.len(), 2);

    // Removing it brings the count back down.
    f.client.remove_campaign_tag(&f.merchant, &id, &tag_two);
    let c3 = f.client.get_campaign(&id);
    assert_eq!(c3.tags.len(), 1);

    // get_campaigns_by_tag should no longer include this campaign.
    let by_tag = f.client.get_campaigns(&crate::from_shade::types::CampaignFilter {
        is_active: None,
        category_id: None,
        tag_id: Some(tag_two),
        merchant_id: None,
    });
    assert_eq!(by_tag.len(), 0);
}

#[test]
fn set_campaign_active_toggles_flag() {
    let f = setup();
    let cat_id = f.client.create_campaign_category(
        &f.admin,
        &String::from_str(&f.env, "Tech"),
        &String::from_str(&f.env, "desc"),
    );
    let id = f.client.create_campaign(
        &f.merchant,
        &String::from_str(&f.env, "C"),
        &String::from_str(&f.env, "d"),
        &cat_id,
        &Vec::new(&f.env),
        &1i128,
        &f.token,
        &future_deadline(&f.env),
    );
    assert!(f.client.get_campaign(&id).active);
    f.client.set_campaign_active(&f.merchant, &id, &false);
    assert!(!f.client.get_campaign(&id).active);
}

#[test]
fn record_campaign_contribution_accumulates() {
    let f = setup();
    let cat_id = f.client.create_campaign_category(
        &f.admin,
        &String::from_str(&f.env, "Tech"),
        &String::from_str(&f.env, "desc"),
    );
    let id = f.client.create_campaign(
        &f.merchant,
        &String::from_str(&f.env, "C"),
        &String::from_str(&f.env, "d"),
        &cat_id,
        &Vec::new(&f.env),
        &1_000i128,
        &f.token,
        &future_deadline(&f.env),
    );
    let contributor = Address::generate(&f.env);
    f.client
        .record_campaign_contribution(&id, &contributor, &250i128);
    f.client
        .record_campaign_contribution(&id, &contributor, &100i128);
    let c = f.client.get_campaign(&id);
    assert_eq!(c.raised_amount, 350);
}

#[test]
#[should_panic(expected = "Error(Contract, #208)")] // CampaignInactive
fn record_campaign_contribution_rejects_inactive_campaign() {
    let f = setup();
    let cat_id = f.client.create_campaign_category(
        &f.admin,
        &String::from_str(&f.env, "Tech"),
        &String::from_str(&f.env, "desc"),
    );
    let id = f.client.create_campaign(
        &f.merchant,
        &String::from_str(&f.env, "C"),
        &String::from_str(&f.env, "d"),
        &cat_id,
        &Vec::new(&f.env),
        &1_000i128,
        &f.token,
        &future_deadline(&f.env),
    );
    f.client.set_campaign_active(&f.merchant, &id, &false);
    f.client
        .record_campaign_contribution(&id, &f.merchant, &10i128);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")] // InvalidAmount
fn record_campaign_contribution_rejects_zero_amount() {
    let f = setup();
    let cat_id = f.client.create_campaign_category(
        &f.admin,
        &String::from_str(&f.env, "Tech"),
        &String::from_str(&f.env, "desc"),
    );
    let id = f.client.create_campaign(
        &f.merchant,
        &String::from_str(&f.env, "C"),
        &String::from_str(&f.env, "d"),
        &cat_id,
        &Vec::new(&f.env),
        &1_000i128,
        &f.token,
        &future_deadline(&f.env),
    );
    f.client
        .record_campaign_contribution(&id, &f.merchant, &0i128);
}

#[test]
#[should_panic(expected = "Error(Contract, #210)")] // CampaignExpired
fn record_campaign_contribution_rejects_expired_campaign() {
    let f = setup();
    let cat_id = f.client.create_campaign_category(
        &f.admin,
        &String::from_str(&f.env, "Tech"),
        &String::from_str(&f.env, "desc"),
    );
    let id = f.client.create_campaign(
        &f.merchant,
        &String::from_str(&f.env, "C"),
        &String::from_str(&f.env, "d"),
        &cat_id,
        &Vec::new(&f.env),
        &1_000i128,
        &f.token,
        &future_deadline(&f.env),
    );
    // Advance the ledger past the deadline.
    // Read the timestamp before with_mut: reading inside the closure double-borrows.
    let now = f.env.ledger().timestamp();
    f.env.ledger().with_mut(|l| l.timestamp = now + 90_000);
    f.client
        .record_campaign_contribution(&id, &f.merchant, &10i128);
}

#[test]
fn get_campaigns_filters_by_category_and_tag() {
    let f = setup();
    let cat_tech = f.client.create_campaign_category(
        &f.admin,
        &String::from_str(&f.env, "Tech"),
        &String::from_str(&f.env, "Tech desc"),
    );
    let cat_art = f.client.create_campaign_category(
        &f.admin,
        &String::from_str(&f.env, "Art"),
        &String::from_str(&f.env, "Art desc"),
    );
    let tag_rust = f
        .client
        .create_campaign_tag(&f.merchant, &String::from_str(&f.env, "rust"));
    let tag_soroban = f
        .client
        .create_campaign_tag(&f.merchant, &String::from_str(&f.env, "soroban"));

    f.client.create_campaign(
        &f.merchant,
        &String::from_str(&f.env, "TechRustCamp"),
        &String::from_str(&f.env, "d"),
        &cat_tech,
        &Vec::from_array(&f.env, [tag_rust]),
        &1i128,
        &f.token,
        &future_deadline(&f.env),
    );
    f.client.create_campaign(
        &f.merchant,
        &String::from_str(&f.env, "TechSorobanCamp"),
        &String::from_str(&f.env, "d"),
        &cat_tech,
        &Vec::from_array(&f.env, [tag_soroban]),
        &1i128,
        &f.token,
        &future_deadline(&f.env),
    );
    f.client.create_campaign(
        &f.merchant,
        &String::from_str(&f.env, "ArtCamp"),
        &String::from_str(&f.env, "d"),
        &cat_art,
        &Vec::from_array(&f.env, [tag_rust]),
        &1i128,
        &f.token,
        &future_deadline(&f.env),
    );

    // Filter: only Tech category
    let only_tech = f.client.get_campaigns(&crate::from_shade::types::CampaignFilter {
        is_active: None,
        category_id: Some(cat_tech),
        tag_id: None,
        merchant_id: None,
    });
    assert_eq!(only_tech.len(), 2);

    // Filter: Tech + rust tag intersection
    let tech_rust = f.client.get_campaigns(&crate::from_shade::types::CampaignFilter {
        is_active: None,
        category_id: Some(cat_tech),
        tag_id: Some(tag_rust),
        merchant_id: None,
    });
    assert_eq!(tech_rust.len(), 1);
    assert_eq!(
        tech_rust.get(0).unwrap().title,
        String::from_str(&f.env, "TechRustCamp")
    );

    // Filter: rust tag across all categories
    let all_rust = f.client.get_campaigns(&crate::from_shade::types::CampaignFilter {
        is_active: None,
        category_id: None,
        tag_id: Some(tag_rust),
        merchant_id: None,
    });
    assert_eq!(all_rust.len(), 2);
}
