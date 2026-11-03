//! Integration tests for Shade contract payment processing into escrow

use crate::{EscrowContract, EscrowContractClient, EscrowStatus};
use soroban_sdk::{
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, Ledger},
    token, Address, Env, String,
};

// Mock Shade contract for testing
mod shade_mock {
    use soroban_sdk::{contract, contractimpl, token, Address, Env};

    #[contract]
    pub struct ShadeMock;

    #[contractimpl]
    impl ShadeMock {
        pub fn pay_invoice(env: Env, payer: Address, invoice_id: u64) {
            // Mock implementation that transfers tokens to the calling contract (escrow)
            let token_addr = env.storage().instance().get(&"token").unwrap();
            let amount: i128 = env.storage().instance().get(&"amount").unwrap();
            
            let token_client = token::TokenClient::new(&env, &token_addr);
            token_client.transfer(&payer, &env.current_contract_address(), &amount);
        }

        pub fn set_test_params(env: Env, token: Address, amount: i128) {
            env.storage().instance().set(&"token", &token);
            env.storage().instance().set(&"amount", &amount);
        }
    }
}

fn create_token_contract<'a>(env: &Env, admin: &Address) -> (Address, token::StellarAssetClient<'a>) {
    let token_address = env.register_stellar_asset_contract(admin.clone());
    (
        token_address.clone(),
        token::StellarAssetClient::new(env, &token_address),
    )
}

#[test]
fn test_shade_escrow_integration_success() {
    let env = Env::default();
    env.mock_all_auths();

    // Create addresses
    let buyer = Address::generate(&env);
    let seller = Address::generate(&env);
    let arbiter = Address::generate(&env);
    let platform_account = Address::generate(&env);

    // Create token
    let (token_addr, token_client) = create_token_contract(&env, &buyer);

    // Deploy escrow contract
    let escrow_id = env.register_contract(None, EscrowContract);
    let escrow_client = EscrowContractClient::new(&env, &escrow_id);

    // Deploy mock Shade contract
    let shade_id = env.register_contract(None, shade_mock::ShadeMock);
    let shade_client = shade_mock::ShadeMockClient::new(&env, &shade_id);

    // Initialize escrow
    let terms = String::from_str(&env, "Test escrow terms");
    let total_amount = 1000i128;
    let fee_percentage_bps = 250u32; // 2.5%

    escrow_client.init(
        &buyer,
        &seller,
        &arbiter,
        &terms,
        &token_addr,
        &total_amount,
        &fee_percentage_bps,
        &platform_account,
    );

    // Set up mock Shade contract
    shade_client.set_test_params(&token_addr, &total_amount);

    // Mint tokens to buyer
    token_client.mint(&buyer, &total_amount);

    // Verify initial state
    assert_eq!(escrow_client.status(), EscrowStatus::Pending);
    assert_eq!(token_client.balance(&escrow_id), 0);

    // Execute deposit through Shade integration
    let invoice_id = 123u64;
    escrow_client.deposit(&shade_id, &invoice_id);

    // Verify deposit success
    assert_eq!(token_client.balance(&escrow_id), total_amount);
    assert_eq!(token_client.balance(&buyer), 0);

    // Verify escrow status remains pending (ready for milestone releases)
    assert_eq!(escrow_client.status(), EscrowStatus::Pending);
}

#[test]
fn test_shade_escrow_integration_insufficient_deposit() {
    let env = Env::default();
    env.mock_all_auths();

    // Create addresses
    let buyer = Address::generate(&env);
    let seller = Address::generate(&env);
    let arbiter = Address::generate(&env);
    let platform_account = Address::generate(&env);

    // Create token
    let (token_addr, token_client) = create_token_contract(&env, &buyer);

    // Deploy escrow contract
    let escrow_id = env.register_contract(None, EscrowContract);
    let escrow_client = EscrowContractClient::new(&env, &escrow_id);

    // Deploy mock Shade contract
    let shade_id = env.register_contract(None, shade_mock::ShadeMock);
    let shade_client = shade_mock::ShadeMockClient::new(&env, &shade_id);

    // Initialize escrow
    let terms = String::from_str(&env, "Test escrow terms");
    let total_amount = 1000i128;
    let insufficient_amount = 500i128; // Less than required
    let fee_percentage_bps = 250u32;

    escrow_client.init(
        &buyer,
        &seller,
        &arbiter,
        &terms,
        &token_addr,
        &total_amount,
        &fee_percentage_bps,
        &platform_account,
    );

    // Set up mock Shade contract with insufficient amount
    shade_client.set_test_params(&token_addr, &insufficient_amount);

    // Mint tokens to buyer
    token_client.mint(&buyer, &insufficient_amount);

    // Attempt deposit - should fail due to insufficient amount
    let invoice_id = 123u64;
    let result = escrow_client.try_deposit(&shade_id, &invoice_id);
    
    // Should panic with DepositFailed error
    assert!(result.is_err());
}

#[test]
fn test_shade_escrow_integration_wrong_status() {
    let env = Env::default();
    env.mock_all_auths();

    // Create addresses
    let buyer = Address::generate(&env);
    let seller = Address::generate(&env);
    let arbiter = Address::generate(&env);
    let platform_account = Address::generate(&env);

    // Create token
    let (token_addr, token_client) = create_token_contract(&env, &buyer);

    // Deploy escrow contract
    let escrow_id = env.register_contract(None, EscrowContract);
    let escrow_client = EscrowContractClient::new(&env, &escrow_id);

    // Deploy mock Shade contract
    let shade_id = env.register_contract(None, shade_mock::ShadeMock);
    let shade_client = shade_mock::ShadeMockClient::new(&env, &shade_id);

    // Initialize escrow
    let terms = String::from_str(&env, "Test escrow terms");
    let total_amount = 1000i128;
    let fee_percentage_bps = 250u32;

    escrow_client.init(
        &buyer,
        &seller,
        &arbiter,
        &terms,
        &token_addr,
        &total_amount,
        &fee_percentage_bps,
        &platform_account,
    );

    // Set up mock Shade contract
    shade_client.set_test_params(&token_addr, &total_amount);

    // Mint tokens and make initial deposit
    token_client.mint(&buyer, &total_amount);
    let invoice_id = 123u64;
    escrow_client.deposit(&shade_id, &invoice_id);

    // Approve release to change status
    escrow_client.approve_release();

    // Try to deposit again - should fail due to wrong status
    token_client.mint(&buyer, &total_amount);
    let result = escrow_client.try_deposit(&shade_id, &invoice_id);
    
    // Should panic with InvalidStatus error
    assert!(result.is_err());
}

#[test]
fn test_shade_escrow_integration_with_milestones() {
    let env = Env::default();
    env.mock_all_auths();

    // Create addresses
    let buyer = Address::generate(&env);
    let seller = Address::generate(&env);
    let arbiter = Address::generate(&env);
    let platform_account = Address::generate(&env);

    // Create token
    let (token_addr, token_client) = create_token_contract(&env, &buyer);

    // Deploy escrow contract
    let escrow_id = env.register_contract(None, EscrowContract);
    let escrow_client = EscrowContractClient::new(&env, &escrow_id);

    // Deploy mock Shade contract
    let shade_id = env.register_contract(None, shade_mock::ShadeMock);
    let shade_client = shade_mock::ShadeMockClient::new(&env, &shade_id);

    // Initialize escrow
    let terms = String::from_str(&env, "Test escrow with milestones");
    let total_amount = 1000i128;
    let fee_percentage_bps = 250u32;

    escrow_client.init(
        &buyer,
        &seller,
        &arbiter,
        &terms,
        &token_addr,
        &total_amount,
        &fee_percentage_bps,
        &platform_account,
    );

    // Add milestones
    use crate::Milestone;
    let milestone1 = Milestone {
        description: String::from_str(&env, "First milestone"),
        percentage_bps: 5000u32, // 50%
    };
    let milestone2 = Milestone {
        description: String::from_str(&env, "Second milestone"),
        percentage_bps: 5000u32, // 50%
    };

    escrow_client.add_milestone(&seller, &milestone1);
    escrow_client.add_milestone(&seller, &milestone2);

    // Set up mock Shade contract and make deposit
    shade_client.set_test_params(&token_addr, &total_amount);
    token_client.mint(&buyer, &total_amount);

    let invoice_id = 123u64;
    escrow_client.deposit(&shade_id, &invoice_id);

    // Verify deposit success
    assert_eq!(token_client.balance(&escrow_id), total_amount);

    // Test milestone releases work after Shade deposit
    let initial_seller_balance = token_client.balance(&seller);
    
    // Release first milestone
    escrow_client.approve_milestone_release(&0u32);
    
    // Verify partial payment (50% minus fees)
    let expected_milestone_amount = (total_amount * 5000) / 10000; // 50%
    let fee_amount = (expected_milestone_amount * fee_percentage_bps as i128) / 10000;
    let net_amount = expected_milestone_amount - fee_amount;
    
    assert_eq!(
        token_client.balance(&seller),
        initial_seller_balance + net_amount
    );
    assert_eq!(
        token_client.balance(&platform_account),
        fee_amount
    );
}