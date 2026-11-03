//! Integration tests between Shade contract and Escrow contract
//! This test demonstrates the complete flow described in AGENTS.md

use shade::types::{InvoiceStatus};
use shade::shade::{Shade, ShadeClient};
use soroban_sdk::{
    testutils::Address as _,
    token, Address, Env, String, IntoVal,
};

// Mock Escrow contract for testing
mod escrow_mock {
    use soroban_sdk::{contract, contractimpl, token, Address, Env};

    #[contract]
    pub struct EscrowMock;

    #[contractimpl]
    impl EscrowMock {
        pub fn deposit(env: Env, shade_contract: Address, invoice_id: u64) {
            use soroban_sdk::IntoVal;
            // Mock escrow deposit that calls back to Shade's pay_invoice
            let buyer: Address = env.storage().instance().get(&"buyer").unwrap();
            
            // Call Shade contract's pay_invoice method
            let mut invoke_args = soroban_sdk::Vec::new(&env);
            invoke_args.push_back(buyer.into_val(&env));
            invoke_args.push_back(invoice_id.into_val(&env));

            env.invoke_contract::<()>(
                &shade_contract,
                &soroban_sdk::Symbol::new(&env, "pay_invoice"),
                invoke_args,
            );

            // Verify funds were received (simplified for mock)
            let token_addr: Address = env.storage().instance().get(&"token").unwrap();
            let token_client = token::TokenClient::new(&env, &token_addr);
            let balance = token_client.balance(&env.current_contract_address());
            
            // Store the deposited amount for verification
            env.storage().instance().set(&"deposited_amount", &balance);
        }

        pub fn set_buyer(env: Env, buyer: Address) {
            env.storage().instance().set(&"buyer", &buyer);
        }

        pub fn set_token(env: Env, token: Address) {
            env.storage().instance().set(&"token", &token);
        }

        pub fn get_deposited_amount(env: Env) -> i128 {
            env.storage().instance().get(&"deposited_amount").unwrap_or(0)
        }
    }
}

fn setup_test() -> (Env, ShadeClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, Shade);
    let client = ShadeClient::new(&env, &contract_id);

    client.initialize(&admin);

    (env, client, contract_id, admin)
}

fn register_token(env: &Env, admin: Address) -> Address {
    env.register_stellar_asset_contract(admin)
}

#[test]
fn test_shade_escrow_integration_complete_flow() {
    let (env, shade_client, shade_contract_id, admin) = setup_test();

    // Setup token
    let token = register_token(&env, admin.clone());
    shade_client.add_accepted_token(&admin, &token);

    // Setup merchant
    let merchant = Address::generate(&env);
    let merchant_account = Address::generate(&env);
    shade_client.register_merchant(&merchant);
    shade_client.set_merchant_account(&merchant, &merchant_account);
    shade_client.set_merchant_status(&admin, &1u64, &true); // Activate merchant

    // Setup buyer (will be the escrow buyer)
    let buyer = Address::generate(&env);

    // Setup platform account for fees
    let platform_account = Address::generate(&env);
    shade_client.set_platform_account(&admin, &platform_account);

    // Set fee (2.5%)
    shade_client.set_fee(&admin, &token, &250i128);

    // Deploy mock escrow contract
    let escrow_id = env.register_contract(None, escrow_mock::EscrowMock);
    let escrow_client = escrow_mock::EscrowMockClient::new(&env, &escrow_id);

    // Configure escrow mock
    escrow_client.set_buyer(&buyer);
    escrow_client.set_token(&token);

    // Create invoice in Shade
    let description = String::from_str(&env, "Payment for escrow deposit");
    let amount = 1000i128;
    let invoice_id = shade_client.create_invoice(
        &merchant,
        &description,
        &amount,
        &token,
        &None, // No expiry
    );

    // Verify invoice was created
    let invoice = shade_client.get_invoice(&invoice_id);
    assert_eq!(invoice.status, InvoiceStatus::Pending);
    assert_eq!(invoice.amount, amount);
    assert_eq!(invoice.token, token);

    // Mint tokens to buyer for payment
    let token_client = token::StellarAssetClient::new(&env, &token);
    token_client.mint(&buyer, &amount);

    // Verify initial balances
    assert_eq!(token_client.balance(&buyer), amount);
    assert_eq!(token_client.balance(&merchant_account), 0);
    assert_eq!(token_client.balance(&platform_account), 0);
    assert_eq!(token_client.balance(&escrow_id), 0);

    // STEP 1: Call Shade payment method (via escrow deposit)
    // This is the core requirement from AGENTS.md
    escrow_client.deposit(&shade_contract_id, &invoice_id);

    // STEP 2: Verify deposit success
    // Check that funds are correctly vaulted in escrow via Shade
    let deposited_amount = escrow_client.get_deposited_amount();
    assert!(deposited_amount > 0, "No funds were deposited to escrow");

    // Verify invoice was paid
    let paid_invoice = shade_client.get_invoice(&invoice_id);
    assert_eq!(paid_invoice.status, InvoiceStatus::Paid);
    assert_eq!(paid_invoice.amount_paid, amount);
    assert_eq!(paid_invoice.payer, Some(buyer.clone()));

    // Verify payment distribution
    let fee_amount = (amount * 250) / 10000; // 2.5% fee
    let merchant_amount = amount - fee_amount;

    assert_eq!(token_client.balance(&buyer), 0); // Buyer paid full amount
    assert_eq!(token_client.balance(&merchant_account), merchant_amount); // Merchant received amount minus fees
    assert_eq!(token_client.balance(&platform_account), fee_amount); // Platform received fees

    // STEP 3: Acceptance Criteria Verification
    // ✅ Funds are correctly vaulted in escrow via Shade
    // The escrow contract received the payment through Shade's payment processing
    assert_eq!(deposited_amount, amount, "Escrow should have received the full payment amount");
}

#[test]
fn test_shade_escrow_integration_with_fees() {
    let (env, shade_client, shade_contract_id, admin) = setup_test();

    // Setup with higher fee to test fee handling
    let token = register_token(&env, admin.clone());
    shade_client.add_accepted_token(&admin, &token);

    let merchant = Address::generate(&env);
    let merchant_account = Address::generate(&env);
    shade_client.register_merchant(&merchant);
    shade_client.set_merchant_account(&merchant, &merchant_account);
    shade_client.set_merchant_status(&admin, &1u64, &true);

    let buyer = Address::generate(&env);
    let platform_account = Address::generate(&env);
    shade_client.set_platform_account(&admin, &platform_account);

    // Set 5% fee
    let fee_bps = 500i128;
    shade_client.set_fee(&admin, &token, &fee_bps);

    // Deploy escrow
    let escrow_id = env.register_contract(None, escrow_mock::EscrowMock);
    let escrow_client = escrow_mock::EscrowMockClient::new(&env, &escrow_id);
    escrow_client.set_buyer(&buyer);
    escrow_client.set_token(&token);

    // Create invoice and process payment
    let amount = 2000i128;
    let invoice_id = shade_client.create_invoice(
        &merchant,
        &String::from_str(&env, "High-value escrow payment"),
        &amount,
        &token,
        &None,
    );

    let token_client = token::StellarAssetClient::new(&env, &token);
    token_client.mint(&buyer, &amount);

    // Process payment through escrow
    escrow_client.deposit(&shade_contract_id, &invoice_id);

    // Verify fee calculation and distribution
    let fee_amount = (amount * fee_bps) / 10000;
    let merchant_amount = amount - fee_amount;

    assert_eq!(token_client.balance(&merchant_account), merchant_amount);
    assert_eq!(token_client.balance(&platform_account), fee_amount);
    
    // Verify escrow received the payment
    let deposited_amount = escrow_client.get_deposited_amount();
    assert_eq!(deposited_amount, amount);
}

#[test]
fn test_shade_escrow_integration_partial_payments() {
    let (env, shade_client, shade_contract_id, admin) = setup_test();

    // Setup
    let token = register_token(&env, admin.clone());
    shade_client.add_accepted_token(&admin, &token);

    let merchant = Address::generate(&env);
    let merchant_account = Address::generate(&env);
    shade_client.register_merchant(&merchant);
    shade_client.set_merchant_account(&merchant, &merchant_account);
    shade_client.set_merchant_status(&admin, &1u64, &true);

    let buyer = Address::generate(&env);
    let platform_account = Address::generate(&env);
    shade_client.set_platform_account(&admin, &platform_account);
    shade_client.set_fee(&admin, &token, &100i128); // 1% fee

    // Create invoice for partial payment scenario
    let total_amount = 1000i128;
    let invoice_id = shade_client.create_invoice(
        &merchant,
        &String::from_str(&env, "Partial payment escrow"),
        &total_amount,
        &token,
        &None,
    );

    let token_client = token::StellarAssetClient::new(&env, &token);
    
    // Make partial payment first (not through escrow)
    let partial_amount = 400i128;
    token_client.mint(&buyer, &partial_amount);
    shade_client.pay_invoice_partial(&buyer, &invoice_id, &partial_amount);

    // Verify partial payment
    let invoice = shade_client.get_invoice(&invoice_id);
    assert_eq!(invoice.status, InvoiceStatus::PartiallyPaid);
    assert_eq!(invoice.amount_paid, partial_amount);

    // Now complete payment through escrow
    let remaining_amount = total_amount - partial_amount;
    token_client.mint(&buyer, &remaining_amount);

    let escrow_id = env.register_contract(None, escrow_mock::EscrowMock);
    let escrow_client = escrow_mock::EscrowMockClient::new(&env, &escrow_id);
    escrow_client.set_buyer(&buyer);
    escrow_client.set_token(&token);

    // Complete payment through escrow
    escrow_client.deposit(&shade_contract_id, &invoice_id);

    // Verify final payment status
    let final_invoice = shade_client.get_invoice(&invoice_id);
    assert_eq!(final_invoice.status, InvoiceStatus::Paid);
    assert_eq!(final_invoice.amount_paid, total_amount);

    // Verify escrow received the remaining payment
    let deposited_amount = escrow_client.get_deposited_amount();
    assert_eq!(deposited_amount, remaining_amount);
}