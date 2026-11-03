use escrow::{EscrowContract, EscrowContractClient, EscrowStatus};
use soroban_sdk::testutils::{Address as _, MockAuth, MockAuthInvoke};
use soroban_sdk::{Address, Env, IntoVal};

fn setup(env: &Env, required_amount: i128) -> (Address, Address, EscrowContractClient<'_>) {
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(env, &contract_id);
    let buyer = Address::generate(env);
    let seller = Address::generate(env);
    client.initialize(&buyer, &seller, &required_amount);
    (contract_id, buyer, client)
}

#[test]
fn test_buyer_can_release_after_funding() {
    let env = Env::default();
    env.mock_all_auths();
    let (_contract_id, _buyer, client) = setup(&env, 1_000);

    client.deposit(&1_000);
    client.release();

    assert_eq!(client.status(), EscrowStatus::Released);
}

#[test]
fn test_non_buyer_cannot_release() {
    let env = Env::default();
    let (contract_id, _buyer, client) = setup(&env, 1_000);
    let seller = client.seller();

    client.deposit(&1_000);

    env.mock_auths(&[MockAuth {
        address: &seller,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "release",
            args: ().into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result = client.try_release();
    assert!(result.is_err());
}

#[test]
fn test_cannot_release_before_funding() {
    let env = Env::default();
    env.mock_all_auths();
    let (_contract_id, _buyer, client) = setup(&env, 1_000);

    let result = client.try_release();
    assert!(result.is_err());
}

#[test]
fn test_status_becomes_released_after_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (_contract_id, _buyer, client) = setup(&env, 1_000);

    client.deposit(&1_000);
    client.release();

    assert_eq!(client.status(), EscrowStatus::Released);
}
