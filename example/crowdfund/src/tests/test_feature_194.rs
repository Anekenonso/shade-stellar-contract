use crate::*;
use soroban_sdk::testutils::{Address as _, Events as _, Ledger as _};
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{Address, Env, String, Symbol, TryIntoVal};

fn setup_env() -> (
    Env,
    Address,
    CrowdfundContractClient<'static>,
    Address,
    Address,
    Address,
) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = 1_000_000);

    let contract = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract);

    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let organizer = Address::generate(&env);
    let contributor = Address::generate(&env);

    (env, contract, client, token, organizer, contributor)
}

fn init_campaign(
    env: &Env,
    client: &CrowdfundContractClient<'static>,
    token: &Address,
    organizer: &Address,
) {
    let deadline = env.ledger().timestamp() + 86_400;
    client.init_campaign(organizer, token, &10_000, &deadline);
}

fn tiers(env: &Env) -> soroban_sdk::Vec<RewardTier> {
    soroban_sdk::vec![
        env,
        RewardTier {
            min_pledge: 200,
            name: String::from_str(env, "Silver")
        },
        RewardTier {
            min_pledge: 1_000,
            name: String::from_str(env, "Gold")
        },
        RewardTier {
            min_pledge: 5_000,
            name: String::from_str(env, "Platinum")
        },
    ]
}

// ── set_reward_tiers ───────────────────────────────────────────────────────

#[test]
fn test_set_reward_tiers_stores_and_retrieves() {
    let (env, _contract, client, token, organizer, _contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);

    client.set_reward_tiers(&tiers(&env));

    let c = Address::generate(&env);
    StellarAssetClient::new(&env, &token).mint(&c, &5_000);
    client.contribute(&c, &5_000);
    client.select_reward_tier(&c, &2);
    assert_eq!(client.get_selected_tier(&c), Some(2));
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #2)")]
fn test_set_reward_tiers_uninitialized_panics() {
    let (env, _contract, client, _token, _organizer, _contributor) = setup_env();
    client.set_reward_tiers(&tiers(&env));
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #3)")]
fn test_set_reward_tiers_non_ascending_panics() {
    let (env, _contract, client, token, organizer, _contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);

    let bad_tiers = soroban_sdk::vec![
        &env,
        RewardTier {
            min_pledge: 5_000,
            name: String::from_str(&env, "High")
        },
        RewardTier {
            min_pledge: 200,
            name: String::from_str(&env, "Low")
        },
    ];
    client.set_reward_tiers(&bad_tiers);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #3)")]
fn test_set_reward_tiers_duplicate_min_pledge_panics() {
    let (env, _contract, client, token, organizer, _contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);

    let dup_tiers = soroban_sdk::vec![
        &env,
        RewardTier {
            min_pledge: 200,
            name: String::from_str(&env, "Silver")
        },
        RewardTier {
            min_pledge: 200,
            name: String::from_str(&env, "Also Silver")
        },
    ];
    client.set_reward_tiers(&dup_tiers);
}

#[test]
fn test_set_reward_tiers_single_tier() {
    let (env, _contract, client, token, organizer, contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);

    let single = soroban_sdk::vec![
        &env,
        RewardTier {
            min_pledge: 100,
            name: String::from_str(&env, "Basic")
        },
    ];
    client.set_reward_tiers(&single);

    StellarAssetClient::new(&env, &token).mint(&contributor, &100);
    client.contribute(&contributor, &100);
    client.select_reward_tier(&contributor, &0);
    assert_eq!(client.get_selected_tier(&contributor), Some(0));
}

#[test]
fn test_set_reward_tiers_empty_vec() {
    let (env, _contract, client, token, organizer, contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);

    let empty: soroban_sdk::Vec<RewardTier> = soroban_sdk::Vec::new(&env);
    client.set_reward_tiers(&empty);

    StellarAssetClient::new(&env, &token).mint(&contributor, &500);
    client.contribute(&contributor, &500);
    let result = client.try_select_reward_tier(&contributor, &0);
    assert!(result.is_err());
}

#[test]
fn test_set_reward_tiers_overwrites_previous() {
    let (env, _contract, client, token, organizer, contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);

    let first = soroban_sdk::vec![
        &env,
        RewardTier {
            min_pledge: 100,
            name: String::from_str(&env, "Basic")
        },
    ];
    client.set_reward_tiers(&first);

    client.set_reward_tiers(&tiers(&env));

    StellarAssetClient::new(&env, &token).mint(&contributor, &150);
    client.contribute(&contributor, &150);
    let result = client.try_select_reward_tier(&contributor, &0);
    assert!(result.is_err());

    StellarAssetClient::new(&env, &token).mint(&contributor, &50);
    client.contribute(&contributor, &50);
    client.select_reward_tier(&contributor, &0);
    assert_eq!(client.get_selected_tier(&contributor), Some(0));
}

// ── select_reward_tier ─────────────────────────────────────────────────────

#[test]
fn test_select_reward_tier_above_minimum() {
    let (env, _contract, client, token, organizer, contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);
    client.set_reward_tiers(&tiers(&env));

    StellarAssetClient::new(&env, &token).mint(&contributor, &2_000);
    client.contribute(&contributor, &2_000);

    client.select_reward_tier(&contributor, &1);
    assert_eq!(client.get_selected_tier(&contributor), Some(1));
}

#[test]
fn test_select_reward_tier_upgrade_and_downgrade() {
    let (env, _contract, client, token, organizer, contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);
    client.set_reward_tiers(&tiers(&env));

    StellarAssetClient::new(&env, &token).mint(&contributor, &6_000);
    client.contribute(&contributor, &6_000);

    client.select_reward_tier(&contributor, &0);
    assert_eq!(client.get_selected_tier(&contributor), Some(0));

    client.select_reward_tier(&contributor, &2);
    assert_eq!(client.get_selected_tier(&contributor), Some(2));

    client.select_reward_tier(&contributor, &1);
    assert_eq!(client.get_selected_tier(&contributor), Some(1));
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #14)")]
fn test_select_reward_tier_invalid_index_panics() {
    let (env, _contract, client, token, organizer, contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);
    client.set_reward_tiers(&tiers(&env));

    StellarAssetClient::new(&env, &token).mint(&contributor, &6_000);
    client.contribute(&contributor, &6_000);

    client.select_reward_tier(&contributor, &10);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #13)")]
fn test_select_reward_tier_below_minimum_panics() {
    let (env, _contract, client, token, organizer, contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);
    client.set_reward_tiers(&tiers(&env));

    StellarAssetClient::new(&env, &token).mint(&contributor, &150);
    client.contribute(&contributor, &150);

    client.select_reward_tier(&contributor, &0);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #13)")]
fn test_select_reward_tier_zero_pledge_panics() {
    let (env, _contract, client, token, organizer, contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);
    client.set_reward_tiers(&tiers(&env));

    client.select_reward_tier(&contributor, &0);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #2)")]
fn test_select_reward_tier_no_tiers_set_panics() {
    let (env, _contract, client, token, organizer, contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);

    StellarAssetClient::new(&env, &token).mint(&contributor, &500);
    client.contribute(&contributor, &500);
    client.select_reward_tier(&contributor, &0);
}

// ── get_selected_tier ──────────────────────────────────────────────────────

#[test]
fn test_get_selected_tier_returns_none_before_selection() {
    let (env, _contract, client, token, organizer, contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);
    client.set_reward_tiers(&tiers(&env));

    assert_eq!(client.get_selected_tier(&contributor), None);
}

#[test]
fn test_get_selected_tier_returns_none_for_non_contributor() {
    let (env, _contract, client, token, organizer, _contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);
    client.set_reward_tiers(&tiers(&env));

    let random = Address::generate(&env);
    assert_eq!(client.get_selected_tier(&random), None);
}

#[test]
fn test_get_selected_tier_returns_some_after_selection() {
    let (env, _contract, client, token, organizer, contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);
    client.set_reward_tiers(&tiers(&env));

    StellarAssetClient::new(&env, &token).mint(&contributor, &5_000);
    client.contribute(&contributor, &5_000);
    client.select_reward_tier(&contributor, &2);

    assert_eq!(client.get_selected_tier(&contributor), Some(2));
}

#[test]
fn test_get_selected_tier_returns_updated_after_reselect() {
    let (env, _contract, client, token, organizer, contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);
    client.set_reward_tiers(&tiers(&env));

    StellarAssetClient::new(&env, &token).mint(&contributor, &5_000);
    client.contribute(&contributor, &5_000);

    client.select_reward_tier(&contributor, &0);
    assert_eq!(client.get_selected_tier(&contributor), Some(0));

    client.select_reward_tier(&contributor, &2);
    assert_eq!(client.get_selected_tier(&contributor), Some(2));
}

// ── fulfill_reward ─────────────────────────────────────────────────────────

#[test]
fn test_fulfill_reward_marks_backer_as_fulfilled() {
    let (env, _contract, client, token, organizer, contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);

    StellarAssetClient::new(&env, &token).mint(&contributor, &1_000);
    client.contribute(&contributor, &1_000);

    assert!(!client.is_fulfilled(&contributor));
    client.fulfill_reward(&contributor);
    assert!(client.is_fulfilled(&contributor));
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #12)")]
fn test_fulfill_reward_twice_panics() {
    let (env, _contract, client, token, organizer, contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);

    StellarAssetClient::new(&env, &token).mint(&contributor, &1_000);
    client.contribute(&contributor, &1_000);

    client.fulfill_reward(&contributor);
    client.fulfill_reward(&contributor);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #2)")]
fn test_fulfill_reward_uninitialized_panics() {
    let (_env, _contract, client, _token, _organizer, contributor) = setup_env();
    client.fulfill_reward(&contributor);
}

#[test]
fn test_fulfill_reward_non_backer_succeeds() {
    let (env, _contract, client, token, organizer, _contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);

    let random = Address::generate(&env);
    assert!(!client.is_fulfilled(&random));
    client.fulfill_reward(&random);
    assert!(client.is_fulfilled(&random));
}

#[test]
fn test_fulfill_reward_multiple_backers() {
    let (env, _contract, client, token, organizer, contributor) = setup_env();
    let contributor2 = Address::generate(&env);
    let contributor3 = Address::generate(&env);
    init_campaign(&env, &client, &token, &organizer);

    let sac = StellarAssetClient::new(&env, &token);
    sac.mint(&contributor, &500);
    sac.mint(&contributor2, &500);
    sac.mint(&contributor3, &500);
    client.contribute(&contributor, &500);
    client.contribute(&contributor2, &500);
    client.contribute(&contributor3, &500);

    client.fulfill_reward(&contributor);
    client.fulfill_reward(&contributor2);

    assert!(client.is_fulfilled(&contributor));
    assert!(client.is_fulfilled(&contributor2));
    assert!(!client.is_fulfilled(&contributor3));
}

// ── is_fulfilled ───────────────────────────────────────────────────────────

#[test]
fn test_is_fulfilled_default_false() {
    let (env, _contract, client, token, organizer, contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);

    assert!(!client.is_fulfilled(&contributor));
}

#[test]
fn test_is_fulfilled_false_for_random_address() {
    let (env, _contract, client, token, organizer, _contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);

    assert!(!client.is_fulfilled(&Address::generate(&env)));
}

#[test]
fn test_is_fulfilled_works_without_contribution() {
    let (env, _contract, client, token, organizer, contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);

    assert!(!client.is_fulfilled(&contributor));
    client.fulfill_reward(&contributor);
    assert!(client.is_fulfilled(&contributor));
}

// ── Auth failure tests (without mock_all_auths) ────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Auth,")]
fn test_non_organizer_cannot_set_reward_tiers() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.timestamp = 1_000_000);
    let contract = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract);
    let org = Address::generate(&env);
    let tok = env
        .register_stellar_asset_contract_v2(org.clone())
        .address();
    let _non_organizer = Address::generate(&env);
    client.init_campaign(&org, &tok, &1_000, &(env.ledger().timestamp() + 1_000));
    client.set_reward_tiers(&soroban_sdk::vec![
        &env,
        RewardTier {
            min_pledge: 100,
            name: String::from_str(&env, "Basic")
        },
    ]);
}

#[test]
#[should_panic(expected = "HostError: Error(Auth,")]
fn test_non_organizer_cannot_fulfill_reward() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.timestamp = 1_000_000);
    let contract = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract);
    let org = Address::generate(&env);
    let tok = env
        .register_stellar_asset_contract_v2(org.clone())
        .address();
    client.init_campaign(&org, &tok, &1_000, &(env.ledger().timestamp() + 1_000));
    let _non_organizer = Address::generate(&env);
    client.fulfill_reward(&_non_organizer);
}

#[test]
#[should_panic(expected = "HostError: Error(Auth,")]
fn test_different_contributor_cannot_select_tier_for_other() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.timestamp = 1_000_000);
    let contract = env.register(CrowdfundContract, ());
    let client = CrowdfundContractClient::new(&env, &contract);
    let org = Address::generate(&env);
    let tok = env
        .register_stellar_asset_contract_v2(org.clone())
        .address();
    client.init_campaign(&org, &tok, &1_000, &(env.ledger().timestamp() + 1_000));
    let contributor = Address::generate(&env);
    let attacker = Address::generate(&env);
    client.set_reward_tiers(&soroban_sdk::vec![
        &env,
        RewardTier {
            min_pledge: 100,
            name: String::from_str(&env, "Basic")
        },
    ]);
    let sac = soroban_sdk::token::StellarAssetClient::new(&env, &tok);
    sac.mint(&contributor, &500);
    client.contribute(&contributor, &500);
    client.select_reward_tier(&attacker, &0);
}

// ── Event verification ─────────────────────────────────────────────────────

fn get_last_event_name(env: &Env) -> Symbol {
    let events = env.events().all();
    assert!(!events.is_empty());
    let (_contract_id, topics, _data) = events.get(events.len() - 1).unwrap();
    topics.get(0).unwrap().try_into_val(env).unwrap()
}

#[test]
fn test_reward_tier_selected_event_emitted() {
    let (env, _contract, client, token, organizer, contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);
    client.set_reward_tiers(&tiers(&env));

    StellarAssetClient::new(&env, &token).mint(&contributor, &5_000);
    client.contribute(&contributor, &5_000);

    client.select_reward_tier(&contributor, &2);

    let event_name = get_last_event_name(&env);
    assert_eq!(event_name, Symbol::new(&env, "reward_tier_selected_event"));
}

#[test]
fn test_reward_fulfilled_event_emitted() {
    let (env, _contract, client, token, organizer, contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);

    StellarAssetClient::new(&env, &token).mint(&contributor, &1_000);
    client.contribute(&contributor, &1_000);

    client.fulfill_reward(&contributor);

    let event_name = get_last_event_name(&env);
    // The contribute call also emits PledgeReceivedEvent.
    // The last event should be reward_fulfilled_event.
    assert_eq!(event_name, Symbol::new(&env, "reward_fulfilled_event"));
}

// ── Storage rollback tests ─────────────────────────────────────────────────

#[test]
fn test_storage_unaffected_when_set_reward_tiers_fails() {
    let (env, _contract, client, token, organizer, contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);

    client.set_reward_tiers(&tiers(&env));

    let bad_tiers = soroban_sdk::vec![
        &env,
        RewardTier {
            min_pledge: 5_000,
            name: String::from_str(&env, "High")
        },
        RewardTier {
            min_pledge: 200,
            name: String::from_str(&env, "Low")
        },
    ];
    let result = client.try_set_reward_tiers(&bad_tiers);
    assert!(result.is_err());

    StellarAssetClient::new(&env, &token).mint(&contributor, &5_000);
    client.contribute(&contributor, &5_000);
    client.select_reward_tier(&contributor, &2);
    assert_eq!(client.get_selected_tier(&contributor), Some(2));
}

#[test]
fn test_storage_unaffected_when_select_tier_fails() {
    let (env, _contract, client, token, organizer, contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);
    client.set_reward_tiers(&tiers(&env));

    StellarAssetClient::new(&env, &token).mint(&contributor, &5_000);
    client.contribute(&contributor, &5_000);

    client.select_reward_tier(&contributor, &2);
    assert_eq!(client.get_selected_tier(&contributor), Some(2));

    let result = client.try_select_reward_tier(&contributor, &10);
    assert!(result.is_err());

    assert_eq!(client.get_selected_tier(&contributor), Some(2));
}

#[test]
fn test_storage_unaffected_when_fulfill_fails() {
    let (env, _contract, client, token, organizer, contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);

    StellarAssetClient::new(&env, &token).mint(&contributor, &1_000);
    client.contribute(&contributor, &1_000);

    client.fulfill_reward(&contributor);
    assert!(client.is_fulfilled(&contributor));

    let result = client.try_fulfill_reward(&contributor);
    assert!(result.is_err());

    assert!(client.is_fulfilled(&contributor));
}

// ── Uninitialized contract access tests ────────────────────────────────────

#[test]
fn test_get_selected_tier_uninitialized_returns_none() {
    let (_env, _contract, client, _token, _organizer, contributor) = setup_env();
    assert_eq!(client.get_selected_tier(&contributor), None);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #2)")]
fn test_select_reward_tier_uninitialized_panics() {
    let (_env, _contract, client, _token, _organizer, contributor) = setup_env();
    client.select_reward_tier(&contributor, &0);
}

#[test]
fn test_is_fulfilled_uninitialized_returns_false() {
    let (_env, _contract, client, _token, _organizer, contributor) = setup_env();
    assert!(!client.is_fulfilled(&contributor));
}

// ── Boundary value tests ───────────────────────────────────────────────────

#[test]
fn test_select_lowest_tier_minimum_pledge_works() {
    let (env, _contract, client, token, organizer, contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);

    let min_tier = soroban_sdk::vec![
        &env,
        RewardTier {
            min_pledge: 1,
            name: String::from_str(&env, "Minimal")
        },
    ];
    client.set_reward_tiers(&min_tier);

    StellarAssetClient::new(&env, &token).mint(&contributor, &1);
    client.contribute(&contributor, &1);
    client.select_reward_tier(&contributor, &0);
    assert_eq!(client.get_selected_tier(&contributor), Some(0));
}

#[test]
fn test_select_highest_tier_works_with_large_pledge() {
    let (env, _contract, client, token, organizer, contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);

    let whale_tier = soroban_sdk::vec![
        &env,
        RewardTier {
            min_pledge: 1_000,
            name: String::from_str(&env, "Whale")
        },
    ];
    client.set_reward_tiers(&whale_tier);

    let large_pledge: i128 = i128::MAX;
    StellarAssetClient::new(&env, &token).mint(&contributor, &large_pledge);
    client.contribute(&contributor, &large_pledge);
    client.select_reward_tier(&contributor, &0);
    assert_eq!(client.get_selected_tier(&contributor), Some(0));
}

#[test]
fn test_set_reward_tiers_single_tier_exact_minimum_works() {
    let (env, _contract, client, token, organizer, contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);

    let tiers = soroban_sdk::vec![
        &env,
        RewardTier {
            min_pledge: 100,
            name: String::from_str(&env, "Supporter")
        },
    ];
    client.set_reward_tiers(&tiers);

    StellarAssetClient::new(&env, &token).mint(&contributor, &100);
    client.contribute(&contributor, &100);
    client.select_reward_tier(&contributor, &0);
    assert_eq!(client.get_selected_tier(&contributor), Some(0));
}

#[test]
fn test_get_selected_tier_same_selection_no_storage_leak() {
    let (env, _contract, client, token, organizer, contributor) = setup_env();
    init_campaign(&env, &client, &token, &organizer);
    client.set_reward_tiers(&tiers(&env));

    StellarAssetClient::new(&env, &token).mint(&contributor, &5_000);
    client.contribute(&contributor, &5_000);

    client.select_reward_tier(&contributor, &2);
    client.select_reward_tier(&contributor, &2);
    assert_eq!(client.get_selected_tier(&contributor), Some(2));
}
