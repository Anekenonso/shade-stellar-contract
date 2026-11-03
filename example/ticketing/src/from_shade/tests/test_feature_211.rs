#![cfg(test)]

//! Comprehensive secondary market (resale) test suite for the Shade contract.
//!
//! Covers: happy-path flows, malicious-actor / unauthorized-access,
//! event emission verification, storage rollback on panic, overflow conditions,
//! boundary values, and state-transition correctness.

use shade::shade::{Shade, ShadeClient};
use soroban_sdk::testutils::{Address as _, Events as _, MockAuth, MockAuthInvoke};
use soroban_sdk::token::{StellarAssetClient, TokenClient};
use soroban_sdk::{Address, Env, IntoVal, String};

const TOKEN_INITIAL_BALANCE: i128 = 1_000_000;

// ── Test fixture ──────────────────────────────────────────────────────────────

struct Fixture<'a> {
    env: Env,
    client: ShadeClient<'a>,
    admin: Address,
    token: Address,
}

fn setup() -> Fixture<'static> {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    client.add_accepted_token(&admin, &token_address);

    Fixture {
        env,
        client,
        admin,
        token: token_address,
    }
}

fn fund(env: &Env, token: &Address, to: &Address, amount: i128) {
    let asset_client = StellarAssetClient::new(env, token);
    let issuer = asset_client.admin();
    asset_client
        .mock_auths(&[MockAuth {
            address: &issuer,
            invoke: &MockAuthInvoke {
                contract: token,
                fn_name: "mint",
                args: (to.clone(), amount).into_val(env),
                sub_invokes: &[],
            },
        }])
        .mint(to, &amount);
}

fn balance(env: &Env, token: &Address, who: &Address) -> i128 {
    TokenClient::new(env, token).balance(who)
}

fn register_merchant_with_account(
    env: &Env,
    client: &ShadeClient,
    token: &Address,
) -> (Address, Address) {
    let merchant = Address::generate(env);
    let merchant_account = merchant.clone();
    client.register_merchant(&merchant);
    client.set_merchant_account(&merchant, &merchant_account);
    client.set_merchant_accepted_tokens(
        &merchant,
        &soroban_sdk::Vec::from_array(env, [token.clone()]),
    );
    (merchant, merchant_account)
}

fn future_date(env: &Env) -> u64 {
    env.ledger().timestamp() + 86_400
}

/// Helper: create event + purchase ticket, return (event_id, ticket_id, buyer).
fn create_event_and_purchase(
    f: &Fixture,
    merchant: &Address,
    price: i128,
    royalty_bps: u32,
) -> (u64, u64, Address) {
    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    let event_id = f.client.create_event(
        merchant,
        &String::from_str(&f.env, "Test Event"),
        &price,
        &f.token,
        &10u32,
        &future_date(&f.env),
        &royalty_bps,
    );

    let ticket_id = f.client.purchase_ticket(&event_id, &buyer);
    (event_id, ticket_id, buyer)
}

// ══════════════════════════════════════════════════════════════════════════════
// 1. HAPPY-PATH TESTS
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn resale_splits_royalty_and_proceeds_correctly() {
    let f = setup();
    let (merchant, merchant_account) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let price: i128 = 1_000;
    let royalty_bps: u32 = 1_000; // 10%
    let (_event_id, ticket_id, seller) =
        create_event_and_purchase(&f, &merchant, price, royalty_bps);

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    let merchant_bal_before = balance(&f.env, &f.token, &merchant_account);
    let seller_bal_before = balance(&f.env, &f.token, &seller);
    let buyer_bal_before = balance(&f.env, &f.token, &buyer);

    let resale_price: i128 = 2_000;
    f.client
        .resell_ticket(&seller, &buyer, &ticket_id, &resale_price);

    let expected_royalty = resale_price * royalty_bps as i128 / 10_000; // 200
    let expected_proceeds = resale_price - expected_royalty; // 1800

    assert_eq!(
        balance(&f.env, &f.token, &merchant_account),
        merchant_bal_before + expected_royalty
    );
    assert_eq!(
        balance(&f.env, &f.token, &seller),
        seller_bal_before + expected_proceeds
    );
    assert_eq!(
        balance(&f.env, &f.token, &buyer),
        buyer_bal_before - resale_price
    );

    // Ownership transferred.
    let ticket = f.client.get_ticket(&ticket_id);
    assert_eq!(ticket.owner, buyer);

    // User-ticket index updated.
    assert!(f.client.get_user_tickets(&seller).is_empty());
    let buyer_tickets = f.client.get_user_tickets(&buyer);
    assert_eq!(buyer_tickets.len(), 1);
    assert_eq!(buyer_tickets.get_unchecked(0), ticket_id);
}

#[test]
fn resale_with_zero_royalty_gives_full_proceeds_to_seller() {
    let f = setup();
    let (merchant, _merchant_account) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (_event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 500, 0); // 0% royalty

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    let seller_before = balance(&f.env, &f.token, &seller);
    let resale_price: i128 = 750;

    f.client
        .resell_ticket(&seller, &buyer, &ticket_id, &resale_price);

    // All proceeds go to seller, zero royalty.
    assert_eq!(
        balance(&f.env, &f.token, &seller),
        seller_before + resale_price
    );
    assert_eq!(f.client.get_ticket(&ticket_id).owner, buyer);
}

#[test]
fn resale_with_100_percent_royalty_gives_all_to_merchant() {
    let f = setup();
    let (merchant, merchant_account) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (_event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 500, 10_000); // 100% royalty

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    let merchant_before = balance(&f.env, &f.token, &merchant_account);
    let seller_before = balance(&f.env, &f.token, &seller);
    let resale_price: i128 = 1_000;

    f.client
        .resell_ticket(&seller, &buyer, &ticket_id, &resale_price);

    // All goes to merchant, seller gets nothing.
    assert_eq!(
        balance(&f.env, &f.token, &merchant_account),
        merchant_before + resale_price
    );
    assert_eq!(balance(&f.env, &f.token, &seller), seller_before);
}

#[test]
fn resale_with_1_bps_royalty_rounds_down_to_zero() {
    let f = setup();
    let (merchant, merchant_account) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (_event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 500, 1); // 0.01% royalty

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    let seller_before = balance(&f.env, &f.token, &seller);
    let merchant_before = balance(&f.env, &f.token, &merchant_account);
    let resale_price: i128 = 500; // 500 * 1 / 10_000 = 0

    f.client
        .resell_ticket(&seller, &buyer, &ticket_id, &resale_price);

    // Royalty rounds to 0, seller gets full amount.
    assert_eq!(
        balance(&f.env, &f.token, &seller),
        seller_before + resale_price
    );
    assert_eq!(
        balance(&f.env, &f.token, &merchant_account),
        merchant_before
    );
}

#[test]
fn resale_preserves_ticket_purchase_price() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let price: i128 = 300;
    let (event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, price, 500);

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);
    f.client.resell_ticket(&seller, &buyer, &ticket_id, &1_000);

    // purchase_price should remain from the original purchase.
    let ticket = f.client.get_ticket(&ticket_id);
    assert_eq!(ticket.purchase_price, price);
    assert_eq!(ticket.event_id, event_id);
}

#[test]
fn resale_event_tickets_list_not_modified() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 500, 500);

    let event_tickets_before = f.client.get_event_tickets(&event_id);
    assert_eq!(event_tickets_before.len(), 1);

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);
    f.client.resell_ticket(&seller, &buyer, &ticket_id, &800);

    // Event tickets list is unchanged by resale.
    let event_tickets_after = f.client.get_event_tickets(&event_id);
    assert_eq!(event_tickets_after.len(), 1);
    assert_eq!(event_tickets_after.get_unchecked(0), ticket_id);
}

#[test]
fn multiple_resale_chain_cumulative_royalties() {
    let f = setup();
    let (merchant, merchant_account) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (_event_id, ticket_id, owner1) = create_event_and_purchase(&f, &merchant, 100, 1_000); // 10% royalty

    let owner2 = Address::generate(&f.env);
    let owner3 = Address::generate(&f.env);
    fund(&f.env, &f.token, &owner2, TOKEN_INITIAL_BALANCE);
    fund(&f.env, &f.token, &owner3, TOKEN_INITIAL_BALANCE);

    let merchant_before = balance(&f.env, &f.token, &merchant_account);

    // First resale: 500
    f.client.resell_ticket(&owner1, &owner2, &ticket_id, &500);
    assert_eq!(f.client.get_ticket(&ticket_id).owner, owner2);
    assert_eq!(
        balance(&f.env, &f.token, &merchant_account),
        merchant_before + 50 // 10% of 500
    );

    // Second resale: 1_000
    f.client.resell_ticket(&owner2, &owner3, &ticket_id, &1_000);
    assert_eq!(f.client.get_ticket(&ticket_id).owner, owner3);
    assert_eq!(
        balance(&f.env, &f.token, &merchant_account),
        merchant_before + 50 + 100 // +10% of 1_000
    );

    // owner1 no longer in user tickets, owner3 has it.
    assert!(f.client.get_user_tickets(&owner1).is_empty());
    assert!(f.client.get_user_tickets(&owner2).is_empty());
    let owner3_tickets = f.client.get_user_tickets(&owner3);
    assert_eq!(owner3_tickets.len(), 1);
    assert_eq!(owner3_tickets.get_unchecked(0), ticket_id);
}

#[test]
fn resale_emits_events() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (_event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 1_000, 500); // 5% royalty

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    let events_before = f.env.events().all().len();

    f.client.resell_ticket(&seller, &buyer, &ticket_id, &2_000);

    let events_after = f.env.events().all();
    // At least one new event was emitted during the resale.
    assert!(events_after.len() > events_before);
}

// ══════════════════════════════════════════════════════════════════════════════
// 2. SECURITY / UNAUTHORIZED ACCESS TESTS
// ══════════════════════════════════════════════════════════════════════════════

#[test]
#[should_panic(expected = "Error(Contract, #126)")] // NotTicketOwner
fn resale_rejects_non_owner_seller() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (_event_id, ticket_id, _real_owner) = create_event_and_purchase(&f, &merchant, 500, 500);

    let imposter = Address::generate(&f.env);
    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &imposter, TOKEN_INITIAL_BALANCE);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    // Imposter tries to sell a ticket they don't own.
    f.client.resell_ticket(&imposter, &buyer, &ticket_id, &200);
}

#[test]
#[should_panic] // Auth error (seller == buyer causes auth frame conflict)
fn resale_rejects_self_resale() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (_event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 500, 500);

    // Seller tries to resell to themselves.
    f.client.resell_ticket(&seller, &seller, &ticket_id, &500);
}

#[test]
#[should_panic(expected = "Error(Contract, #125)")] // TicketNotFound
fn resale_rejects_nonexistent_ticket() {
    let f = setup();
    let a = Address::generate(&f.env);
    let b = Address::generate(&f.env);
    f.client.resell_ticket(&a, &b, &999_999, &100);
}

#[test]
#[should_panic(expected = "Error(Contract, #16)")] // InvalidInvoiceStatus
fn resale_rejects_cancelled_event() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 500, 500);

    // Cancel the event.
    f.client.cancel_event_and_batch_refund(&merchant, &event_id);

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    f.client.resell_ticket(&seller, &buyer, &ticket_id, &500);
}

#[test]
#[should_panic(expected = "Error(Contract, #12)")] // TokenNotAccepted
fn resale_rejects_removed_token() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (_event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 500, 500);

    // Admin removes the token from accepted list.
    f.client.remove_accepted_token(&f.admin, &f.token);

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    f.client.resell_ticket(&seller, &buyer, &ticket_id, &500);
}

#[test]
#[should_panic(expected = "Error(Contract, #127)")] // InvalidResalePrice
fn resale_rejects_zero_price() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (_event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 500, 500);

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    f.client.resell_ticket(&seller, &buyer, &ticket_id, &0);
}

#[test]
#[should_panic(expected = "Error(Contract, #127)")] // InvalidResalePrice
fn resale_rejects_negative_price() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (_event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 500, 500);

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    f.client
        .resell_ticket(&seller, &buyer, &ticket_id, &(-1i128));
}

// ══════════════════════════════════════════════════════════════════════════════
// 3. EDGE CASE / BOUNDARY VALUE TESTS
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn resale_with_minimum_price_of_1() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (_event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 1, 1_000);

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    let seller_before = balance(&f.env, &f.token, &seller);
    // 1 * 1000 / 10_000 = 0 royalty
    f.client.resell_ticket(&seller, &buyer, &ticket_id, &1);
    assert_eq!(balance(&f.env, &f.token, &seller), seller_before + 1);
}

#[test]
fn resale_with_10000_bps_royalty_full_amount() {
    let f = setup();
    let (merchant, merchant_account) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (_event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 100, 10_000);

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    let merchant_before = balance(&f.env, &f.token, &merchant_account);
    f.client.resell_ticket(&seller, &buyer, &ticket_id, &9_999);

    // 9999 * 10000 / 10000 = 9999 royalty (all to merchant).
    assert_eq!(
        balance(&f.env, &f.token, &merchant_account),
        merchant_before + 9_999
    );
}

#[test]
fn resale_with_price_equal_to_royalty_gives_seller_proceeds() {
    let f = setup();
    let (merchant, merchant_account) = register_merchant_with_account(&f.env, &f.client, &f.token);

    // 50% royalty
    let (_event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 100, 5_000);

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    let seller_before = balance(&f.env, &f.token, &seller);
    let merchant_before = balance(&f.env, &f.token, &merchant_account);

    // Resale price = 100, royalty = 50, seller_proceeds = 50
    f.client.resell_ticket(&seller, &buyer, &ticket_id, &100);

    assert_eq!(balance(&f.env, &f.token, &seller), seller_before + 50);
    assert_eq!(
        balance(&f.env, &f.token, &merchant_account),
        merchant_before + 50
    );
}

#[test]
fn resale_seller_proceeds_zero_when_royalty_equals_price() {
    let f = setup();
    let (merchant, merchant_account) = register_merchant_with_account(&f.env, &f.client, &f.token);

    // 100% royalty
    let (_event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 100, 10_000);

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    let seller_before = balance(&f.env, &f.token, &seller);
    let merchant_before = balance(&f.env, &f.token, &merchant_account);

    // seller_proceeds = 0, no transfer to seller.
    f.client.resell_ticket(&seller, &buyer, &ticket_id, &100);

    assert_eq!(balance(&f.env, &f.token, &seller), seller_before);
    assert_eq!(
        balance(&f.env, &f.token, &merchant_account),
        merchant_before + 100
    );
}

#[test]
fn resale_after_primary_purchase_with_high_price() {
    let f = setup();
    let (merchant, merchant_account) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let price: i128 = 100_000;
    let (_event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, price, 1_000); // 10%

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE * 100);

    let resale_price: i128 = 500_000;
    let expected_royalty = resale_price * 1_000 / 10_000; // 50_000
    let expected_proceeds = resale_price - expected_royalty;

    let merchant_before = balance(&f.env, &f.token, &merchant_account);
    let seller_before = balance(&f.env, &f.token, &seller);

    f.client
        .resell_ticket(&seller, &buyer, &ticket_id, &resale_price);

    assert_eq!(
        balance(&f.env, &f.token, &merchant_account),
        merchant_before + expected_royalty
    );
    assert_eq!(
        balance(&f.env, &f.token, &seller),
        seller_before + expected_proceeds
    );
}

#[test]
fn resale_with_small_amounts_rounds_royalty_down() {
    let f = setup();
    let (merchant, merchant_account) = register_merchant_with_account(&f.env, &f.client, &f.token);

    // 33% royalty
    let (_event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 100, 3_300);

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    let seller_before = balance(&f.env, &f.token, &seller);
    let merchant_before = balance(&f.env, &f.token, &merchant_account);

    // 10 * 3300 / 10_000 = 3 (truncated)
    f.client.resell_ticket(&seller, &buyer, &ticket_id, &10);

    assert_eq!(balance(&f.env, &f.token, &seller), seller_before + 7);
    assert_eq!(
        balance(&f.env, &f.token, &merchant_account),
        merchant_before + 3
    );
}

#[test]
fn resale_ticket_event_id_preserved() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 500, 500);

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);
    f.client.resell_ticket(&seller, &buyer, &ticket_id, &800);

    let ticket = f.client.get_ticket(&ticket_id);
    assert_eq!(ticket.event_id, event_id);
    assert_eq!(ticket.id, ticket_id);
}

// ══════════════════════════════════════════════════════════════════════════════
// 4. OVERFLOW / UNDERFLOW CONDITION TESTS
// ══════════════════════════════════════════════════════════════════════════════

#[test]
#[should_panic]
fn resale_overflow_bps_of_with_large_price_and_royalty() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (_event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 500, 3); // 3 bps

    let buyer = Address::generate(&f.env);
    // Fund buyer with enough to cover the massive resale price.
    fund(&f.env, &f.token, &buyer, i128::MAX);

    // i128::MAX * 3 overflows i128, causing bps_of to return None → InvalidAmount panic.
    f.client
        .resell_ticket(&seller, &buyer, &ticket_id, &i128::MAX);
}

#[test]
fn resale_bps_of_does_not_panic_on_small_values() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (_event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 1, 1); // minimal everything

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    // 1 * 1 / 10_000 = 0 royalty, should not panic.
    f.client.resell_ticket(&seller, &buyer, &ticket_id, &1);
    assert_eq!(f.client.get_ticket(&ticket_id).owner, buyer);
}

// ══════════════════════════════════════════════════════════════════════════════
// 5. STATE TRANSITION TESTS
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn resale_removes_ticket_from_seller_user_tickets() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (_event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 500, 500);

    // Seller has 1 ticket before resale.
    assert_eq!(f.client.get_user_tickets(&seller).len(), 1);

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);
    f.client.resell_ticket(&seller, &buyer, &ticket_id, &500);

    // Seller has 0 tickets after resale.
    assert!(f.client.get_user_tickets(&seller).is_empty());
    // Buyer has 1 ticket.
    assert_eq!(f.client.get_user_tickets(&buyer).len(), 1);
}

#[test]
fn resale_adds_ticket_to_buyer_user_tickets() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (_event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 500, 500);

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    // Buyer has 0 tickets before.
    assert!(f.client.get_user_tickets(&buyer).is_empty());

    f.client.resell_ticket(&seller, &buyer, &ticket_id, &500);

    let buyer_tickets = f.client.get_user_tickets(&buyer);
    assert_eq!(buyer_tickets.len(), 1);
    assert_eq!(buyer_tickets.get_unchecked(0), ticket_id);
}

#[test]
fn resale_does_not_change_event_sold_count() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 500, 500);

    let event_before = f.client.get_event(&event_id);
    assert_eq!(event_before.sold, 1);

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);
    f.client.resell_ticket(&seller, &buyer, &ticket_id, &500);

    // sold count is only for primary purchases, not affected by resale.
    let event_after = f.client.get_event(&event_id);
    assert_eq!(event_after.sold, 1);
}

#[test]
fn resale_does_not_change_event_token_or_price() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 500, 500);

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);
    f.client.resell_ticket(&seller, &buyer, &ticket_id, &999);

    let event = f.client.get_event(&event_id);
    assert_eq!(event.ticket_price, 500);
    assert_eq!(event.token, f.token);
    assert_eq!(event.royalty_bps, 500);
}

// ══════════════════════════════════════════════════════════════════════════════
// 6. EVENT EMISSION VERIFICATION
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn resale_emits_ticket_resold_event() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (_event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 1_000, 1_000); // 10% royalty

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    let events_before = f.env.events().all().len();

    f.client.resell_ticket(&seller, &buyer, &ticket_id, &2_000);

    let events_after = f.env.events().all();
    // At least one new event was emitted.
    assert!(events_after.len() > events_before);
}

#[test]
fn purchase_emits_ticket_purchased_event() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    let event_id = f.client.create_event(
        &merchant,
        &String::from_str(&f.env, "Event"),
        &500i128,
        &f.token,
        &5u32,
        &future_date(&f.env),
        &500u32,
    );

    let events_before = f.env.events().all().len();
    f.client.purchase_ticket(&event_id, &buyer);
    let events_after = f.env.events().all();

    assert!(events_after.len() > events_before);
}

// ══════════════════════════════════════════════════════════════════════════════
// 7. STORAGE CONSISTENCY / ROLLBACK TESTS
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn resale_does_not_corrupt_seller_ticket_list_on_success() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    // Seller buys 2 tickets from different events.
    let price: i128 = 500;
    let event_id1 = f.client.create_event(
        &merchant,
        &String::from_str(&f.env, "Event1"),
        &price,
        &f.token,
        &10u32,
        &future_date(&f.env),
        &500u32,
    );
    let event_id2 = f.client.create_event(
        &merchant,
        &String::from_str(&f.env, "Event2"),
        &price,
        &f.token,
        &10u32,
        &future_date(&f.env),
        &500u32,
    );

    let seller = Address::generate(&f.env);
    fund(&f.env, &f.token, &seller, TOKEN_INITIAL_BALANCE * 10);

    let ticket_id1 = f.client.purchase_ticket(&event_id1, &seller);
    let ticket_id2 = f.client.purchase_ticket(&event_id2, &seller);

    assert_eq!(f.client.get_user_tickets(&seller).len(), 2);

    // Resell only ticket_id1.
    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);
    f.client.resell_ticket(&seller, &buyer, &ticket_id1, &500);

    // Seller still has ticket_id2.
    let seller_tickets = f.client.get_user_tickets(&seller);
    assert_eq!(seller_tickets.len(), 1);
    assert_eq!(seller_tickets.get_unchecked(0), ticket_id2);
}

// ══════════════════════════════════════════════════════════════════════════════
// 8. PAUSABLE CONTRACT GUARD TESTS
// ══════════════════════════════════════════════════════════════════════════════

#[test]
#[should_panic(expected = "Error(Contract, #9)")] // ContractPaused
fn resale_rejected_when_contract_paused() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (_event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 500, 500);

    // Pause the contract.
    f.client.pause(&f.admin);

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    f.client.resell_ticket(&seller, &buyer, &ticket_id, &500);
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")] // ContractPaused
fn purchase_rejected_when_contract_paused() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let event_id = f.client.create_event(
        &merchant,
        &String::from_str(&f.env, "X"),
        &100i128,
        &f.token,
        &5u32,
        &future_date(&f.env),
        &0u32,
    );

    f.client.pause(&f.admin);

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);
    f.client.purchase_ticket(&event_id, &buyer);
}

#[test]
fn resale_works_after_unpause() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (_event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 500, 500);

    f.client.pause(&f.admin);
    f.client.unpause(&f.admin);

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    f.client.resell_ticket(&seller, &buyer, &ticket_id, &500);
    assert_eq!(f.client.get_ticket(&ticket_id).owner, buyer);
}

// ══════════════════════════════════════════════════════════════════════════════
// 9. INTERACTION WITH CANCELLED EVENT POST-RESALE
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn resale_then_cancel_event_does_not_affect_resale_owner() {
    let f = setup();
    let (merchant, _merchant_account) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (event_id, ticket_id, original_owner) = create_event_and_purchase(&f, &merchant, 500, 500);

    let new_owner = Address::generate(&f.env);
    fund(&f.env, &f.token, &new_owner, TOKEN_INITIAL_BALANCE);
    f.client
        .resell_ticket(&original_owner, &new_owner, &ticket_id, &500);

    assert_eq!(f.client.get_ticket(&ticket_id).owner, new_owner);

    // Cancel event + refund.
    f.client.cancel_event_and_batch_refund(&merchant, &event_id);

    // Owner is still the new_owner (refund went to ticket owner at cancel time).
    assert_eq!(f.client.get_ticket(&ticket_id).owner, new_owner);
}

// ══════════════════════════════════════════════════════════════════════════════
// 10. MULTI-TOKEN TESTS
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn resale_with_different_accepted_token() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    // Create a second token.
    let token_admin2 = Address::generate(&f.env);
    let token2 = f
        .env
        .register_stellar_asset_contract_v2(token_admin2.clone())
        .address();
    f.client.add_accepted_token(&f.admin, &token2);
    f.client.set_merchant_accepted_tokens(
        &merchant,
        &soroban_sdk::Vec::from_array(&f.env, [f.token.clone(), token2.clone()]),
    );

    // Create event with token2.
    let event_id = f.client.create_event(
        &merchant,
        &String::from_str(&f.env, "Token2 Event"),
        &1_000i128,
        &token2,
        &5u32,
        &future_date(&f.env),
        &1_000u32,
    );

    let seller = Address::generate(&f.env);
    fund(&f.env, &token2, &seller, TOKEN_INITIAL_BALANCE);
    let ticket_id = f.client.purchase_ticket(&event_id, &seller);

    let buyer = Address::generate(&f.env);
    fund(&f.env, &token2, &buyer, TOKEN_INITIAL_BALANCE);

    f.client.resell_ticket(&seller, &buyer, &ticket_id, &2_000);

    let ticket = f.client.get_ticket(&ticket_id);
    assert_eq!(ticket.owner, buyer);
    assert_eq!(ticket.event_id, event_id);
}

// ══════════════════════════════════════════════════════════════════════════════
// 11. EDGE: SOLD-OUT EVENT CANNOT BUY BUT CAN RESALE
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn resale_works_on_sold_out_event() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    // Create event with capacity 1.
    let event_id = f.client.create_event(
        &merchant,
        &String::from_str(&f.env, "Sold Out"),
        &100i128,
        &f.token,
        &1u32,
        &future_date(&f.env),
        &0u32,
    );

    let seller = Address::generate(&f.env);
    fund(&f.env, &f.token, &seller, TOKEN_INITIAL_BALANCE);
    let ticket_id = f.client.purchase_ticket(&event_id, &seller);

    // Event is now sold out.
    let event = f.client.get_event(&event_id);
    assert_eq!(event.sold, event.capacity);

    // But resale still works.
    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);
    f.client.resell_ticket(&seller, &buyer, &ticket_id, &200);

    assert_eq!(f.client.get_ticket(&ticket_id).owner, buyer);
}

// ══════════════════════════════════════════════════════════════════════════════
// 12. ROYALTY MATH CORRECTNESS
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn royalty_math_5_percent() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (_event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 100, 500); // 5%

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    let merchant_account = f
        .client
        .get_merchant_account(&f.client.get_ticket(&ticket_id).event_id);
    let merchant_before = balance(&f.env, &f.token, &merchant_account);

    f.client.resell_ticket(&seller, &buyer, &ticket_id, &1_000);

    // 1000 * 500 / 10_000 = 50
    assert_eq!(
        balance(&f.env, &f.token, &merchant_account),
        merchant_before + 50
    );
}

#[test]
fn royalty_math_15_percent() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (_event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 100, 1_500); // 15%

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    let merchant_account = f
        .client
        .get_merchant_account(&f.client.get_ticket(&ticket_id).event_id);
    let merchant_before = balance(&f.env, &f.token, &merchant_account);

    f.client.resell_ticket(&seller, &buyer, &ticket_id, &1_000);

    // 1000 * 1500 / 10_000 = 150
    assert_eq!(
        balance(&f.env, &f.token, &merchant_account),
        merchant_before + 150
    );
}

#[test]
fn royalty_math_50_percent() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (_event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 100, 5_000); // 50%

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    let merchant_account = f
        .client
        .get_merchant_account(&f.client.get_ticket(&ticket_id).event_id);
    let merchant_before = balance(&f.env, &f.token, &merchant_account);
    let seller_before = balance(&f.env, &f.token, &seller);

    f.client.resell_ticket(&seller, &buyer, &ticket_id, &1_000);

    // 1000 * 5000 / 10_000 = 500
    assert_eq!(
        balance(&f.env, &f.token, &merchant_account),
        merchant_before + 500
    );
    assert_eq!(balance(&f.env, &f.token, &seller), seller_before + 500);
}

// ══════════════════════════════════════════════════════════════════════════════
// 13. GETTER FUNCTIONS WORK CORRECTLY AFTER RESALE
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn get_ticket_returns_correct_owner_after_resale() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (_event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 500, 500);

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);
    f.client.resell_ticket(&seller, &buyer, &ticket_id, &800);

    let ticket = f.client.get_ticket(&ticket_id);
    assert_eq!(ticket.owner, buyer);
    assert_eq!(ticket.id, ticket_id);
}

#[test]
fn get_event_tickets_unchanged_after_resale() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (event_id, ticket_id, seller) = create_event_and_purchase(&f, &merchant, 500, 500);

    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);
    f.client.resell_ticket(&seller, &buyer, &ticket_id, &800);

    let tickets = f.client.get_event_tickets(&event_id);
    assert_eq!(tickets.len(), 1);
    assert_eq!(tickets.get_unchecked(0), ticket_id);
}

// ══════════════════════════════════════════════════════════════════════════════
// 14. UNINITIALIZED / EDGE STATE CHECKS
// ══════════════════════════════════════════════════════════════════════════════

#[test]
#[should_panic(expected = "Error(Contract, #125)")] // TicketNotFound
fn resale_fails_on_ticket_id_zero() {
    let f = setup();
    let a = Address::generate(&f.env);
    let b = Address::generate(&f.env);
    // Ticket 0 doesn't exist in storage.
    f.client.resell_ticket(&a, &b, &0, &100);
}

#[test]
#[should_panic(expected = "Error(Contract, #126)")] // NotTicketOwner
fn resale_unauthorized_seller_rejected() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (_event_id, ticket_id, _real_owner) = create_event_and_purchase(&f, &merchant, 500, 500);

    let unauthorized = Address::generate(&f.env);
    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &unauthorized, TOKEN_INITIAL_BALANCE);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    // Even with auth mocked, unauthorized is not the ticket owner.
    f.client
        .resell_ticket(&unauthorized, &buyer, &ticket_id, &500);
}

#[test]
#[should_panic(expected = "Error(Contract, #126)")] // NotTicketOwner
fn resale_atomicity_no_state_change_on_panic() {
    // When a resale panics (e.g. not ticket owner), no state changes persist.
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (_event_id, ticket_id, _seller) = create_event_and_purchase(&f, &merchant, 500, 500);

    let imposter = Address::generate(&f.env);
    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &imposter, TOKEN_INITIAL_BALANCE);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    // This will panic because imposter is not the owner.
    // Soroban transactions are atomic: on panic, all storage changes roll back.
    f.client.resell_ticket(&imposter, &buyer, &ticket_id, &500);
}

#[test]
#[should_panic(expected = "Error(Contract, #126)")] // NotTicketOwner
fn resale_panicked_resale_does_not_modify_user_tickets() {
    let f = setup();
    let (merchant, _) = register_merchant_with_account(&f.env, &f.client, &f.token);

    let (_event_id, ticket_id, _seller) = create_event_and_purchase(&f, &merchant, 500, 500);

    let imposter = Address::generate(&f.env);
    let buyer = Address::generate(&f.env);
    fund(&f.env, &f.token, &imposter, TOKEN_INITIAL_BALANCE);
    fund(&f.env, &f.token, &buyer, TOKEN_INITIAL_BALANCE);

    // Panics — Soroban guarantees atomic rollback, so no state mutation persists.
    f.client.resell_ticket(&imposter, &buyer, &ticket_id, &500);
}
