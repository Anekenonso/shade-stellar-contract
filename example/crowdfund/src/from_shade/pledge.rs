use shade::components::admin;
use shade::components::merchant;
use crate::from_shade::errors::{CampaignError, ContractError};
use crate::from_shade::events;
use crate::from_shade::types::{CampaignKey, Pledge, PledgeCampaign, PledgeCampaignStatus, PledgeStatus};
use soroban_sdk::{panic_with_error, token, Address, Env, String, Vec};

fn get_campaign_count(env: &Env) -> u64 {
    env.storage()
        .persistent()
        .get(&CampaignKey::PledgeCampaignCount)
        .unwrap_or(0)
}

fn get_pledge_count(env: &Env) -> u64 {
    env.storage()
        .persistent()
        .get(&CampaignKey::PledgeCount)
        .unwrap_or(0)
}

fn track_campaign_pledge(env: &Env, campaign_id: u64, pledge_id: u64) {
    let mut pledge_ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&CampaignKey::CampaignPledges(campaign_id))
        .unwrap_or_else(|| Vec::new(env));
    pledge_ids.push_back(pledge_id);
    env.storage()
        .persistent()
        .set(&CampaignKey::CampaignPledges(campaign_id), &pledge_ids);
}

fn track_contributor_pledge(env: &Env, contributor: &Address, pledge_id: u64) {
    let mut pledge_ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&CampaignKey::ContributorPledges(contributor.clone()))
        .unwrap_or_else(|| Vec::new(env));
    pledge_ids.push_back(pledge_id);
    env.storage().persistent().set(
        &CampaignKey::ContributorPledges(contributor.clone()),
        &pledge_ids,
    );
}

pub fn create_campaign(
    env: &Env,
    merchant_address: &Address,
    title: &String,
    goal: i128,
    token: &Address,
    deadline: u64,
) -> u64 {
    merchant_address.require_auth();

    if goal <= 0 {
        panic_with_error!(env, shade::errors::ContractError::InvalidAmount);
    }
    if deadline <= env.ledger().timestamp() {
        panic_with_error!(env, shade::errors::ContractError::InvalidAmount);
    }
    if !admin::is_accepted_token(env, token) {
        panic_with_error!(env, shade::errors::ContractError::TokenNotAccepted);
    }

    let merchant_id = merchant::get_merchant_id(env, merchant_address);
    if !merchant::is_merchant_active(env, merchant_id) {
        panic_with_error!(env, shade::errors::ContractError::MerchantNotActive);
    }

    let campaign_id = get_campaign_count(env) + 1;
    env.storage()
        .persistent()
        .set(&CampaignKey::PledgeCampaignCount, &campaign_id);

    let campaign = PledgeCampaign {
        id: campaign_id,
        merchant_id,
        merchant: merchant_address.clone(),
        title: title.clone(),
        goal,
        token: token.clone(),
        deadline,
        raised: 0,
        status: PledgeCampaignStatus::Active,
        date_created: env.ledger().timestamp(),
        refunds_processed: false,
    };
    env.storage()
        .persistent()
        .set(&CampaignKey::PledgeCampaign(campaign_id), &campaign);

    events::publish_pledge_campaign_created_event(
        env,
        campaign_id,
        merchant_address.clone(),
        merchant_id,
        title.clone(),
        goal,
        token.clone(),
        deadline,
        env.ledger().timestamp(),
    );

    campaign_id
}

pub fn get_campaign(env: &Env, campaign_id: u64) -> PledgeCampaign {
    env.storage()
        .persistent()
        .get(&CampaignKey::PledgeCampaign(campaign_id))
        .unwrap_or_else(|| panic_with_error!(env, CampaignError::CampaignNotFound))
}

pub fn pledge(
    env: &Env,
    contributor: &Address,
    campaign_id: u64,
    amount: i128,
    token: &Address,
) -> u64 {
    contributor.require_auth();

    if amount <= 0 {
        panic_with_error!(env, shade::errors::ContractError::InvalidAmount);
    }

    let mut campaign = get_campaign(env, campaign_id);
    if campaign.status != PledgeCampaignStatus::Active {
        panic_with_error!(env, shade::errors::ContractError::InvalidInvoiceStatus);
    }
    if env.ledger().timestamp() > campaign.deadline {
        panic_with_error!(env, shade::errors::ContractError::InvoiceExpired);
    }
    if campaign.raised >= campaign.goal {
        panic_with_error!(env, shade::errors::ContractError::InvalidAmount);
    }

    if !admin::is_accepted_token(env, token) {
        panic_with_error!(env, shade::errors::ContractError::TokenNotAccepted);
    }

    let pledge_id = get_pledge_count(env) + 1;
    env.storage()
        .persistent()
        .set(&CampaignKey::PledgeCount, &pledge_id);

    let pledge_entry = Pledge {
        id: pledge_id,
        campaign_id,
        contributor: contributor.clone(),
        amount,
        token: token.clone(),
        status: PledgeStatus::Active,
        timestamp: env.ledger().timestamp(),
    };
    env.storage()
        .persistent()
        .set(&CampaignKey::Pledge(pledge_id), &pledge_entry);

    let token_client = token::TokenClient::new(env, token);
    let merchant_account = merchant::get_merchant_account(env, campaign.merchant_id);
    token_client.transfer(contributor, &merchant_account, &amount);

    campaign.raised = campaign.raised.saturating_add(amount);
    env.storage()
        .persistent()
        .set(&CampaignKey::PledgeCampaign(campaign_id), &campaign);

    track_campaign_pledge(env, campaign_id, pledge_id);
    track_contributor_pledge(env, contributor, pledge_id);

    events::publish_pledge_made_event(
        env,
        pledge_id,
        campaign_id,
        contributor.clone(),
        amount,
        token.clone(),
        env.ledger().timestamp(),
    );

    pledge_id
}

pub fn execute_campaign(env: &Env, merchant_address: &Address, campaign_id: u64) {
    merchant_address.require_auth();

    let mut campaign = get_campaign(env, campaign_id);

    if campaign.merchant != *merchant_address {
        panic_with_error!(env, shade::errors::ContractError::NotAuthorized);
    }
    if env.ledger().timestamp() <= campaign.deadline {
        panic_with_error!(env, shade::errors::ContractError::InvoiceExpired);
    }
    if campaign.status != PledgeCampaignStatus::Active {
        panic_with_error!(env, shade::errors::ContractError::AlreadyInitialized);
    }
    if campaign.raised < campaign.goal {
        panic_with_error!(env, shade::errors::ContractError::InvalidAmount);
    }

    campaign.status = PledgeCampaignStatus::Executed;
    env.storage()
        .persistent()
        .set(&CampaignKey::PledgeCampaign(campaign_id), &campaign);

    events::publish_campaign_executed_event(
        env,
        campaign_id,
        merchant_address.clone(),
        campaign.raised,
        env.ledger().timestamp(),
    );
}

pub fn cancel_campaign(env: &Env, merchant_address: &Address, campaign_id: u64) {
    merchant_address.require_auth();

    let mut campaign = get_campaign(env, campaign_id);

    if campaign.merchant != *merchant_address {
        panic_with_error!(env, shade::errors::ContractError::NotAuthorized);
    }
    if campaign.status != PledgeCampaignStatus::Active {
        panic_with_error!(env, shade::errors::ContractError::InvalidInvoiceStatus);
    }
    if env.ledger().timestamp() > campaign.deadline {
        panic_with_error!(env, shade::errors::ContractError::InvoiceExpired);
    }

    campaign.status = PledgeCampaignStatus::Cancelled;
    env.storage()
        .persistent()
        .set(&CampaignKey::PledgeCampaign(campaign_id), &campaign);

    events::publish_campaign_cancelled_event(
        env,
        campaign_id,
        merchant_address.clone(),
        env.ledger().timestamp(),
    );
}

pub fn claim_refund(env: &Env, contributor: &Address, campaign_id: u64) {
    contributor.require_auth();

    let campaign = get_campaign(env, campaign_id);

    if env.ledger().timestamp() <= campaign.deadline {
        panic_with_error!(env, shade::errors::ContractError::InvoiceExpired);
    }
    if campaign.raised >= campaign.goal {
        panic_with_error!(env, shade::errors::ContractError::InvalidAmount);
    }
    if campaign.status == PledgeCampaignStatus::Executed {
        panic_with_error!(env, shade::errors::ContractError::AlreadyInitialized);
    }

    let pledge_ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&CampaignKey::CampaignPledges(campaign_id))
        .unwrap_or_else(|| Vec::new(env));

    for pledge_id in pledge_ids.iter() {
        let mut pledge_entry: Pledge = match env
            .storage()
            .persistent()
            .get(&CampaignKey::Pledge(pledge_id))
        {
            Some(p) => p,
            None => continue,
        };

        if pledge_entry.contributor != *contributor {
            continue;
        }
        if pledge_entry.status != PledgeStatus::Active {
            continue;
        }
        if pledge_entry.amount <= 0 {
            continue;
        }

        pledge_entry.status = PledgeStatus::Refunded;
        env.storage()
            .persistent()
            .set(&CampaignKey::Pledge(pledge_id), &pledge_entry);

        let merchant_account = merchant::get_merchant_account(env, campaign.merchant_id);
        let token_client = token::TokenClient::new(env, &pledge_entry.token);
        token_client.transfer(&merchant_account, contributor, &pledge_entry.amount);

        events::publish_pledge_refunded_event(
            env,
            pledge_id,
            campaign_id,
            contributor.clone(),
            pledge_entry.amount,
            pledge_entry.token.clone(),
            env.ledger().timestamp(),
        );
    }
}

pub fn batch_refund(env: &Env, campaign_id: u64) {
    let mut campaign = get_campaign(env, campaign_id);

    if env.ledger().timestamp() <= campaign.deadline {
        panic_with_error!(env, shade::errors::ContractError::InvoiceExpired);
    }
    if campaign.raised >= campaign.goal {
        panic_with_error!(env, shade::errors::ContractError::InvalidAmount);
    }
    if campaign.status == PledgeCampaignStatus::Executed {
        panic_with_error!(env, shade::errors::ContractError::AlreadyInitialized);
    }
    if campaign.refunds_processed {
        panic_with_error!(env, shade::errors::ContractError::AlreadyInitialized);
    }

    campaign.refunds_processed = true;
    env.storage()
        .persistent()
        .set(&CampaignKey::PledgeCampaign(campaign_id), &campaign);

    let pledge_ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&CampaignKey::CampaignPledges(campaign_id))
        .unwrap_or_else(|| Vec::new(env));
    let count = pledge_ids.len();
    let mut total_refunded: i128 = 0;

    for pledge_id in pledge_ids.iter() {
        let mut pledge_entry: Pledge = match env
            .storage()
            .persistent()
            .get(&CampaignKey::Pledge(pledge_id))
        {
            Some(p) => p,
            None => continue,
        };

        if pledge_entry.status != PledgeStatus::Active {
            continue;
        }
        if pledge_entry.amount <= 0 {
            continue;
        }

        pledge_entry.status = PledgeStatus::Refunded;
        env.storage()
            .persistent()
            .set(&CampaignKey::Pledge(pledge_id), &pledge_entry);

        let merchant_account = merchant::get_merchant_account(env, campaign.merchant_id);
        let token_client = token::TokenClient::new(env, &pledge_entry.token);
        token_client.transfer(
            &merchant_account,
            &pledge_entry.contributor,
            &pledge_entry.amount,
        );

        total_refunded = total_refunded.saturating_add(pledge_entry.amount);
    }

    events::publish_campaign_batch_refunded_event(
        env,
        campaign_id,
        total_refunded,
        count,
        env.ledger().timestamp(),
    );
}

pub fn get_pledge(env: &Env, pledge_id: u64) -> Pledge {
    env.storage()
        .persistent()
        .get(&CampaignKey::Pledge(pledge_id))
        .unwrap_or_else(|| panic_with_error!(env, shade::errors::ContractError::InvoiceNotFound))
}

pub fn get_campaign_pledges(env: &Env, campaign_id: u64) -> Vec<Pledge> {
    let pledge_ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&CampaignKey::CampaignPledges(campaign_id))
        .unwrap_or_else(|| Vec::new(env));

    let mut pledges: Vec<Pledge> = Vec::new(env);
    for pledge_id in pledge_ids.iter() {
        if let Some(pledge_entry) = env
            .storage()
            .persistent()
            .get(&CampaignKey::Pledge(pledge_id))
        {
            pledges.push_back(pledge_entry);
        }
    }
    pledges
}

pub fn get_contributor_pledges(env: &Env, contributor: &Address) -> Vec<Pledge> {
    let pledge_ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&CampaignKey::ContributorPledges(contributor.clone()))
        .unwrap_or_else(|| Vec::new(env));

    let mut pledges: Vec<Pledge> = Vec::new(env);
    for pledge_id in pledge_ids.iter() {
        if let Some(pledge_entry) = env
            .storage()
            .persistent()
            .get(&CampaignKey::Pledge(pledge_id))
        {
            pledges.push_back(pledge_entry);
        }
    }
    pledges
}
