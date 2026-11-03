use shade::components::admin;
use crate::from_shade::campaigns;
use crate::from_shade::errors::ContractError;
use crate::from_shade::events;
use crate::from_shade::types::{CampaignRoyaltyConfig, CampaignSecondarySale, DataKey};
use soroban_sdk::{panic_with_error, token, Address, Env, Option};

const MAX_BPS: u32 = 10_000;

pub fn set_campaign_royalty(
    env: &Env,
    merchant: &Address,
    campaign_id: u64,
    royalty_bps: u32,
    recipient: Option<Address>,
) {
    merchant.require_auth();

    if royalty_bps > MAX_BPS {
        panic_with_error!(env, ContractError::InvalidRoyaltyBps);
    }

    let campaign = campaigns::get_campaign(env, campaign_id);
    if campaign.merchant != *merchant {
        panic_with_error!(env, ContractError::NotCampaignMerchant);
    }

    if !admin::is_accepted_token(env, &campaign.token) {
        panic_with_error!(env, shade::errors::ContractError::TokenNotAccepted);
    }

    let royalty_recipient = recipient.unwrap_or_else(|| campaign.merchant.clone());
    let config = CampaignRoyaltyConfig {
        campaign_id,
        merchant_id: campaign.merchant_id,
        recipient: royalty_recipient.clone(),
        token: campaign.token.clone(),
        royalty_bps,
        updated_at: env.ledger().timestamp(),
    };

    env.storage()
        .persistent()
        .set(&DataKey::CampaignRoyaltyConfig(campaign_id), &config);

    events::publish_campaign_royalty_configured_event(
        env,
        campaign_id,
        campaign.merchant_id,
        merchant.clone(),
        royalty_recipient,
        campaign.token,
        royalty_bps,
        env.ledger().timestamp(),
    );
}

pub fn get_campaign_royalty_config(
    env: &Env,
    campaign_id: u64,
) -> Option<CampaignRoyaltyConfig> {
    env.storage()
        .persistent()
        .get(&DataKey::CampaignRoyaltyConfig(campaign_id))
}

pub fn execute_secondary_sale(
    env: &Env,
    seller: &Address,
    buyer: &Address,
    campaign_id: u64,
    token: &Address,
    gross_amount: i128,
) -> u64 {
    seller.require_auth();
    buyer.require_auth();

    if seller == buyer {
        panic_with_error!(env, shade::errors::ContractError::NotAuthorized);
    }
    if gross_amount <= 0 {
        panic_with_error!(env, ContractError::InvalidResalePrice);
    }

    let campaign = campaigns::get_campaign(env, campaign_id);
    if !campaign.active {
        panic_with_error!(env, ContractError::CampaignInactive);
    }
    if campaign.token != *token {
        panic_with_error!(env, shade::errors::ContractError::TokenNotAccepted);
    }
    if !admin::is_accepted_token(env, token) {
        panic_with_error!(env, shade::errors::ContractError::TokenNotAccepted);
    }

    let config = get_campaign_royalty_config(env, campaign_id).unwrap_or(CampaignRoyaltyConfig {
        campaign_id,
        merchant_id: campaign.merchant_id,
        recipient: campaign.merchant.clone(),
        token: campaign.token.clone(),
        royalty_bps: 0,
        updated_at: campaign.created_at,
    });

    if config.royalty_bps > MAX_BPS {
        panic_with_error!(env, ContractError::InvalidRoyaltyBps);
    }
    if config.merchant_id != campaign.merchant_id || config.token != *token {
        panic_with_error!(env, shade::errors::ContractError::NotAuthorized);
    }

    let royalty_amount = bps_of(gross_amount, config.royalty_bps)
        .unwrap_or_else(|| panic_with_error!(env, shade::errors::ContractError::InvalidAmount));
    if royalty_amount < 0 || royalty_amount > gross_amount {
        panic_with_error!(env, shade::errors::ContractError::InvalidAmount);
    }
    let seller_amount = gross_amount - royalty_amount;

    let token_client = token::TokenClient::new(env, token);
    if seller_amount > 0 {
        token_client.transfer(buyer, seller, &seller_amount);
    }
    if royalty_amount > 0 {
        token_client.transfer(buyer, &config.recipient, &royalty_amount);
    }

    let sale_id = get_sale_count(env) + 1;
    let sale = CampaignSecondarySale {
        sale_id,
        campaign_id,
        merchant_id: campaign.merchant_id,
        seller: seller.clone(),
        buyer: buyer.clone(),
        token: token.clone(),
        gross_amount,
        royalty_amount,
        seller_amount,
        royalty_bps: config.royalty_bps,
        royalty_recipient: config.recipient.clone(),
        timestamp: env.ledger().timestamp(),
    };

    env.storage()
        .persistent()
        .set(&DataKey::CampaignSecondarySale(sale_id), &sale);
    env.storage()
        .persistent()
        .set(&DataKey::CampaignSecondarySaleCount, &sale_id);

    let total: i128 = env
        .storage()
        .persistent()
        .get(&DataKey::CampaignRoyaltyEarnings(campaign_id))
        .unwrap_or(0);
    env.storage().persistent().set(
        &DataKey::CampaignRoyaltyEarnings(campaign_id),
        &(total.saturating_add(royalty_amount)),
    );

    let token_total: i128 = env
        .storage()
        .persistent()
        .get(&DataKey::CampaignRoyaltyEarningsByToken(campaign_id, token.clone()))
        .unwrap_or(0);
    env.storage().persistent().set(
        &DataKey::CampaignRoyaltyEarningsByToken(campaign_id, token.clone()),
        &(token_total.saturating_add(royalty_amount)),
    );

    events::publish_campaign_secondary_sale_event(
        env,
        sale_id,
        campaign_id,
        campaign.merchant_id,
        seller.clone(),
        buyer.clone(),
        token.clone(),
        gross_amount,
        royalty_amount,
        seller_amount,
        config.royalty_bps,
        config.recipient,
        env.ledger().timestamp(),
    );

    sale_id
}

pub fn get_campaign_secondary_sale(env: &Env, sale_id: u64) -> CampaignSecondarySale {
    env.storage()
        .persistent()
        .get(&DataKey::CampaignSecondarySale(sale_id))
        .unwrap_or_else(|| panic_with_error!(env, ContractError::NotFound))
}

pub fn get_campaign_secondary_sale_count(env: &Env) -> u64 {
    get_sale_count(env)
}

pub fn get_campaign_royalty_earnings(env: &Env, campaign_id: u64) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::CampaignRoyaltyEarnings(campaign_id))
        .unwrap_or(0)
}

pub fn get_campaign_royalty_earnings_for_token(
    env: &Env,
    campaign_id: u64,
    token: &Address,
) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::CampaignRoyaltyEarningsByToken(campaign_id, token.clone()))
        .unwrap_or(0)
}

fn get_sale_count(env: &Env) -> u64 {
    env.storage()
        .persistent()
        .get(&DataKey::CampaignSecondarySaleCount)
        .unwrap_or(0)
}

fn bps_of(value: i128, bps: u32) -> Option<i128> {
    let scaled = value.checked_mul(i128::from(bps))?;
    Some(scaled / i128::from(MAX_BPS))
}
