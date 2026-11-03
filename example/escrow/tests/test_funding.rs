use escrow::{EscrowContract, EscrowContractClient, EscrowStatus};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env};

fn setup(env: &Env, required_amount: i128) -> EscrowContractClient<'_> {
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(env, &contract_id);
    let buyer = Address::generate(env);
    let seller = Address::generate(env);
    client.initialize(&buyer, &seller, &required_amount);
    client
}

#[test]
fn test_partial_deposit_keeps_created_status() {
    let env = Env::default();
    env.mock_all_auths();
    let client = setup(&env, 1_000);

    client.deposit(&400);

    assert_eq!(client.deposited_amount(), 400);
    assert_eq!(client.status(), EscrowStatus::Created);
}

#[test]
fn test_full_deposit_sets_funded_status() {
    let env = Env::default();
    env.mock_all_auths();
    let client = setup(&env, 1_000);

    client.deposit(&1_000);

    assert_eq!(client.deposited_amount(), 1_000);
    assert_eq!(client.status(), EscrowStatus::Funded);
}

#[test]
fn test_multiple_deposits_accumulate() {
    let env = Env::default();
    env.mock_all_auths();
    let client = setup(&env, 1_000);

    client.deposit(&200);
    client.deposit(&300);
    client.deposit(&500);

    assert_eq!(client.deposited_amount(), 1_000);
    assert_eq!(client.status(), EscrowStatus::Funded);
}
