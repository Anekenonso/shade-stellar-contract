#![cfg(test)]

use shade::shade::{Shade, ShadeClient};
use crate::from_shade::types::{BackerPerk, BackerRewardTier};
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{Address, Env, String, Vec};

struct Fixture<'a> {
    env: Env,
    client: ShadeClient<'a>,
    #[allow(dead_code)]
    admin: Address,
    token: Address,
    merchant: Address,
}

fn setup() -> Fixture<'static> {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    client.add_accepted_token(&admin, &token);

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    Fixture {
        env,
        client,
        admin,
        token,
        merchant,
    }
}

fn future_deadline(env: &Env) -> u64 {
    env.ledger().timestamp() + 86_400
}

fn fund(env: &Env, token: &Address, to: &Address, amount: i128) {
    StellarAssetClient::new(env, token).mint(to, &amount);
}

fn sample_tiers(env: &Env) -> Vec<BackerRewardTier> {
    let basic_perks = Vec::from_array(
        env,
        [BackerPerk {
            name: String::from_str(env, "Digital Badge"),
            description: String::from_str(env, "Exclusive backer badge"),
        }],
    );
    let premium_perks = Vec::from_array(
        env,
        [
            BackerPerk {
                name: String::from_str(env, "Digital Badge"),
                description: String::from_str(env, "Exclusive backer badge"),
            },
            BackerPerk {
                name: String::from_str(env, "Early Access"),
                description: String::from_str(env, "Beta access pass"),
            },
        ],
    );

    Vec::from_array(
        env,
        [
            BackerRewardTier {
                min_pledge: 100,
                name: String::from_str(env, "Basic"),
                description: String::from_str(env, "Entry tier"),
                perks: basic_perks,
                max_backers: 0,
            },
            BackerRewardTier {
                min_pledge: 500,
                name: String::from_str(env, "Premium"),
                description: String::from_str(env, "Premium tier"),
                perks: premium_perks,
                max_backers: 2,
            },
        ],
    )
}

#[test]
fn test_create_backer_campaign_stores_fields() {
    let f = setup();
    let name = String::from_str(&f.env, "Community Build");
    let deadline = future_deadline(&f.env);

    let campaign_id = f
        .client
        .create_backer_campaign(&f.merchant, &name, &f.token, &deadline);
    assert_eq!(campaign_id, 1);

    let campaign = f.client.get_backer_campaign(&campaign_id);
    assert_eq!(campaign.id, 1);
    assert_eq!(campaign.merchant_id, 1);
    assert_eq!(campaign.name, name);
    assert_eq!(campaign.token, f.token);
    assert_eq!(campaign.deadline, deadline);
    assert_eq!(campaign.raised, 0);
    assert!(campaign.active);
}

#[test]
fn test_set_reward_tiers_and_select_tier() {
    let f = setup();
    let campaign_id = f.client.create_backer_campaign(
        &f.merchant,
        &String::from_str(&f.env, "Tier Campaign"),
        &f.token,
        &future_deadline(&f.env),
    );
    f.client
        .set_backer_reward_tiers(&f.merchant, &campaign_id, &sample_tiers(&f.env));

    let backer = Address::generate(&f.env);
    fund(&f.env, &f.token, &backer, 500);
    f.client.pledge_to_campaign(&backer, &campaign_id, &500);

    f.client
        .select_backer_reward_tier(&backer, &campaign_id, &1);
    assert_eq!(
        f.client.get_backer_selected_tier(&campaign_id, &backer),
        Some(1)
    );
}

#[test]
fn test_cumulative_pledge_unlocks_higher_tier() {
    let f = setup();
    let campaign_id = f.client.create_backer_campaign(
        &f.merchant,
        &String::from_str(&f.env, "Cumulative"),
        &f.token,
        &future_deadline(&f.env),
    );
    f.client
        .set_backer_reward_tiers(&f.merchant, &campaign_id, &sample_tiers(&f.env));

    let backer = Address::generate(&f.env);
    fund(&f.env, &f.token, &backer, 600);
    f.client.pledge_to_campaign(&backer, &campaign_id, &300);
    f.client.pledge_to_campaign(&backer, &campaign_id, &300);

    assert_eq!(f.client.get_backer_pledge(&campaign_id, &backer), 600);
    f.client
        .select_backer_reward_tier(&backer, &campaign_id, &1);
    assert_eq!(
        f.client.get_backer_selected_tier(&campaign_id, &backer),
        Some(1)
    );
}

#[test]
fn test_fulfill_reward_and_claim_perk() {
    let f = setup();
    let campaign_id = f.client.create_backer_campaign(
        &f.merchant,
        &String::from_str(&f.env, "Fulfillment"),
        &f.token,
        &future_deadline(&f.env),
    );
    f.client
        .set_backer_reward_tiers(&f.merchant, &campaign_id, &sample_tiers(&f.env));

    let backer = Address::generate(&f.env);
    fund(&f.env, &f.token, &backer, 500);
    f.client.pledge_to_campaign(&backer, &campaign_id, &500);
    f.client
        .select_backer_reward_tier(&backer, &campaign_id, &1);

    assert!(!f.client.is_backer_reward_fulfilled(&campaign_id, &backer));
    f.client
        .fulfill_backer_reward(&f.merchant, &campaign_id, &backer);
    assert!(f.client.is_backer_reward_fulfilled(&campaign_id, &backer));

    assert!(!f.client.is_backer_perk_claimed(&campaign_id, &backer, &0));
    f.client.claim_backer_perk(&backer, &campaign_id, &0);
    assert!(f.client.is_backer_perk_claimed(&campaign_id, &backer, &0));
}

#[test]
fn test_fulfillment_is_independent_per_backer() {
    let f = setup();
    let campaign_id = f.client.create_backer_campaign(
        &f.merchant,
        &String::from_str(&f.env, "Multi Backer"),
        &f.token,
        &future_deadline(&f.env),
    );

    let backer1 = Address::generate(&f.env);
    let backer2 = Address::generate(&f.env);
    fund(&f.env, &f.token, &backer1, 200);
    fund(&f.env, &f.token, &backer2, 200);
    f.client.pledge_to_campaign(&backer1, &campaign_id, &200);
    f.client.pledge_to_campaign(&backer2, &campaign_id, &200);

    f.client
        .fulfill_backer_reward(&f.merchant, &campaign_id, &backer1);
    assert!(f.client.is_backer_reward_fulfilled(&campaign_id, &backer1));
    assert!(!f.client.is_backer_reward_fulfilled(&campaign_id, &backer2));
}

#[test]
#[should_panic]
fn test_select_tier_below_minimum_panics() {
    let f = setup();
    let campaign_id = f.client.create_backer_campaign(
        &f.merchant,
        &String::from_str(&f.env, "Below Min"),
        &f.token,
        &future_deadline(&f.env),
    );
    f.client
        .set_backer_reward_tiers(&f.merchant, &campaign_id, &sample_tiers(&f.env));

    let backer = Address::generate(&f.env);
    fund(&f.env, &f.token, &backer, 50);
    f.client.pledge_to_campaign(&backer, &campaign_id, &50);
    f.client
        .select_backer_reward_tier(&backer, &campaign_id, &0);
}

#[test]
#[should_panic]
fn test_tier_capacity_enforced() {
    let f = setup();
    let campaign_id = f.client.create_backer_campaign(
        &f.merchant,
        &String::from_str(&f.env, "Capacity"),
        &f.token,
        &future_deadline(&f.env),
    );
    f.client
        .set_backer_reward_tiers(&f.merchant, &campaign_id, &sample_tiers(&f.env));

    let backer1 = Address::generate(&f.env);
    let backer2 = Address::generate(&f.env);
    let backer3 = Address::generate(&f.env);
    fund(&f.env, &f.token, &backer1, 500);
    fund(&f.env, &f.token, &backer2, 500);
    fund(&f.env, &f.token, &backer3, 500);

    f.client.pledge_to_campaign(&backer1, &campaign_id, &500);
    f.client.pledge_to_campaign(&backer2, &campaign_id, &500);
    f.client.pledge_to_campaign(&backer3, &campaign_id, &500);

    f.client
        .select_backer_reward_tier(&backer1, &campaign_id, &1);
    f.client
        .select_backer_reward_tier(&backer2, &campaign_id, &1);
    // Premium tier max_backers = 2 — third selection must panic.
    f.client
        .select_backer_reward_tier(&backer3, &campaign_id, &1);
}

#[test]
#[should_panic]
fn test_claim_perk_before_fulfillment_panics() {
    let f = setup();
    let campaign_id = f.client.create_backer_campaign(
        &f.merchant,
        &String::from_str(&f.env, "Early Claim"),
        &f.token,
        &future_deadline(&f.env),
    );
    f.client
        .set_backer_reward_tiers(&f.merchant, &campaign_id, &sample_tiers(&f.env));

    let backer = Address::generate(&f.env);
    fund(&f.env, &f.token, &backer, 500);
    f.client.pledge_to_campaign(&backer, &campaign_id, &500);
    f.client
        .select_backer_reward_tier(&backer, &campaign_id, &1);
    f.client.claim_backer_perk(&backer, &campaign_id, &0);
}

#[test]
#[should_panic]
fn test_pledge_after_deadline_panics() {
    let f = setup();
    let deadline = future_deadline(&f.env);
    let campaign_id = f.client.create_backer_campaign(
        &f.merchant,
        &String::from_str(&f.env, "Expired"),
        &f.token,
        &deadline,
    );

    f.env.ledger().with_mut(|l| l.timestamp = deadline + 1);

    let backer = Address::generate(&f.env);
    fund(&f.env, &f.token, &backer, 100);
    f.client.pledge_to_campaign(&backer, &campaign_id, &100);
}
