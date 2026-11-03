#![cfg(test)]

extern crate std;

use shade::shade::{Shade, ShadeClient};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::testutils::Events as _;
use soroban_sdk::{Address, Env, String};

fn setup(env: &Env) -> (Address, ShadeClient<'_>) {
    env.mock_all_auths();

    let contract_id = env.register(Shade, ());
    let client = ShadeClient::new(env, &contract_id);

    let admin = Address::generate(env);
    client.initialize(&admin);

    (admin, client)
}

#[test]
fn test_campaign_fee_policy_and_leaderboard() {
    let env = Env::default();
    let (_admin, client) = setup(&env);

    let owner = Address::generate(&env);
    let participant_a = Address::generate(&env);
    let participant_b = Address::generate(&env);
    let name = String::from_str(&env, "Save the forest");

    let campaign_id = client.create_fee_campaign(&owner, &name, &true, &1200, &800, &1000);
    client.configure_campaign_fee_policy(&owner, &campaign_id, &2500, &1500);

    let discounted = client.calculate_campaign_discount(&campaign_id, &10_000i128);
    assert_eq!(discounted, 6_000i128);

    client.record_fee_campaign_contribution(&participant_a, &campaign_id, &6_000i128);
    client.record_fee_campaign_contribution(&participant_b, &campaign_id, &4_000i128);
    client.stake_campaign(&participant_a, &campaign_id, &2_000i128);
    client.stake_campaign(&participant_b, &campaign_id, &1_000i128);
    // `env.events().all()` returns only the events of the most recent
    // invocation, so assert on that call's output before making further calls.
    assert!(!env.events().all().is_empty());

    let leaderboard = client.get_campaign_leaderboard(&campaign_id, &5u32);
    assert_eq!(leaderboard.len(), 2);
    let first = leaderboard.get(0).unwrap();
    let second = leaderboard.get(1).unwrap();
    assert_eq!(first.0, participant_a);
    assert_eq!(first.1, 8_000i128);
    assert_eq!(second.0, participant_b);
    assert_eq!(second.1, 5_000i128);

    let campaign = client.get_fee_campaign(&campaign_id);
    assert_eq!(campaign.owner, owner);
    assert_eq!(campaign.total_raised, 10_000i128);
    assert_eq!(campaign.total_staked, 3_000i128);
}

#[test]
fn test_campaign_unauthorized_access_is_rejected() {
    let env = Env::default();
    let (_admin, client) = setup(&env);

    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);
    let name = String::from_str(&env, "No access");

    let campaign_id = client.create_fee_campaign(&owner, &name, &false, &0, &0, &0);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.configure_campaign_fee_policy(&attacker, &campaign_id, &1000, &1000);
    }));

    assert!(result.is_err());

    let campaign = client.get_fee_campaign(&campaign_id);
    assert_eq!(campaign.fee_waiver_bps, 0);
    assert_eq!(campaign.discount_bps, 0);
}

#[test]
fn test_campaign_slashing_does_not_leave_partial_state() {
    let env = Env::default();
    let (_admin, client) = setup(&env);

    let owner = Address::generate(&env);
    let participant = Address::generate(&env);
    let name = String::from_str(&env, "Fraud prevention");

    let campaign_id = client.create_fee_campaign(&owner, &name, &true, &500, &250, &1000);
    client.stake_campaign(&participant, &campaign_id, &2_000i128);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.slash_campaign_stake(&owner, &campaign_id, &participant, &3_000i128);
    }));

    assert!(result.is_err());

    let participant_entry = client.get_campaign_participant(&campaign_id, &participant);
    assert_eq!(participant_entry.staked, 2_000i128);
    assert_eq!(participant_entry.slashed, 0i128);

    let campaign = client.get_fee_campaign(&campaign_id);
    assert_eq!(campaign.total_staked, 2_000i128);
    assert_eq!(campaign.total_slashed, 0i128);
}
