//! Subscription cases cut from the hub's analytics and history test files.

#[test]
fn test_analytics_with_subscription_payments() {
    let env = Env::default();
    let (_admin, client, token1, _token2, merchant, _merchant_account) = setup(&env);
    let customer = Address::generate(&env);
    fund_payer(&env, &token1, &customer, 1_000_000);
    // Subscription charges pull funds via `transfer_from`, so the customer must
    // approve the shade contract to spend their tokens.
    soroban_sdk::token::TokenClient::new(&env, &token1).approve(
        &customer,
        &client.address,
        &1_000_000,
        &1_000_000,
    );

    // Create a subscription plan
    let plan_id = client.create_subscription_plan(
        &merchant,
        &String::from_str(&env, "Monthly Plan"),
        &token1,
        &1000,    // 1000 units per month
        &2592000, // 30 days in seconds
    );

    // Customer subscribes
    client.subscribe(&customer, &plan_id);

    // Charge the subscription
    client.charge_subscription(&1u64); // subscription ID 1

    // Verify analytics include subscription charges
    let analytics = client.get_merchant_analytics(&merchant, &token1);
    assert_eq!(analytics.total_volume, 1000);
    assert_eq!(analytics.total_fees, 100); // 10% fee
    assert_eq!(analytics.transaction_count, 1);

    let token_analytics = client.get_token_analytics(&token1);
    assert_eq!(token_analytics.total_volume, 1000);
    assert_eq!(token_analytics.total_fees, 100);
    assert_eq!(token_analytics.transaction_count, 1);

    // Charge again after time passes
    env.ledger().with_mut(|li| {
        li.timestamp += 2592000; // Move forward 30 days
    });

    client.charge_subscription(&1u64);

    // Verify cumulative analytics
    let updated_analytics = client.get_merchant_analytics(&merchant, &token1);
    assert_eq!(updated_analytics.total_volume, 2000);
    assert_eq!(updated_analytics.total_fees, 200);
    assert_eq!(updated_analytics.transaction_count, 2);
}

#[test]
fn test_subscription_charge_records_history() {
    let (env, shade_client, _shade_contract_id, _admin, token) = setup_test();

    let merchant = Address::generate(&env);
    shade_client.register_merchant(&merchant);
    let merchant_account = Address::generate(&env);
    shade_client.set_merchant_account(&merchant, &merchant_account);

    let plan_desc = String::from_str(&env, "Plan History");
    let plan_id = shade_client.create_subscription_plan(&merchant, &plan_desc, &token, &100, &3600);

    let customer = Address::generate(&env);
    let token_client = token::StellarAssetClient::new(&env, &token);
    token_client.mint(&customer, &1000);

    // Manual approval for subscription
    let token_token_client = token::TokenClient::new(&env, &token);
    token_token_client.approve(&customer, &shade_client.address, &1000, &2000);

    let sub_id = shade_client.subscribe(&customer, &plan_id);
    shade_client.charge_subscription(&sub_id);

    let history = shade_client.get_user_transactions(&customer);
    assert_eq!(history.len(), 1);

    let tx = history.get(0).unwrap();
    assert_eq!(tx.transaction_type, TransactionType::SubscriptionCharge);
    assert_eq!(tx.ref_id, sub_id);
    assert_eq!(tx.amount, 100);
    assert_eq!(tx.token, token);
    assert_eq!(tx.description, plan_desc);
}
