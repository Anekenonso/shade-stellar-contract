extern crate std;

use crate::shade::{Shade, ShadeClient};
use crate::types::InvoiceStatus;
use account::account::{MerchantAccount, MerchantAccountClient};
use soroban_sdk::testutils::{Address as _, Events, Ledger as _};
use soroban_sdk::{token, Address, Env, Map, String, Symbol, TryIntoVal, Val};

/// Set up a paid invoice with an expiry timestamp.
fn setup_paid_invoice(
    pay_ts: u64,
    expires_at: u64,
) -> (
    Env,
    ShadeClient<'static>,
    Address,
    u64,
    Address,
    Address,
    Address,
) {
    let env = Env::default();
    env.mock_all_auths();

    let shade_id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &shade_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token = token_contract.address();
    client.add_accepted_token(&admin, &token);
    client.set_fee(&admin, &token, &0);

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    let merchant_account_id = env.register(MerchantAccount, ());
    let ma_client = MerchantAccountClient::new(&env, &merchant_account_id);
    ma_client.initialize(&merchant, &shade_id, &1_u64);
    client.set_merchant_account(&merchant, &merchant_account_id);

    let amount = 1_000_i128;
    let description = String::from_str(&env, "Feature 197 Invoice");
    let invoice_id =
        client.create_invoice(&merchant, &description, &amount, &token, &Some(expires_at));

    let buyer = Address::generate(&env);
    let token_mint = token::StellarAssetClient::new(&env, &token);
    token_mint.mint(&buyer, &amount);

    env.ledger().set_timestamp(pay_ts);
    client.pay_invoice(&buyer, &invoice_id);

    (
        env,
        client,
        buyer,
        invoice_id,
        token,
        merchant_account_id,
        merchant,
    )
}

// ===========================================================================
// Happy path — standard execution flow
// ===========================================================================

#[test]
fn test_full_claim_after_expiry() {
    let (env, client, buyer, invoice_id, token, merchant_account_id, _) =
        setup_paid_invoice(1_000, 5_000);

    env.ledger().set_timestamp(6_000);
    client.claim_refund(&buyer, &invoice_id);

    let invoice = client.get_invoice(&invoice_id);
    assert_eq!(invoice.status, InvoiceStatus::Refunded);
    assert_eq!(invoice.amount_refunded, 1_000);

    let tok = token::TokenClient::new(&env, &token);
    assert_eq!(tok.balance(&buyer), 1_000);
    assert_eq!(tok.balance(&merchant_account_id), 0);
}

#[test]
fn test_claim_on_partially_paid_invoice() {
    let env = Env::default();
    env.mock_all_auths();

    let shade_id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &shade_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token = token_contract.address();
    client.add_accepted_token(&admin, &token);
    client.set_fee(&admin, &token, &0);

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    let merchant_account_id = env.register(MerchantAccount, ());
    let ma_client = MerchantAccountClient::new(&env, &merchant_account_id);
    ma_client.initialize(&merchant, &shade_id, &1_u64);
    client.set_merchant_account(&merchant, &merchant_account_id);

    let full_amount = 2_000_i128;
    let partial_amount = 800_i128;
    let description = String::from_str(&env, "Partial Pay Invoice");
    let expires_at = 5_000_u64;
    let invoice_id = client.create_invoice(
        &merchant,
        &description,
        &full_amount,
        &token,
        &Some(expires_at),
    );

    let buyer = Address::generate(&env);
    let token_mint = token::StellarAssetClient::new(&env, &token);
    token_mint.mint(&buyer, &partial_amount);

    env.ledger().set_timestamp(1_000);
    client.pay_invoice_partial(&buyer, &invoice_id, &partial_amount);

    let invoice = client.get_invoice(&invoice_id);
    assert_eq!(invoice.status, InvoiceStatus::PartiallyPaid);
    assert_eq!(invoice.amount_paid, partial_amount);

    env.ledger().set_timestamp(6_000);
    client.claim_refund(&buyer, &invoice_id);

    let invoice = client.get_invoice(&invoice_id);
    assert_eq!(invoice.status, InvoiceStatus::Refunded);
    assert_eq!(invoice.amount_refunded, partial_amount);

    let tok = token::TokenClient::new(&env, &token);
    assert_eq!(tok.balance(&buyer), partial_amount);
    assert_eq!(tok.balance(&merchant_account_id), 0);
}

// ===========================================================================
// Unauthorized access — malicious actors
// ===========================================================================

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn test_random_address_cannot_claim() {
    let (env, client, _buyer, invoice_id, _token, _ma, _merchant) =
        setup_paid_invoice(1_000, 5_000);

    env.ledger().set_timestamp(6_000);
    let random = Address::generate(&env);
    client.claim_refund(&random, &invoice_id);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn test_merchant_cannot_claim() {
    let (env, client, _buyer, invoice_id, _token, _ma, merchant) = setup_paid_invoice(1_000, 5_000);

    env.ledger().set_timestamp(6_000);
    client.claim_refund(&merchant, &invoice_id);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn test_admin_cannot_claim() {
    let env = Env::default();
    env.mock_all_auths();

    let shade_id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &shade_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token = token_contract.address();
    client.add_accepted_token(&admin, &token);
    client.set_fee(&admin, &token, &0);

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    let merchant_account_id = env.register(MerchantAccount, ());
    let ma_client = MerchantAccountClient::new(&env, &merchant_account_id);
    ma_client.initialize(&merchant, &shade_id, &1_u64);
    client.set_merchant_account(&merchant, &merchant_account_id);

    let amount = 1_000_i128;
    let description = String::from_str(&env, "Admin claim test");
    let invoice_id = client.create_invoice(&merchant, &description, &amount, &token, &Some(5_000));

    let buyer = Address::generate(&env);
    let token_mint = token::StellarAssetClient::new(&env, &token);
    token_mint.mint(&buyer, &amount);

    env.ledger().set_timestamp(1_000);
    client.pay_invoice(&buyer, &invoice_id);

    env.ledger().set_timestamp(6_000);
    // Admin tries to claim — should fail with NotAuthorized
    client.claim_refund(&admin, &invoice_id);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn test_different_payer_cannot_claim() {
    let env = Env::default();
    env.mock_all_auths();

    let shade_id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &shade_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token = token_contract.address();
    client.add_accepted_token(&admin, &token);
    client.set_fee(&admin, &token, &0);

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    let merchant_account_id = env.register(MerchantAccount, ());
    let ma_client = MerchantAccountClient::new(&env, &merchant_account_id);
    ma_client.initialize(&merchant, &shade_id, &1_u64);
    client.set_merchant_account(&merchant, &merchant_account_id);

    let amount = 1_000_i128;
    let description = String::from_str(&env, "Diff payer");
    let invoice_id = client.create_invoice(&merchant, &description, &amount, &token, &Some(5_000));

    let buyer = Address::generate(&env);
    let token_mint = token::StellarAssetClient::new(&env, &token);
    token_mint.mint(&buyer, &amount);

    env.ledger().set_timestamp(1_000);
    client.pay_invoice(&buyer, &invoice_id);

    env.ledger().set_timestamp(6_000);
    // A different address tries to claim
    let other = Address::generate(&env);
    client.claim_refund(&other, &invoice_id);
}

// ===========================================================================
// State transition failures
// ===========================================================================

#[test]
#[should_panic(expected = "HostError: Error(Contract, #50)")]
fn test_claim_before_expiry_fails() {
    let (env, client, buyer, invoice_id, _token, _ma, _merchant) = setup_paid_invoice(1_000, 5_000);

    // Still before expiry
    env.ledger().set_timestamp(3_000);
    client.claim_refund(&buyer, &invoice_id);
}

#[test]
fn test_claim_at_exact_expiry_boundary_succeeds() {
    let (env, client, buyer, invoice_id, token, merchant_account_id, _merchant) =
        setup_paid_invoice(1_000, 5_000);

    // Exactly at expiry boundary — the check is `timestamp < expires_at`,
    // so when timestamp == expires_at, it passes the expiry check and succeeds.
    env.ledger().set_timestamp(5_000);
    client.claim_refund(&buyer, &invoice_id);

    let invoice = client.get_invoice(&invoice_id);
    assert_eq!(invoice.status, InvoiceStatus::Refunded);
    assert_eq!(invoice.amount_refunded, 1_000);

    let tok = token::TokenClient::new(&env, &token);
    assert_eq!(tok.balance(&buyer), 1_000);
    assert_eq!(tok.balance(&merchant_account_id), 0);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #51)")]
fn test_double_claim_fails() {
    let (env, client, buyer, invoice_id, _token, _ma, _merchant) = setup_paid_invoice(1_000, 5_000);

    env.ledger().set_timestamp(6_000);
    client.claim_refund(&buyer, &invoice_id);
    // Second claim fails because invoice status is now Refunded (not Paid/PartiallyPaid)
    client.claim_refund(&buyer, &invoice_id);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #27)")]
fn test_claim_without_expiry_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let shade_id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &shade_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token = token_contract.address();
    client.add_accepted_token(&admin, &token);
    client.set_fee(&admin, &token, &0);

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    let merchant_account_id = env.register(MerchantAccount, ());
    let ma_client = MerchantAccountClient::new(&env, &merchant_account_id);
    ma_client.initialize(&merchant, &shade_id, &1_u64);
    client.set_merchant_account(&merchant, &merchant_account_id);

    let amount = 500_i128;
    let description = String::from_str(&env, "No Expiry");
    let invoice_id = client.create_invoice(&merchant, &description, &amount, &token, &None);

    let buyer = Address::generate(&env);
    let token_mint = token::StellarAssetClient::new(&env, &token);
    token_mint.mint(&buyer, &amount);

    env.ledger().set_timestamp(1_000);
    client.pay_invoice(&buyer, &invoice_id);

    env.ledger().set_timestamp(9_999_999);
    client.claim_refund(&buyer, &invoice_id);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #16)")]
fn test_unpaid_invoice_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let shade_id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &shade_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token = token_contract.address();
    client.add_accepted_token(&admin, &token);
    client.set_fee(&admin, &token, &0);

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    let merchant_account_id = env.register(MerchantAccount, ());
    let ma_client = MerchantAccountClient::new(&env, &merchant_account_id);
    ma_client.initialize(&merchant, &shade_id, &1_u64);
    client.set_merchant_account(&merchant, &merchant_account_id);

    let description = String::from_str(&env, "Unpaid");
    let invoice_id = client.create_invoice(&merchant, &description, &500, &token, &Some(5_000));

    let buyer = Address::generate(&env);
    // Never paid — invoice.payer is None, so claim_refund fails with NotAuthorized (#1)
    env.ledger().set_timestamp(6_000);
    client.claim_refund(&buyer, &invoice_id);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #16)")]
fn test_cancelled_invoice_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let shade_id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &shade_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token = token_contract.address();
    client.add_accepted_token(&admin, &token);
    client.set_fee(&admin, &token, &0);

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    let merchant_account_id = env.register(MerchantAccount, ());
    let ma_client = MerchantAccountClient::new(&env, &merchant_account_id);
    ma_client.initialize(&merchant, &shade_id, &1_u64);
    client.set_merchant_account(&merchant, &merchant_account_id);

    let description = String::from_str(&env, "Cancelled");
    let invoice_id = client.create_invoice(&merchant, &description, &500, &token, &Some(5_000));

    client.void_invoice(&merchant, &invoice_id);

    let buyer = Address::generate(&env);
    env.ledger().set_timestamp(6_000);
    // Cancelled invoice has no payer, so fails with NotAuthorized (#1)
    client.claim_refund(&buyer, &invoice_id);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #51)")]
fn test_refunded_invoice_fails() {
    let (env, client, buyer, invoice_id, _token, _ma, _merchant) = setup_paid_invoice(1_000, 5_000);

    env.ledger().set_timestamp(6_000);
    client.claim_refund(&buyer, &invoice_id);
    // Already refunded — second claim fails with InvalidInvoiceStatus (#16)
    // because invoice status is Refunded, not Paid/PartiallyPaid
    client.claim_refund(&buyer, &invoice_id);
}

// ===========================================================================
// Edge cases — boundary values and uninitialized states
// ===========================================================================

#[test]
#[should_panic(expected = "HostError: Error(Contract, #30)")]
fn test_insufficient_merchant_balance_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let shade_id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &shade_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token = token_contract.address();
    client.add_accepted_token(&admin, &token);
    client.set_fee(&admin, &token, &0);

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    // Deploy merchant account but do NOT link it — this will cause
    // the merchant account balance check to fail
    let merchant_account_id = env.register(MerchantAccount, ());
    let ma_client = MerchantAccountClient::new(&env, &merchant_account_id);
    ma_client.initialize(&merchant, &shade_id, &1_u64);
    client.set_merchant_account(&merchant, &merchant_account_id);

    let amount = 1_000_i128;
    let description = String::from_str(&env, "No balance");
    let invoice_id = client.create_invoice(&merchant, &description, &amount, &token, &Some(5_000));

    let buyer = Address::generate(&env);
    let token_mint = token::StellarAssetClient::new(&env, &token);
    token_mint.mint(&buyer, &amount);

    env.ledger().set_timestamp(1_000);
    client.pay_invoice(&buyer, &invoice_id);

    // Drain the merchant account balance
    let tok = token::TokenClient::new(&env, &token);
    tok.transfer(&merchant_account_id, Address::generate(&env), &amount);

    env.ledger().set_timestamp(6_000);
    client.claim_refund(&buyer, &invoice_id);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #8)")]
fn test_nonexistent_invoice_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let shade_id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &shade_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let buyer = Address::generate(&env);
    env.ledger().set_timestamp(1_000);
    client.claim_refund(&buyer, &999_999);
}

#[test]
fn test_claim_with_nonzero_fee() {
    let env = Env::default();
    env.mock_all_auths();

    let shade_id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &shade_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token = token_contract.address();
    client.add_accepted_token(&admin, &token);
    // 10% fee
    client.set_fee(&admin, &token, &1_000);

    let platform_account = Address::generate(&env);
    client.set_platform_account(&admin, &platform_account);

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    let merchant_account_id = env.register(MerchantAccount, ());
    let ma_client = MerchantAccountClient::new(&env, &merchant_account_id);
    ma_client.initialize(&merchant, &shade_id, &1_u64);
    client.set_merchant_account(&merchant, &merchant_account_id);

    let amount = 1_000_i128;
    let description = String::from_str(&env, "With Fee");
    let invoice_id = client.create_invoice(&merchant, &description, &amount, &token, &Some(5_000));

    let buyer = Address::generate(&env);
    let token_mint = token::StellarAssetClient::new(&env, &token);
    token_mint.mint(&buyer, &amount);

    env.ledger().set_timestamp(1_000);
    client.pay_invoice(&buyer, &invoice_id);

    // Fee is 100 (10% of 1000), merchant receives 900, platform gets 100
    let tok = token::TokenClient::new(&env, &token);
    assert_eq!(tok.balance(&merchant_account_id), 900);
    assert_eq!(tok.balance(&platform_account), 100);

    // Fund the merchant account with extra 100 so it can refund the full amount
    token_mint.mint(&merchant_account_id, &100);

    env.ledger().set_timestamp(6_000);
    client.claim_refund(&buyer, &invoice_id);

    let invoice = client.get_invoice(&invoice_id);
    assert_eq!(invoice.status, InvoiceStatus::Refunded);
    // Full amount_paid (1000) refunded even though merchant only received 900
    assert_eq!(invoice.amount_refunded, 1_000);

    // Buyer gets back the full amount
    assert_eq!(tok.balance(&buyer), 1_000);
    // Platform account still holds the fee (not touched by refund)
    assert_eq!(tok.balance(&platform_account), 100);
    // Merchant account is drained (had 900 + 100 funded = 1000, all sent to buyer)
    assert_eq!(tok.balance(&merchant_account_id), 0);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #51)")]
fn test_claim_after_merchant_refund_fails() {
    // Scenario: a merchant issues a full refund (status becomes Refunded).
    // After expiry, the buyer cannot claim_refund because status is Refunded,
    // which is not Paid or PartiallyPaid.
    let (env, client, buyer, invoice_id, _token, _ma, merchant) = setup_paid_invoice(1_000, 5_000);

    // Merchant issues a full refund
    client.refund_invoice(&merchant, &invoice_id);
    let invoice = client.get_invoice(&invoice_id);
    assert_eq!(invoice.status, InvoiceStatus::Refunded);

    // After expiry, buyer tries to claim_refund — but status is Refunded
    env.ledger().set_timestamp(6_000);
    client.claim_refund(&buyer, &invoice_id);
}

// ===========================================================================
// Event emission verification
// ===========================================================================

#[test]
fn test_emits_invoice_expired_refund_event() {
    let (env, client, buyer, invoice_id, token, _ma, _merchant) = setup_paid_invoice(1_000, 5_000);

    env.ledger().set_timestamp(6_000);
    client.claim_refund(&buyer, &invoice_id);

    let events = env.events().all();
    assert!(!events.is_empty(), "No events emitted");

    // `claim_refund` also transfers tokens, so the SAC's own events trail ours.
    // Find the InvoiceExpiredRefundEvent by its topic instead of assuming it is last.
    let wanted = Symbol::new(&env, "invoice_expired_refund_event");
    let mut found = None;
    for i in 0..events.len() {
        let (_c, topics, data) = events.get(i).unwrap();
        if topics.is_empty() {
            continue;
        }
        let name: Result<Symbol, _> = topics.get(0).unwrap().try_into_val(&env);
        if matches!(name, Ok(n) if n == wanted) {
            found = Some(data);
            break;
        }
    }
    let data = found.expect("InvoiceExpiredRefundEvent not emitted");

    let data_map: Map<Symbol, Val> = data.try_into_val(&env).unwrap();

    let invoice_id_val: u64 = data_map
        .get(Symbol::new(&env, "invoice_id"))
        .unwrap()
        .try_into_val(&env)
        .unwrap();
    let buyer_val: Address = data_map
        .get(Symbol::new(&env, "buyer"))
        .unwrap()
        .try_into_val(&env)
        .unwrap();
    let amount_val: i128 = data_map
        .get(Symbol::new(&env, "amount"))
        .unwrap()
        .try_into_val(&env)
        .unwrap();
    let token_val: Address = data_map
        .get(Symbol::new(&env, "token"))
        .unwrap()
        .try_into_val(&env)
        .unwrap();
    let timestamp_val: u64 = data_map
        .get(Symbol::new(&env, "timestamp"))
        .unwrap()
        .try_into_val(&env)
        .unwrap();

    assert_eq!(invoice_id_val, invoice_id);
    assert_eq!(buyer_val, buyer);
    assert_eq!(amount_val, 1_000);
    assert_eq!(token_val, token);
    assert!(timestamp_val > 0);
}

#[test]
fn test_event_emitted_with_partial_payment() {
    let env = Env::default();
    env.mock_all_auths();

    let shade_id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &shade_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token = token_contract.address();
    client.add_accepted_token(&admin, &token);
    client.set_fee(&admin, &token, &0);

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    let merchant_account_id = env.register(MerchantAccount, ());
    let ma_client = MerchantAccountClient::new(&env, &merchant_account_id);
    ma_client.initialize(&merchant, &shade_id, &1_u64);
    client.set_merchant_account(&merchant, &merchant_account_id);

    let full_amount = 2_000_i128;
    let partial_amount = 800_i128;
    let description = String::from_str(&env, "Partial Event");
    let invoice_id =
        client.create_invoice(&merchant, &description, &full_amount, &token, &Some(5_000));

    let buyer = Address::generate(&env);
    let token_mint = token::StellarAssetClient::new(&env, &token);
    token_mint.mint(&buyer, &partial_amount);

    env.ledger().set_timestamp(1_000);
    client.pay_invoice_partial(&buyer, &invoice_id, &partial_amount);

    env.ledger().set_timestamp(6_000);
    client.claim_refund(&buyer, &invoice_id);

    let events = env.events().all();
    let (_event_contract_id, _topics, data) = events.get(events.len() - 1).unwrap();
    let data_map: Map<Symbol, Val> = data.try_into_val(&env).unwrap();

    let amount_val: i128 = data_map
        .get(Symbol::new(&env, "amount"))
        .unwrap()
        .try_into_val(&env)
        .unwrap();
    // Amount should be 800 (only what was paid), not 2000 (full invoice)
    assert_eq!(amount_val, 800);
}

// ===========================================================================
// Storage rollback on panic
// ===========================================================================

#[test]
fn test_storage_unaffected_when_claim_fails_before_expiry() {
    let (env, client, buyer, invoice_id, _token, _ma, _merchant) = setup_paid_invoice(1_000, 5_000);

    // Capture state before failed claim
    let invoice_before = client.get_invoice(&invoice_id);

    env.ledger().set_timestamp(3_000);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.claim_refund(&buyer, &invoice_id);
    }));
    assert!(result.is_err(), "claim_refund should have panicked");

    // State must be unchanged
    let invoice_after = client.get_invoice(&invoice_id);
    assert_eq!(invoice_after.status, invoice_before.status);
    assert_eq!(
        invoice_after.amount_refunded,
        invoice_before.amount_refunded
    );
    assert_eq!(invoice_after.amount_paid, invoice_before.amount_paid);
}

#[test]
fn test_storage_unaffected_when_double_claim_fails() {
    let (env, client, buyer, invoice_id, _token, _ma, _merchant) = setup_paid_invoice(1_000, 5_000);

    env.ledger().set_timestamp(6_000);
    client.claim_refund(&buyer, &invoice_id);

    let invoice_after_first = client.get_invoice(&invoice_id);
    assert_eq!(invoice_after_first.status, InvoiceStatus::Refunded);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.claim_refund(&buyer, &invoice_id);
    }));
    assert!(result.is_err(), "double claim should have panicked");

    // State must still show Refunded, not changed further
    let invoice_after_second = client.get_invoice(&invoice_id);
    assert_eq!(invoice_after_second.status, InvoiceStatus::Refunded);
    assert_eq!(invoice_after_second.amount_refunded, 1_000);
}

#[test]
fn test_storage_unaffected_when_merchant_balance_insufficient() {
    let env = Env::default();
    env.mock_all_auths();

    let shade_id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &shade_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token = token_contract.address();
    client.add_accepted_token(&admin, &token);
    client.set_fee(&admin, &token, &0);

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    let merchant_account_id = env.register(MerchantAccount, ());
    let ma_client = MerchantAccountClient::new(&env, &merchant_account_id);
    ma_client.initialize(&merchant, &shade_id, &1_u64);
    client.set_merchant_account(&merchant, &merchant_account_id);

    let amount = 1_000_i128;
    let description = String::from_str(&env, "Drain test");
    let invoice_id = client.create_invoice(&merchant, &description, &amount, &token, &Some(5_000));

    let buyer = Address::generate(&env);
    let token_mint = token::StellarAssetClient::new(&env, &token);
    token_mint.mint(&buyer, &amount);

    env.ledger().set_timestamp(1_000);
    client.pay_invoice(&buyer, &invoice_id);

    let invoice_before = client.get_invoice(&invoice_id);

    // Drain merchant account
    let tok = token::TokenClient::new(&env, &token);
    tok.transfer(&merchant_account_id, Address::generate(&env), &amount);

    env.ledger().set_timestamp(6_000);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.claim_refund(&buyer, &invoice_id);
    }));
    assert!(
        result.is_err(),
        "claim should have panicked due to insufficient balance"
    );

    // Invoice state must be unchanged
    let invoice_after = client.get_invoice(&invoice_id);
    assert_eq!(invoice_after.status, invoice_before.status);
    assert_eq!(
        invoice_after.amount_refunded,
        invoice_before.amount_refunded
    );
}

#[test]
fn test_storage_unaffected_when_random_address_claims() {
    let (env, client, _buyer, invoice_id, _token, _ma, _merchant) =
        setup_paid_invoice(1_000, 5_000);

    let invoice_before = client.get_invoice(&invoice_id);

    env.ledger().set_timestamp(6_000);
    let random = Address::generate(&env);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.claim_refund(&random, &invoice_id);
    }));
    assert!(
        result.is_err(),
        "random address should not be able to claim"
    );

    let invoice_after = client.get_invoice(&invoice_id);
    assert_eq!(invoice_after.status, invoice_before.status);
    assert_eq!(invoice_after.amount_paid, invoice_before.amount_paid);
    assert_eq!(
        invoice_after.amount_refunded,
        invoice_before.amount_refunded
    );
}
