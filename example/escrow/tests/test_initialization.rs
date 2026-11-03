use escrow::{EscrowContract, EscrowContractClient, EscrowStatus};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env};

#[test]
fn test_escrow_creation_initial_state() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let buyer = Address::generate(&env);
    let seller = Address::generate(&env);
    let required_amount = 1_000i128;

    client.initialize(&buyer, &seller, &required_amount);

    assert_eq!(client.buyer(), buyer);
    assert_eq!(client.seller(), seller);
    assert_eq!(client.required_amount(), required_amount);
    assert_eq!(client.deposited_amount(), 0);
    assert_eq!(client.status(), EscrowStatus::Created);
}
