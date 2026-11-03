use shade::components::admin;
use crate::from_shade::analytics;
use crate::from_shade::errors::{CampaignError, ContractError};
use crate::from_shade::events;
use shade::types::{Merchant};
use crate::from_shade::types::{BackerCampaign, BackerKey, BackerPerk, BackerRewardTier, DataKey};
use soroban_sdk::{panic_with_error, token, Address, Env, String, Vec};

fn get_campaign_count(env: &Env) -> u64 {
    env.storage()
        .persistent()
        .get(&BackerKey::CampaignCount)
        .unwrap_or(0)
}

fn get_merchant_id(env: &Env, merchant_addr: &Address) -> u64 {
    env.storage()
        .persistent()
        .get(&shade::types::DataKey::MerchantId(merchant_addr.clone()))
        .unwrap_or_else(|| panic_with_error!(env, shade::errors::ContractError::MerchantNotFound))
}

fn get_campaign(env: &Env, campaign_id: u64) -> BackerCampaign {
    env.storage()
        .persistent()
        .get(&BackerKey::Campaign(campaign_id))
        .unwrap_or_else(|| panic_with_error!(env, CampaignError::CampaignNotFound))
}

fn assert_campaign_merchant(env: &Env, campaign: &BackerCampaign, merchant: &Address) {
    let merchant_id = get_merchant_id(env, merchant);
    if campaign.merchant_id != merchant_id {
        panic_with_error!(env, shade::errors::ContractError::NotAuthorized);
    }
}

fn assert_campaign_open(env: &Env, campaign: &BackerCampaign) {
    if !campaign.active {
        panic_with_error!(env, CampaignError::CampaignNotActive);
    }
    if env.ledger().timestamp() > campaign.deadline {
        panic_with_error!(env, CampaignError::CampaignEnded);
    }
}

fn get_reward_tiers(env: &Env, campaign_id: u64) -> Vec<BackerRewardTier> {
    env.storage()
        .persistent()
        .get(&BackerKey::RewardTiers(campaign_id))
        .unwrap_or_else(|| Vec::new(env))
}

fn validate_tier_ordering(env: &Env, tiers: &Vec<BackerRewardTier>) {
    let mut prev = 0_i128;
    for tier in tiers.iter() {
        if tier.min_pledge <= prev {
            panic_with_error!(env, CampaignError::InvalidTierOrdering);
        }
        prev = tier.min_pledge;
    }
}

fn tier_backer_count(env: &Env, campaign_id: u64, tier_index: u32) -> u32 {
    env.storage()
        .persistent()
        .get(&BackerKey::TierBackerCount(campaign_id, tier_index))
        .unwrap_or(0)
}

fn increment_tier_backer_count(env: &Env, campaign_id: u64, tier_index: u32) {
    let count = tier_backer_count(env, campaign_id, tier_index).saturating_add(1);
    env.storage()
        .persistent()
        .set(&BackerKey::TierBackerCount(campaign_id, tier_index), &count);
}

pub fn create_backer_campaign(
    env: &Env,
    merchant_addr: Address,
    name: String,
    token: Address,
    deadline: u64,
) -> u64 {
    merchant_addr.require_auth();

    if deadline <= env.ledger().timestamp() {
        panic_with_error!(env, CampaignError::InvalidCampaignDeadline);
    }
    if !admin::is_accepted_token(env, &token) {
        panic_with_error!(env, shade::errors::ContractError::TokenNotAccepted);
    }

    let merchant_id = get_merchant_id(env, &merchant_addr);
    let merchant_record: Merchant = env
        .storage()
        .persistent()
        .get(&shade::types::DataKey::Merchant(merchant_id))
        .unwrap_or_else(|| panic_with_error!(env, shade::errors::ContractError::MerchantNotFound));

    if !merchant_record.active {
        panic_with_error!(env, shade::errors::ContractError::MerchantNotActive);
    }

    let campaign_id = get_campaign_count(env) + 1;
    env.storage()
        .persistent()
        .set(&BackerKey::CampaignCount, &campaign_id);

    let campaign = BackerCampaign {
        id: campaign_id,
        merchant_id,
        name: name.clone(),
        token: token.clone(),
        deadline,
        raised: 0,
        active: true,
    };
    env.storage()
        .persistent()
        .set(&BackerKey::Campaign(campaign_id), &campaign);

    events::publish_backer_campaign_created_event(
        env,
        campaign_id,
        merchant_addr,
        merchant_id,
        name,
        token,
        deadline,
        env.ledger().timestamp(),
    );

    campaign_id
}

pub fn get_backer_campaign(env: &Env, campaign_id: u64) -> BackerCampaign {
    get_campaign(env, campaign_id)
}

/// Asserts `merchant` owns `campaign_id` and returns the campaign.
///
/// Shared with the vesting component, which pays a campaign's raised funds out
/// to the creator that owns it.
pub fn assert_backer_campaign_owner(
    env: &Env,
    campaign_id: u64,
    merchant: &Address,
) -> BackerCampaign {
    let campaign = get_campaign(env, campaign_id);
    assert_campaign_merchant(env, &campaign, merchant);
    campaign
}

/// Whether `caller` is the merchant that owns `campaign`.
///
/// Unlike [`assert_backer_campaign_owner`] this answers rather than panics, and
/// an address that is not a registered merchant at all is simply not the owner.
/// Callers that want their own domain-specific error for a failed ownership
/// check use this; the analytics component does.
pub fn is_backer_campaign_owner(env: &Env, campaign: &BackerCampaign, caller: &Address) -> bool {
    env.storage()
        .persistent()
        .get(&shade::types::DataKey::MerchantId(caller.clone()))
        .is_some_and(|merchant_id: u64| campaign.merchant_id == merchant_id)
}

pub fn set_backer_reward_tiers(
    env: &Env,
    merchant_addr: Address,
    campaign_id: u64,
    tiers: Vec<BackerRewardTier>,
) {
    merchant_addr.require_auth();
    let campaign = get_campaign(env, campaign_id);
    assert_campaign_merchant(env, &campaign, &merchant_addr);
    validate_tier_ordering(env, &tiers);

    env.storage()
        .persistent()
        .set(&BackerKey::RewardTiers(campaign_id), &tiers);

    events::publish_backer_reward_tiers_set_event(
        env,
        campaign_id,
        merchant_addr,
        tiers.len(),
        env.ledger().timestamp(),
    );
}

pub fn get_backer_reward_tiers(env: &Env, campaign_id: u64) -> Vec<BackerRewardTier> {
    get_reward_tiers(env, campaign_id)
}

pub fn pledge_to_campaign(env: &Env, backer: Address, campaign_id: u64, amount: i128) {
    backer.require_auth();

    if amount <= 0 {
        panic_with_error!(env, shade::errors::ContractError::InvalidAmount);
    }

    let mut campaign = get_campaign(env, campaign_id);
    assert_campaign_open(env, &campaign);

    let token_client = token::TokenClient::new(env, &campaign.token);
    token_client.transfer(&backer, env.current_contract_address(), &amount);

    let prev: i128 = env
        .storage()
        .persistent()
        .get(&BackerKey::Pledge(campaign_id, backer.clone()))
        .unwrap_or(0);
    let new_pledge = prev.saturating_add(amount);
    env.storage()
        .persistent()
        .set(&BackerKey::Pledge(campaign_id, backer.clone()), &new_pledge);

    campaign.raised = campaign.raised.saturating_add(amount);
    env.storage()
        .persistent()
        .set(&BackerKey::Campaign(campaign_id), &campaign);

    events::publish_backer_pledge_recorded_event(
        env,
        campaign_id,
        backer.clone(),
        amount,
        new_pledge,
        env.ledger().timestamp(),
    );

    // Fold the contribution into the campaign's analytics aggregate. `prev == 0`
    // is what makes this backer new; the read above already established it, so
    // analytics does not pay for a second one.
    analytics::record_pledge(env, campaign_id, &backer, amount, prev == 0);
}

pub fn get_backer_pledge(env: &Env, campaign_id: u64, backer: Address) -> i128 {
    env.storage()
        .persistent()
        .get(&BackerKey::Pledge(campaign_id, backer))
        .unwrap_or(0)
}

pub fn select_backer_reward_tier(env: &Env, backer: Address, campaign_id: u64, tier_index: u32) {
    backer.require_auth();

    let tiers = get_reward_tiers(env, campaign_id);
    if tiers.is_empty() {
        panic_with_error!(env, CampaignError::InvalidRewardTier);
    }

    let tier = tiers
        .get(tier_index)
        .unwrap_or_else(|| panic_with_error!(env, CampaignError::InvalidRewardTier));

    let pledge = get_backer_pledge(env, campaign_id, backer.clone());
    if pledge < tier.min_pledge {
        panic_with_error!(env, CampaignError::PledgeBelowTierMinimum);
    }

    if tier.max_backers > 0 {
        let count = tier_backer_count(env, campaign_id, tier_index);
        let already_selected = env
            .storage()
            .persistent()
            .get(&BackerKey::SelectedTier(campaign_id, backer.clone()))
            .unwrap_or(None::<u32>);
        let is_new_selection = already_selected != Some(tier_index);
        if is_new_selection && count >= tier.max_backers {
            panic_with_error!(env, CampaignError::RewardTierAtCapacity);
        }
    }

    let previous = env
        .storage()
        .persistent()
        .get(&BackerKey::SelectedTier(campaign_id, backer.clone()))
        .unwrap_or(None::<u32>);

    if previous != Some(tier_index) {
        if let Some(prev_index) = previous {
            let prev_count = tier_backer_count(env, campaign_id, prev_index);
            if prev_count > 0 {
                env.storage().persistent().set(
                    &BackerKey::TierBackerCount(campaign_id, prev_index),
                    &prev_count.saturating_sub(1),
                );
            }
        }
        increment_tier_backer_count(env, campaign_id, tier_index);
    }

    env.storage().persistent().set(
        &BackerKey::SelectedTier(campaign_id, backer.clone()),
        &tier_index,
    );

    events::publish_backer_reward_tier_selected_event(
        env,
        campaign_id,
        backer,
        tier_index,
        tier.min_pledge,
        tier.perks.len(),
        env.ledger().timestamp(),
    );
}

pub fn get_backer_selected_tier(env: &Env, campaign_id: u64, backer: Address) -> Option<u32> {
    env.storage()
        .persistent()
        .get(&BackerKey::SelectedTier(campaign_id, backer))
}

pub fn fulfill_backer_reward(env: &Env, merchant_addr: Address, campaign_id: u64, backer: Address) {
    merchant_addr.require_auth();
    let campaign = get_campaign(env, campaign_id);
    assert_campaign_merchant(env, &campaign, &merchant_addr);

    let pledge = get_backer_pledge(env, campaign_id, backer.clone());
    if pledge <= 0 {
        panic_with_error!(env, CampaignError::NotBacker);
    }

    if env
        .storage()
        .persistent()
        .get(&BackerKey::RewardFulfilled(campaign_id, backer.clone()))
        .unwrap_or(false)
    {
        panic_with_error!(env, CampaignError::BackerRewardAlreadyFulfilled);
    }

    env.storage().persistent().set(
        &BackerKey::RewardFulfilled(campaign_id, backer.clone()),
        &true,
    );

    let tier_index = get_backer_selected_tier(env, campaign_id, backer.clone());

    events::publish_backer_reward_fulfilled_event(
        env,
        campaign_id,
        merchant_addr,
        backer,
        tier_index.unwrap_or(0),
        pledge,
        env.ledger().timestamp(),
    );
}

pub fn is_backer_reward_fulfilled(env: &Env, campaign_id: u64, backer: Address) -> bool {
    env.storage()
        .persistent()
        .get(&BackerKey::RewardFulfilled(campaign_id, backer))
        .unwrap_or(false)
}

pub fn claim_backer_perk(env: &Env, backer: Address, campaign_id: u64, perk_index: u32) {
    backer.require_auth();

    if !is_backer_reward_fulfilled(env, campaign_id, backer.clone()) {
        panic_with_error!(env, CampaignError::BackerRewardNotFulfilled);
    }

    let tier_index = get_backer_selected_tier(env, campaign_id, backer.clone())
        .unwrap_or_else(|| panic_with_error!(env, CampaignError::InvalidRewardTier));

    let tiers = get_reward_tiers(env, campaign_id);
    let tier = tiers
        .get(tier_index)
        .unwrap_or_else(|| panic_with_error!(env, CampaignError::InvalidRewardTier));

    let perk: BackerPerk = tier
        .perks
        .get(perk_index)
        .unwrap_or_else(|| panic_with_error!(env, CampaignError::PerkNotFound));

    if env
        .storage()
        .persistent()
        .get(&BackerKey::PerkClaimed(
            campaign_id,
            backer.clone(),
            perk_index,
        ))
        .unwrap_or(false)
    {
        panic_with_error!(env, CampaignError::PerkAlreadyClaimed);
    }

    env.storage().persistent().set(
        &BackerKey::PerkClaimed(campaign_id, backer.clone(), perk_index),
        &true,
    );

    events::publish_backer_perk_claimed_event(
        env,
        campaign_id,
        backer,
        tier_index,
        perk_index,
        perk.name,
        env.ledger().timestamp(),
    );
}

pub fn is_backer_perk_claimed(
    env: &Env,
    campaign_id: u64,
    backer: Address,
    perk_index: u32,
) -> bool {
    env.storage()
        .persistent()
        .get(&BackerKey::PerkClaimed(campaign_id, backer, perk_index))
        .unwrap_or(false)
}
