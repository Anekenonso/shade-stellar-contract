#![cfg(test)]

extern crate std;

use crate::{CrowdfundContract, CrowdfundContractClient};
use soroban_sdk::testutils::{Address as _, Events, Ledger as _};
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{Address, Env, Map, Symbol, TryIntoVal, Val};

fn setup() -> (Env, Address, CrowdfundContractClient<'static>, Address, Address, Address) {
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

    let goal = 10_000_i128;
    let deadline = env.ledger().timestamp() + 86_400;
    client.init_campaign(&organizer, &token, &goal, &deadline);

    (env, contract, client, token, organizer, contributor)
}

// ── set_affiliate_commission_bps ──────────────────────────────────────────────

#[test]
fn test_set_affiliate_commission_bps_succeeds() {
    let (_env, _contract, client, _token, organizer, _contributor) = setup();
    client.set_affiliate_commission_bps(&organizer, &500);
}

#[test]
#[should_panic]
fn test_set_affiliate_commission_bps_zero_panics() {
    let (_env, _contract, client, _token, organizer, _contributor) = setup();
    client.set_affiliate_commission_bps(&organizer, &0);
}

#[test]
#[should_panic]
fn test_set_affiliate_commission_bps_over_max_panics() {
    let (_env, _contract, client, _token, organizer, _contributor) = setup();
    client.set_affiliate_commission_bps(&organizer, &10_001);
}

#[test]
fn test_set_affiliate_commission_bps_at_max_boundary_succeeds() {
    let (_env, _contract, client, _token, organizer, _contributor) = setup();
    client.set_affiliate_commission_bps(&organizer, &10_000);
}

#[test]
#[should_panic]
fn test_non_organizer_cannot_set_commission_panics() {
    let (env, _contract, client, _token, _organizer, _contributor) = setup();
    let impostor = Address::generate(&env);
    client.set_affiliate_commission_bps(&impostor, &500);
}

// ── register_affiliate ────────────────────────────────────────────────────────

#[test]
fn test_register_affiliate_emits_event() {
    let (env, contract, client, _token, organizer, _contributor) = setup();
    let affiliate = Address::generate(&env);

    client.register_affiliate(&organizer, &affiliate);

    let events = env.events().all();
    let (event_contract_id, _topics, data) = events.get(events.len() - 1).unwrap();
    assert_eq!(event_contract_id, contract);
    let data_map: Map<Symbol, Val> = data.try_into_val(&env).unwrap();
    let affiliate_in_event: Address = data_map
        .get(Symbol::new(&env, "affiliate"))
        .unwrap()
        .try_into_val(&env)
        .unwrap();
    assert_eq!(affiliate_in_event, affiliate);

    assert!(client.is_affiliate(&affiliate));
}

#[test]
#[should_panic]
fn test_register_duplicate_affiliate_panics() {
    let (env, _contract, client, _token, organizer, _contributor) = setup();
    let affiliate = Address::generate(&env);
    client.register_affiliate(&organizer, &affiliate);
    client.register_affiliate(&organizer, &affiliate);
}

#[test]
#[should_panic]
fn test_non_organizer_cannot_register_affiliate_panics() {
    let (env, _contract, client, _token, _organizer, _contributor) = setup();
    let impostor = Address::generate(&env);
    let affiliate = Address::generate(&env);
    client.register_affiliate(&impostor, &affiliate);
}

#[test]
fn test_malicious_address_cannot_register_affiliate_storage_rolls_back() {
    let (env, _contract, client, _token, _organizer, _contributor) = setup();
    let impostor = Address::generate(&env);
    let affiliate = Address::generate(&env);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.register_affiliate(&impostor, &affiliate);
    }));
    assert!(result.is_err());
    assert!(!client.is_affiliate(&affiliate));
}

#[test]
fn test_unregistered_address_is_not_affiliate() {
    let (env, _contract, client, _token, _organizer, _contributor) = setup();
    let random = Address::generate(&env);
    assert!(!client.is_affiliate(&random));
}

// ── contribute_with_affiliate: happy path + event verification ───────────────

#[test]
fn test_contribute_with_affiliate_accrues_exact_commission_and_emits_event() {
    let (env, contract, client, token, organizer, contributor) = setup();
    let affiliate = Address::generate(&env);

    client.register_affiliate(&organizer, &affiliate);
    client.set_affiliate_commission_bps(&organizer, &500); // 5%

    StellarAssetClient::new(&env, &token).mint(&contributor, &1_000);
    client.contribute_with_affiliate(&contributor, &1_000, &affiliate);

    let events = env.events().all();
    let (event_contract_id, _topics, data) = events.get(events.len() - 1).unwrap();
    assert_eq!(event_contract_id, contract);
    let data_map: Map<Symbol, Val> = data.try_into_val(&env).unwrap();
    let affiliate_in_event: Address = data_map
        .get(Symbol::new(&env, "affiliate"))
        .unwrap()
        .try_into_val(&env)
        .unwrap();
    let contributor_in_event: Address = data_map
        .get(Symbol::new(&env, "contributor"))
        .unwrap()
        .try_into_val(&env)
        .unwrap();
    let commission_in_event: i128 = data_map
        .get(Symbol::new(&env, "commission"))
        .unwrap()
        .try_into_val(&env)
        .unwrap();
    assert_eq!(affiliate_in_event, affiliate);
    assert_eq!(contributor_in_event, contributor);
    assert_eq!(commission_in_event, 50);

    assert_eq!(client.affiliate_balance(&affiliate), 50);
    assert_eq!(client.pledge_of(&contributor), 1_000);
    assert_eq!(client.raised(), 1_000);
}

#[test]
fn test_contribute_with_affiliate_no_commission_when_rate_unset() {
    let (env, _contract, client, token, organizer, contributor) = setup();
    let affiliate = Address::generate(&env);
    client.register_affiliate(&organizer, &affiliate);

    StellarAssetClient::new(&env, &token).mint(&contributor, &1_000);
    client.contribute_with_affiliate(&contributor, &1_000, &affiliate);

    assert_eq!(client.affiliate_balance(&affiliate), 0);
    assert_eq!(client.raised(), 1_000);
}

#[test]
#[should_panic]
fn test_contribute_with_unregistered_affiliate_panics() {
    let (env, _contract, client, token, _organizer, contributor) = setup();
    let affiliate = Address::generate(&env);
    StellarAssetClient::new(&env, &token).mint(&contributor, &1_000);
    client.contribute_with_affiliate(&contributor, &1_000, &affiliate);
}

#[test]
fn test_contribute_with_unregistered_affiliate_storage_rolls_back() {
    let (env, _contract, client, token, _organizer, contributor) = setup();
    let affiliate = Address::generate(&env);
    StellarAssetClient::new(&env, &token).mint(&contributor, &1_000);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.contribute_with_affiliate(&contributor, &1_000, &affiliate);
    }));
    assert!(result.is_err());
    assert_eq!(client.pledge_of(&contributor), 0);
    assert_eq!(client.raised(), 0);
}

#[test]
#[should_panic]
fn test_contribute_with_affiliate_zero_amount_panics() {
    let (env, _contract, client, _token, organizer, contributor) = setup();
    let affiliate = Address::generate(&env);
    client.register_affiliate(&organizer, &affiliate);
    client.contribute_with_affiliate(&contributor, &0, &affiliate);
}

#[test]
#[should_panic]
fn test_contribute_with_affiliate_after_deadline_panics() {
    let (env, _contract, client, token, organizer, contributor) = setup();
    let affiliate = Address::generate(&env);
    client.register_affiliate(&organizer, &affiliate);
    StellarAssetClient::new(&env, &token).mint(&contributor, &1_000);

    env.ledger().with_mut(|l| l.timestamp += 100_000);
    client.contribute_with_affiliate(&contributor, &1_000, &affiliate);
}

#[test]
fn test_contribute_with_affiliate_accrues_across_multiple_contributors() {
    let (env, _contract, client, token, organizer, contributor) = setup();
    let contributor2 = Address::generate(&env);
    let affiliate = Address::generate(&env);

    client.register_affiliate(&organizer, &affiliate);
    client.set_affiliate_commission_bps(&organizer, &1_000); // 10%

    StellarAssetClient::new(&env, &token).mint(&contributor, &1_000);
    StellarAssetClient::new(&env, &token).mint(&contributor2, &2_000);

    client.contribute_with_affiliate(&contributor, &1_000, &affiliate);
    client.contribute_with_affiliate(&contributor2, &2_000, &affiliate);

    assert_eq!(client.affiliate_balance(&affiliate), 300);
    assert_eq!(client.raised(), 3_000);
}

#[test]
fn test_contribute_with_affiliate_zero_commission_below_rounding_threshold() {
    let (env, _contract, client, token, organizer, contributor) = setup();
    let affiliate = Address::generate(&env);

    client.register_affiliate(&organizer, &affiliate);
    client.set_affiliate_commission_bps(&organizer, &1); // 0.01%

    StellarAssetClient::new(&env, &token).mint(&contributor, &10);
    client.contribute_with_affiliate(&contributor, &10, &affiliate);

    // 10 * 1 / 10_000 == 0 (integer division rounds down); no commission accrues.
    assert_eq!(client.affiliate_balance(&affiliate), 0);
    assert_eq!(client.raised(), 10);
}

// ── claim_affiliate_commission ────────────────────────────────────────────────

#[test]
fn test_claim_affiliate_commission_transfers_and_zeroes_balance() {
    let (env, contract, client, token, organizer, contributor) = setup();
    let affiliate = Address::generate(&env);

    client.register_affiliate(&organizer, &affiliate);
    client.set_affiliate_commission_bps(&organizer, &500);
    StellarAssetClient::new(&env, &token).mint(&contributor, &1_000);
    client.contribute_with_affiliate(&contributor, &1_000, &affiliate);

    client.claim_affiliate_commission(&affiliate);

    assert_eq!(client.affiliate_balance(&affiliate), 0);
    let token_client = soroban_sdk::token::TokenClient::new(&env, &token);
    assert_eq!(token_client.balance(&affiliate), 50);
    assert_eq!(token_client.balance(&contract), 950);
}

#[test]
fn test_claim_affiliate_commission_emits_exact_event_arguments() {
    let (env, contract, client, token, organizer, contributor) = setup();
    let affiliate = Address::generate(&env);

    client.register_affiliate(&organizer, &affiliate);
    client.set_affiliate_commission_bps(&organizer, &500);
    StellarAssetClient::new(&env, &token).mint(&contributor, &1_000);
    client.contribute_with_affiliate(&contributor, &1_000, &affiliate);

    client.claim_affiliate_commission(&affiliate);

    let events = env.events().all();
    let (event_contract_id, _topics, data) = events.get(events.len() - 1).unwrap();
    assert_eq!(event_contract_id, contract);
    let data_map: Map<Symbol, Val> = data.try_into_val(&env).unwrap();
    let affiliate_in_event: Address = data_map
        .get(Symbol::new(&env, "affiliate"))
        .unwrap()
        .try_into_val(&env)
        .unwrap();
    let amount_in_event: i128 = data_map
        .get(Symbol::new(&env, "amount"))
        .unwrap()
        .try_into_val(&env)
        .unwrap();
    assert_eq!(affiliate_in_event, affiliate);
    assert_eq!(amount_in_event, 50);
}

#[test]
#[should_panic]
fn test_claim_with_no_commission_owed_panics() {
    let (env, _contract, client, _token, _organizer, _contributor) = setup();
    let affiliate = Address::generate(&env);
    client.claim_affiliate_commission(&affiliate);
}

#[test]
#[should_panic]
fn test_claim_twice_panics_second_time() {
    let (env, _contract, client, token, organizer, contributor) = setup();
    let affiliate = Address::generate(&env);

    client.register_affiliate(&organizer, &affiliate);
    client.set_affiliate_commission_bps(&organizer, &500);
    StellarAssetClient::new(&env, &token).mint(&contributor, &1_000);
    client.contribute_with_affiliate(&contributor, &1_000, &affiliate);

    client.claim_affiliate_commission(&affiliate);
    client.claim_affiliate_commission(&affiliate);
}

#[test]
fn test_claim_twice_storage_rolls_back_on_second_attempt() {
    let (env, _contract, client, token, organizer, contributor) = setup();
    let affiliate = Address::generate(&env);

    client.register_affiliate(&organizer, &affiliate);
    client.set_affiliate_commission_bps(&organizer, &500);
    StellarAssetClient::new(&env, &token).mint(&contributor, &1_000);
    client.contribute_with_affiliate(&contributor, &1_000, &affiliate);
    client.claim_affiliate_commission(&affiliate);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.claim_affiliate_commission(&affiliate);
    }));
    assert!(result.is_err());
    assert_eq!(client.affiliate_balance(&affiliate), 0);
}

#[test]
fn test_affiliate_balance_isolated_per_affiliate() {
    let (env, _contract, client, token, organizer, contributor) = setup();
    let affiliate_a = Address::generate(&env);
    let affiliate_b = Address::generate(&env);

    client.register_affiliate(&organizer, &affiliate_a);
    client.register_affiliate(&organizer, &affiliate_b);
    client.set_affiliate_commission_bps(&organizer, &500);
    StellarAssetClient::new(&env, &token).mint(&contributor, &1_000);

    client.contribute_with_affiliate(&contributor, &1_000, &affiliate_a);

    assert_eq!(client.affiliate_balance(&affiliate_a), 50);
    assert_eq!(client.affiliate_balance(&affiliate_b), 0);
}
