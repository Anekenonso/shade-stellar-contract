#![cfg(test)]

use shade::shade::{Shade, ShadeClient};
use crate::from_shade::types::{CampaignStatus};
use account::account::{MerchantAccount, MerchantAccountClient};
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{token, Address, Env};

struct CampaignContext<'a> {
    env: Env,
    client: ShadeClient<'a>,
    merchant: Address,
    token: Address,
    backer: Address,
}

fn setup() -> CampaignContext<'static> {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let shade_id = env.register(Shade, ());
    let client = ShadeClient::new(&env, &shade_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token = env
        .register_stellar_asset_contract_v2(Address::generate(&env))
        .address();
    client.add_accepted_token(&admin, &token);
    client.set_fee(&admin, &token, &0);

    let merchant = Address::generate(&env);
    client.register_merchant(&merchant);

    let merchant_account_id = env.register(MerchantAccount, ());
    let merchant_account = MerchantAccountClient::new(&env, &merchant_account_id);
    merchant_account.initialize(&merchant, &shade_id, &1_u64);
    client.set_merchant_account(&merchant, &merchant_account_id);

    let backer = Address::generate(&env);

    CampaignContext {
        env,
        client,
        merchant,
        token,
        backer,
    }
}

#[test]
fn failed_campaign_refund_zeroes_pledge_and_returns_funds() {
    let ctx = setup();
    let campaign_id = ctx
        .client
        .create_campaign(&ctx.merchant, &5_000, &ctx.token, &2_000);

    token::StellarAssetClient::new(&ctx.env, &ctx.token).mint(&ctx.backer, &1_500);
    ctx.client
        .pledge_campaign(&ctx.backer, &campaign_id, &1_500);

    ctx.env.ledger().set_timestamp(2_001);
    let refunded = ctx.client.claim_campaign_refund(&ctx.backer, &campaign_id);

    assert_eq!(refunded, 1_500);
    assert_eq!(
        token::TokenClient::new(&ctx.env, &ctx.token).balance(&ctx.backer),
        1_500
    );
    assert_eq!(ctx.client.get_campaign_pledge(&campaign_id, &ctx.backer), 0);

    let campaign = ctx.client.get_campaign(&campaign_id);
    assert_eq!(campaign.status, CampaignStatus::Failed);
    assert_eq!(campaign.total_refunded, 1_500);
    assert_eq!(campaign.refund_count, 1);
}

#[test]
fn batch_refund_processes_failed_campaign_in_limited_chunks() {
    let ctx = setup();
    let backer_two = Address::generate(&ctx.env);
    let campaign_id = ctx
        .client
        .create_campaign(&ctx.merchant, &10_000, &ctx.token, &2_000);

    let token_admin = token::StellarAssetClient::new(&ctx.env, &ctx.token);
    token_admin.mint(&ctx.backer, &3_000);
    token_admin.mint(&backer_two, &2_000);
    ctx.client
        .pledge_campaign(&ctx.backer, &campaign_id, &3_000);
    ctx.client
        .pledge_campaign(&backer_two, &campaign_id, &2_000);

    ctx.env.ledger().set_timestamp(2_001);
    let first = ctx.client.process_failed_campaign_refunds(&campaign_id, &1);
    let second = ctx.client.process_failed_campaign_refunds(&campaign_id, &1);

    assert_eq!(first, (3_000, 1));
    assert_eq!(second, (2_000, 1));
    assert_eq!(
        token::TokenClient::new(&ctx.env, &ctx.token).balance(&ctx.backer),
        3_000
    );
    assert_eq!(
        token::TokenClient::new(&ctx.env, &ctx.token).balance(&backer_two),
        2_000
    );

    let campaign = ctx.client.get_campaign(&campaign_id);
    assert_eq!(campaign.status, CampaignStatus::Refunded);
    assert_eq!(campaign.total_refunded, 5_000);
    assert_eq!(campaign.refund_count, 2);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #16)")]
fn successful_campaign_cannot_be_refunded() {
    let ctx = setup();
    let campaign_id = ctx
        .client
        .create_campaign(&ctx.merchant, &1_000, &ctx.token, &2_000);

    token::StellarAssetClient::new(&ctx.env, &ctx.token).mint(&ctx.backer, &1_000);
    ctx.client
        .pledge_campaign(&ctx.backer, &campaign_id, &1_000);

    ctx.env.ledger().set_timestamp(2_001);
    ctx.client.finalize_campaign(&ctx.merchant, &campaign_id);
    ctx.client.claim_campaign_refund(&ctx.backer, &campaign_id);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn double_campaign_refund_fails_after_pledge_is_zeroed() {
    let ctx = setup();
    let campaign_id = ctx
        .client
        .create_campaign(&ctx.merchant, &5_000, &ctx.token, &2_000);

    token::StellarAssetClient::new(&ctx.env, &ctx.token).mint(&ctx.backer, &1_000);
    ctx.client
        .pledge_campaign(&ctx.backer, &campaign_id, &1_000);

    ctx.env.ledger().set_timestamp(2_001);
    ctx.client.claim_campaign_refund(&ctx.backer, &campaign_id);
    ctx.client.claim_campaign_refund(&ctx.backer, &campaign_id);
}
