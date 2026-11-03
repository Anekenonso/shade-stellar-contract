use shade::components::core;
use crate::from_shade::errors::{CampaignError, ContractError};
use crate::from_shade::events;
use crate::from_shade::types::{BackerComment, CampaignKey, CommentFlag, CommentStatus};
use soroban_sdk::{panic_with_error, Address, Env, String, Vec};

fn get_comment_count(env: &Env) -> u64 {
    env.storage()
        .persistent()
        .get(&CampaignKey::CommentCount)
        .unwrap_or(0)
}

fn set_comment_count(env: &Env, count: u64) {
    env.storage()
        .persistent()
        .set(&CampaignKey::CommentCount, &count);
}

pub fn create_comment(env: &Env, author: Address, crowdfund_id: u64, content: String) -> u64 {
    author.require_auth();

    if content.is_empty() {
        panic_with_error!(env, CampaignError::EmptyComment);
    }

    let comment_id = get_comment_count(env) + 1;
    set_comment_count(env, comment_id);

    let now = env.ledger().timestamp();
    let comment = BackerComment {
        id: comment_id,
        crowdfund_id,
        author: author.clone(),
        content: content.clone(),
        status: CommentStatus::Active,
        created_at: now,
        updated_at: now,
        flag_count: 0,
    };

    env.storage()
        .persistent()
        .set(&CampaignKey::Comment(comment_id), &comment);

    let mut crowdfund_comments: Vec<u64> = env
        .storage()
        .persistent()
        .get(&CampaignKey::CrowdfundComments(crowdfund_id))
        .unwrap_or_else(|| Vec::new(env));
    crowdfund_comments.push_back(comment_id);
    env.storage().persistent().set(
        &CampaignKey::CrowdfundComments(crowdfund_id),
        &crowdfund_comments,
    );

    let mut user_comments: Vec<u64> = env
        .storage()
        .persistent()
        .get(&CampaignKey::UserComments(author.clone()))
        .unwrap_or_else(|| Vec::new(env));
    user_comments.push_back(comment_id);
    env.storage()
        .persistent()
        .set(&CampaignKey::UserComments(author.clone()), &user_comments);

    events::publish_backer_comment_created_event(
        env,
        comment_id,
        crowdfund_id,
        author,
        content.len() as u64,
        now,
    );

    comment_id
}

pub fn get_comment(env: &Env, comment_id: u64) -> BackerComment {
    env.storage()
        .persistent()
        .get(&CampaignKey::Comment(comment_id))
        .unwrap_or_else(|| panic_with_error!(env, CampaignError::CommentNotFound))
}

pub fn flag_comment(env: &Env, flagger: Address, comment_id: u64, reason: String) {
    flagger.require_auth();

    if reason.is_empty() {
        panic_with_error!(env, shade::errors::ContractError::InvalidDescription);
    }

    let mut comment = get_comment(env, comment_id);
    let flag = CommentFlag {
        comment_id,
        flagger: flagger.clone(),
        reason: reason.clone(),
        flagged_at: env.ledger().timestamp(),
    };

    env.storage()
        .persistent()
        .set(&CampaignKey::CommentFlag(comment_id), &flag);

    comment.flag_count = comment.flag_count.saturating_add(1);
    if comment.status == CommentStatus::Active && comment.flag_count >= 3 {
        comment.status = CommentStatus::Flagged;
    }
    comment.updated_at = env.ledger().timestamp();

    env.storage()
        .persistent()
        .set(&CampaignKey::Comment(comment_id), &comment);

    let now = env.ledger().timestamp();
    events::publish_backer_comment_flagged_event(
        env,
        comment_id,
        flagger,
        reason.len() as u64,
        comment.flag_count,
        now,
    );
}

pub fn remove_comment(env: &Env, moderator: Address, comment_id: u64) {
    moderator.require_auth();
    let contract_admin = core::get_admin(env);
    if moderator != contract_admin {
        panic_with_error!(env, shade::errors::ContractError::NotAuthorized);
    }

    let mut comment = get_comment(env, comment_id);
    if comment.status == CommentStatus::Removed {
        panic_with_error!(env, CampaignError::InvalidCommentStatus);
    }

    comment.status = CommentStatus::Removed;
    comment.updated_at = env.ledger().timestamp();

    env.storage()
        .persistent()
        .set(&CampaignKey::Comment(comment_id), &comment);

    let now = env.ledger().timestamp();
    events::publish_backer_comment_removed_event(
        env,
        comment_id,
        comment.crowdfund_id,
        moderator.clone(),
        now,
    );

    events::publish_comment_moderation_applied_event(
        env,
        comment_id,
        moderator,
        String::from_str(env, "removed"),
        now,
    );
}

pub fn approve_flagged_comment(env: &Env, moderator: Address, comment_id: u64) {
    moderator.require_auth();
    let contract_admin = core::get_admin(env);
    if moderator != contract_admin {
        panic_with_error!(env, shade::errors::ContractError::NotAuthorized);
    }

    let mut comment = get_comment(env, comment_id);
    if comment.status != CommentStatus::Flagged {
        panic_with_error!(env, CampaignError::InvalidCommentStatus);
    }

    comment.status = CommentStatus::Active;
    comment.flag_count = 0;
    comment.updated_at = env.ledger().timestamp();

    env.storage()
        .persistent()
        .set(&CampaignKey::Comment(comment_id), &comment);

    let now = env.ledger().timestamp();
    events::publish_comment_moderation_applied_event(
        env,
        comment_id,
        moderator,
        String::from_str(env, "approved"),
        now,
    );
}

pub fn get_crowdfund_comments(env: &Env, crowdfund_id: u64) -> Vec<u64> {
    env.storage()
        .persistent()
        .get(&CampaignKey::CrowdfundComments(crowdfund_id))
        .unwrap_or_else(|| Vec::new(env))
}

pub fn get_user_comments(env: &Env, user: Address) -> Vec<u64> {
    env.storage()
        .persistent()
        .get(&CampaignKey::UserComments(user))
        .unwrap_or_else(|| Vec::new(env))
}
