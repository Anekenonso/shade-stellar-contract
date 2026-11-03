#![no_std]

mod errors;
#[cfg(test)]
mod test;
#[cfg(test)]
mod tests;

use crate::errors::FactoryError;
use soroban_sdk::{
    contract, contractevent, contractimpl, contracttype, panic_with_error, Address, Bytes, BytesN,
    Env, IntoVal, String, Symbol, Vec,
};

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct CampaignRef {
    pub campaign_id: u64,
    pub contract: Address,
    pub organizer: Address,
    pub deployed_at: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum ProposalStatus {
    Pending,
    Approved,
    Rejected,
    Executed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum DaoProposalStatus {
    Voting,
    Executed,
    Rejected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct CampaignProposal {
    pub id: u64,
    pub organizer: Address,
    pub token: Address,
    pub goal: i128,
    pub deadline: u64,
    pub status: ProposalStatus,
    pub created_at: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct DaoProposal {
    pub id: u64,
    pub proposer: Address,
    pub description: String,
    pub new_wasm_hash: BytesN<32>,
    pub votes_for: u32,
    pub votes_against: u32,
    pub status: DaoProposalStatus,
    pub created_at: u64,
    pub voting_deadline: u64,
}

#[derive(Clone)]
#[contracttype]
enum DataKey {
    CrowdfundWasmHash,
    CampaignRef(u64),
    CampaignRefCount,
    // Governance admin, authorised to grant/revoke reviewers (#358).
    Admin,
    // Whether a given address may approve/reject campaign proposals (#358).
    Reviewer(Address),
    CampaignProposal(u64),
    CampaignProposalCount,
    // Address authorised to add/remove DAO members (#359).
    DaoAdmin,
    // Whether a given address is a voting DAO member.
    DaoMember(Address),
    DaoMemberCount,
    // Basis points (1 bp = 0.01%) of the member count required to reach quorum.
    DaoQuorumBps,
    DaoProposal(u64),
    DaoProposalCount,
    // Tracks whether a member already voted on a specific proposal.
    DaoVote(u64, Address),
}

#[contractevent]
pub struct CampaignDeployedEvent {
    pub campaign_id: u64,
    pub contract: Address,
    pub organizer: Address,
    pub deployed_at: u64,
}

#[contractevent]
pub struct ReviewerGrantedEvent {
    pub reviewer: Address,
    pub granted_by: Address,
}

#[contractevent]
pub struct ReviewerRevokedEvent {
    pub reviewer: Address,
    pub revoked_by: Address,
}

#[contractevent]
pub struct CampaignProposalCreatedEvent {
    pub proposal_id: u64,
    pub organizer: Address,
    pub token: Address,
    pub goal: i128,
    pub deadline: u64,
    pub created_at: u64,
}

#[contractevent]
pub struct CampaignProposalApprovedEvent {
    pub proposal_id: u64,
    pub reviewer: Address,
    pub approved_at: u64,
}

#[contractevent]
pub struct CampaignProposalRejectedEvent {
    pub proposal_id: u64,
    pub reviewer: Address,
    pub reason: String,
    pub rejected_at: u64,
}

#[contractevent]
pub struct CampaignProposalExecutedEvent {
    pub proposal_id: u64,
    pub campaign_id: u64,
    pub contract: Address,
    pub executed_at: u64,
}

#[contractevent]
pub struct DaoMemberAddedEvent {
    pub member: Address,
}

#[contractevent]
pub struct DaoMemberRemovedEvent {
    pub member: Address,
}

#[contractevent]
pub struct DaoProposalCreatedEvent {
    pub proposal_id: u64,
    pub proposer: Address,
    pub description: String,
    pub voting_deadline: u64,
}

#[contractevent]
pub struct DaoVoteCastEvent {
    pub proposal_id: u64,
    pub voter: Address,
    pub support: bool,
    pub votes_for: u32,
    pub votes_against: u32,
}

#[contractevent]
pub struct DaoProposalExecutedEvent {
    pub proposal_id: u64,
    pub new_wasm_hash: BytesN<32>,
}

#[contractevent]
pub struct DaoProposalRejectedEvent {
    pub proposal_id: u64,
}

fn get_campaign_count(env: &Env) -> u64 {
    env.storage()
        .persistent()
        .get(&DataKey::CampaignRefCount)
        .unwrap_or(0)
}

fn get_campaign_proposal_count(env: &Env) -> u64 {
    env.storage()
        .persistent()
        .get(&DataKey::CampaignProposalCount)
        .unwrap_or(0)
}

fn get_admin(env: &Env) -> Address {
    env.storage()
        .persistent()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic_with_error!(env, FactoryError::GovernanceNotInitialized))
}

fn require_reviewer(env: &Env, caller: &Address) {
    caller.require_auth();
    let admin: Address = get_admin(env);
    if *caller == admin {
        return;
    }
    let is_reviewer: bool = env
        .storage()
        .persistent()
        .get(&DataKey::Reviewer(caller.clone()))
        .unwrap_or(false);
    if !is_reviewer {
        panic_with_error!(env, FactoryError::NotReviewer);
    }
}

fn get_dao_admin(env: &Env) -> Address {
    env.storage()
        .persistent()
        .get(&DataKey::DaoAdmin)
        .unwrap_or_else(|| panic_with_error!(env, FactoryError::DaoNotInitialized))
}

fn get_dao_member_count(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::DaoMemberCount)
        .unwrap_or(0)
}

fn get_dao_proposal_count(env: &Env) -> u64 {
    env.storage()
        .persistent()
        .get(&DataKey::DaoProposalCount)
        .unwrap_or(0)
}

fn is_dao_member(env: &Env, address: &Address) -> bool {
    env.storage()
        .persistent()
        .get(&DataKey::DaoMember(address.clone()))
        .unwrap_or(false)
}

#[contract]
pub struct CrowdfundFactory;

#[contractimpl]
impl CrowdfundFactory {
    pub fn initialize(env: Env, crowdfund_wasm_hash: BytesN<32>) {
        if env.storage().persistent().has(&DataKey::CrowdfundWasmHash) {
            panic_with_error!(&env, FactoryError::AlreadyInitialized);
        }
        env.storage()
            .persistent()
            .set(&DataKey::CrowdfundWasmHash, &crowdfund_wasm_hash);
        env.storage()
            .persistent()
            .set(&DataKey::CampaignRefCount, &0_u64);
    }

    pub fn set_crowdfund_wasm_hash(env: Env, crowdfund_wasm_hash: BytesN<32>) {
        if !env.storage().persistent().has(&DataKey::CrowdfundWasmHash) {
            panic_with_error!(&env, FactoryError::NotInitialized);
        }
        env.storage()
            .persistent()
            .set(&DataKey::CrowdfundWasmHash, &crowdfund_wasm_hash);
    }

    /// Deploy and initialize an independent crowdfund campaign (#316).
    pub fn deploy_campaign(
        env: Env,
        organizer: Address,
        token: Address,
        goal: i128,
        deadline: u64,
    ) -> CampaignRef {
        organizer.require_auth();
        Self::deploy_campaign_internal(&env, organizer, token, goal, deadline)
    }

    fn deploy_campaign_internal(
        env: &Env,
        organizer: Address,
        token: Address,
        goal: i128,
        deadline: u64,
    ) -> CampaignRef {
        let wasm_hash: BytesN<32> = env
            .storage()
            .persistent()
            .get(&DataKey::CrowdfundWasmHash)
            .unwrap_or_else(|| panic_with_error!(env, FactoryError::WasmHashNotSet));

        let random: BytesN<32> = env.prng().gen();
        let salt = env
            .crypto()
            .keccak256(&Bytes::from_slice(env, &random.to_array()));

        let campaign_addr = env
            .deployer()
            .with_current_contract(salt)
            .deploy_v2(wasm_hash, ());
        env.invoke_contract::<()>(
            &campaign_addr,
            &Symbol::new(env, "init_campaign"),
            (organizer.clone(), token, goal, deadline).into_val(env),
        );

        let campaign_id = get_campaign_count(env) + 1;
        let deployed_at = env.ledger().timestamp();
        let campaign_ref = CampaignRef {
            campaign_id,
            contract: campaign_addr.clone(),
            organizer: organizer.clone(),
            deployed_at,
        };

        env.storage()
            .persistent()
            .set(&DataKey::CampaignRef(campaign_id), &campaign_ref);
        env.storage()
            .persistent()
            .set(&DataKey::CampaignRefCount, &campaign_id);

        CampaignDeployedEvent {
            campaign_id,
            contract: campaign_addr,
            organizer,
            deployed_at,
        }
        .publish(env);

        campaign_ref
    }

    // ── Campaign approval governance (#358) ──────────────────────────────────

    /// One-time setup of the governance admin. Independent of `initialize`
    /// so existing deployments can adopt governance without redeploying.
    pub fn init_governance(env: Env, admin: Address) {
        admin.require_auth();
        if env.storage().persistent().has(&DataKey::Admin) {
            panic_with_error!(&env, FactoryError::AlreadyInitialized);
        }
        env.storage().persistent().set(&DataKey::Admin, &admin);
    }

    pub fn get_governance_admin(env: Env) -> Address {
        get_admin(&env)
    }

    /// Grant an address permission to approve/reject campaign proposals.
    pub fn grant_reviewer(env: Env, admin: Address, reviewer: Address) {
        admin.require_auth();
        if admin != get_admin(&env) {
            panic_with_error!(&env, FactoryError::NotReviewer);
        }
        env.storage()
            .persistent()
            .set(&DataKey::Reviewer(reviewer.clone()), &true);
        ReviewerGrantedEvent {
            reviewer,
            granted_by: admin,
        }
        .publish(&env);
    }

    /// Revoke a previously granted reviewer's approval rights.
    pub fn revoke_reviewer(env: Env, admin: Address, reviewer: Address) {
        admin.require_auth();
        if admin != get_admin(&env) {
            panic_with_error!(&env, FactoryError::NotReviewer);
        }
        env.storage()
            .persistent()
            .set(&DataKey::Reviewer(reviewer.clone()), &false);
        ReviewerRevokedEvent {
            reviewer,
            revoked_by: admin,
        }
        .publish(&env);
    }

    pub fn is_reviewer(env: Env, address: Address) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::Reviewer(address))
            .unwrap_or(false)
    }

    /// Organizer submits a campaign for governance approval instead of
    /// deploying directly via `deploy_campaign`.
    pub fn propose_campaign(
        env: Env,
        organizer: Address,
        token: Address,
        goal: i128,
        deadline: u64,
    ) -> u64 {
        organizer.require_auth();
        if !env.storage().persistent().has(&DataKey::Admin) {
            panic_with_error!(&env, FactoryError::GovernanceNotInitialized);
        }
        if goal <= 0 {
            panic_with_error!(&env, FactoryError::InvalidGoal);
        }
        if deadline <= env.ledger().timestamp() {
            panic_with_error!(&env, FactoryError::InvalidDeadline);
        }

        let proposal_id = get_campaign_proposal_count(&env) + 1;
        let created_at = env.ledger().timestamp();
        let proposal = CampaignProposal {
            id: proposal_id,
            organizer: organizer.clone(),
            token: token.clone(),
            goal,
            deadline,
            status: ProposalStatus::Pending,
            created_at,
        };

        env.storage()
            .persistent()
            .set(&DataKey::CampaignProposal(proposal_id), &proposal);
        env.storage()
            .persistent()
            .set(&DataKey::CampaignProposalCount, &proposal_id);

        CampaignProposalCreatedEvent {
            proposal_id,
            organizer,
            token,
            goal,
            deadline,
            created_at,
        }
        .publish(&env);

        proposal_id
    }

    pub fn get_campaign_proposal(env: Env, proposal_id: u64) -> CampaignProposal {
        env.storage()
            .persistent()
            .get(&DataKey::CampaignProposal(proposal_id))
            .unwrap_or_else(|| panic_with_error!(&env, FactoryError::ProposalNotFound))
    }

    pub fn get_campaign_proposal_count(env: Env) -> u64 {
        get_campaign_proposal_count(&env)
    }

    /// Admin or a granted reviewer approves a pending proposal.
    pub fn approve_campaign_proposal(env: Env, reviewer: Address, proposal_id: u64) {
        require_reviewer(&env, &reviewer);

        let mut proposal: CampaignProposal = env
            .storage()
            .persistent()
            .get(&DataKey::CampaignProposal(proposal_id))
            .unwrap_or_else(|| panic_with_error!(&env, FactoryError::ProposalNotFound));

        if proposal.status != ProposalStatus::Pending {
            panic_with_error!(&env, FactoryError::ProposalNotPending);
        }

        proposal.status = ProposalStatus::Approved;
        env.storage()
            .persistent()
            .set(&DataKey::CampaignProposal(proposal_id), &proposal);

        let approved_at = env.ledger().timestamp();
        CampaignProposalApprovedEvent {
            proposal_id,
            reviewer,
            approved_at,
        }
        .publish(&env);
    }

    /// Admin or a granted reviewer rejects a pending proposal.
    pub fn reject_campaign_proposal(env: Env, reviewer: Address, proposal_id: u64, reason: String) {
        require_reviewer(&env, &reviewer);

        let mut proposal: CampaignProposal = env
            .storage()
            .persistent()
            .get(&DataKey::CampaignProposal(proposal_id))
            .unwrap_or_else(|| panic_with_error!(&env, FactoryError::ProposalNotFound));

        if proposal.status != ProposalStatus::Pending {
            panic_with_error!(&env, FactoryError::ProposalNotPending);
        }

        proposal.status = ProposalStatus::Rejected;
        env.storage()
            .persistent()
            .set(&DataKey::CampaignProposal(proposal_id), &proposal);

        let rejected_at = env.ledger().timestamp();
        CampaignProposalRejectedEvent {
            proposal_id,
            reviewer,
            reason,
            rejected_at,
        }
        .publish(&env);
    }

    /// Deploy the campaign contract for an approved proposal. Callable only
    /// by the proposal's organizer.
    pub fn execute_campaign_proposal(env: Env, proposal_id: u64) -> CampaignRef {
        let mut proposal: CampaignProposal = env
            .storage()
            .persistent()
            .get(&DataKey::CampaignProposal(proposal_id))
            .unwrap_or_else(|| panic_with_error!(&env, FactoryError::ProposalNotFound));

        proposal.organizer.require_auth();

        if proposal.status != ProposalStatus::Approved {
            panic_with_error!(&env, FactoryError::ProposalNotApproved);
        }

        proposal.status = ProposalStatus::Executed;
        env.storage()
            .persistent()
            .set(&DataKey::CampaignProposal(proposal_id), &proposal);

        let campaign_ref = Self::deploy_campaign_internal(
            &env,
            proposal.organizer.clone(),
            proposal.token.clone(),
            proposal.goal,
            proposal.deadline,
        );

        let executed_at = env.ledger().timestamp();
        CampaignProposalExecutedEvent {
            proposal_id,
            campaign_id: campaign_ref.campaign_id,
            contract: campaign_ref.contract.clone(),
            executed_at,
        }
        .publish(&env);

        campaign_ref
    }

    pub fn get_campaign_ref(env: Env, campaign_id: u64) -> CampaignRef {
        env.storage()
            .persistent()
            .get(&DataKey::CampaignRef(campaign_id))
            .unwrap_or_else(|| panic_with_error!(&env, FactoryError::CampaignNotFound))
    }

    pub fn get_campaign_count(env: Env) -> u64 {
        get_campaign_count(&env)
    }

    pub fn get_all_campaigns(env: Env) -> Vec<CampaignRef> {
        let count = get_campaign_count(&env);
        let mut campaigns = Vec::new(&env);
        for i in 1..=count {
            if let Some(campaign_ref) = env.storage().persistent().get(&DataKey::CampaignRef(i)) {
                campaigns.push_back(campaign_ref);
            }
        }
        campaigns
    }

    // ── DAO governance (#359) ─────────────────────────────────────────────────

    /// One-time setup of the DAO admin and quorum. The admin manages
    /// membership; quorum is expressed in basis points of the current
    /// member count (e.g. 5000 = 50%).
    pub fn init_dao(env: Env, admin: Address, quorum_bps: u32) {
        admin.require_auth();
        if env.storage().persistent().has(&DataKey::DaoAdmin) {
            panic_with_error!(&env, FactoryError::DaoAlreadyInitialized);
        }
        if quorum_bps == 0 || quorum_bps > 10_000 {
            panic_with_error!(&env, FactoryError::InvalidQuorum);
        }
        env.storage().persistent().set(&DataKey::DaoAdmin, &admin);
        env.storage()
            .persistent()
            .set(&DataKey::DaoQuorumBps, &quorum_bps);
        env.storage()
            .persistent()
            .set(&DataKey::DaoMemberCount, &0_u32);
        env.storage()
            .persistent()
            .set(&DataKey::DaoProposalCount, &0_u64);
    }

    pub fn add_dao_member(env: Env, admin: Address, member: Address) {
        admin.require_auth();
        if admin != get_dao_admin(&env) {
            panic_with_error!(&env, FactoryError::NotDaoAdmin);
        }
        if is_dao_member(&env, &member) {
            panic_with_error!(&env, FactoryError::AlreadyDaoMember);
        }
        env.storage()
            .persistent()
            .set(&DataKey::DaoMember(member.clone()), &true);
        let count = get_dao_member_count(&env) + 1;
        env.storage()
            .persistent()
            .set(&DataKey::DaoMemberCount, &count);
        DaoMemberAddedEvent { member }.publish(&env);
    }

    pub fn remove_dao_member(env: Env, admin: Address, member: Address) {
        admin.require_auth();
        if admin != get_dao_admin(&env) {
            panic_with_error!(&env, FactoryError::NotDaoAdmin);
        }
        if !is_dao_member(&env, &member) {
            panic_with_error!(&env, FactoryError::NotDaoMember);
        }
        env.storage()
            .persistent()
            .set(&DataKey::DaoMember(member.clone()), &false);
        let count = get_dao_member_count(&env).saturating_sub(1);
        env.storage()
            .persistent()
            .set(&DataKey::DaoMemberCount, &count);
        DaoMemberRemovedEvent { member }.publish(&env);
    }

    pub fn is_dao_member(env: Env, address: Address) -> bool {
        is_dao_member(&env, &address)
    }

    pub fn get_dao_member_count(env: Env) -> u32 {
        get_dao_member_count(&env)
    }

    /// A DAO member proposes updating the platform's crowdfund wasm hash.
    /// `voting_period` is in seconds and must be > 0.
    pub fn create_dao_proposal(
        env: Env,
        proposer: Address,
        description: String,
        new_wasm_hash: BytesN<32>,
        voting_period: u64,
    ) -> u64 {
        proposer.require_auth();
        if !is_dao_member(&env, &proposer) {
            panic_with_error!(&env, FactoryError::NotDaoMember);
        }
        if voting_period == 0 {
            panic_with_error!(&env, FactoryError::InvalidVotingPeriod);
        }

        let proposal_id = get_dao_proposal_count(&env) + 1;
        let created_at = env.ledger().timestamp();
        let voting_deadline = created_at + voting_period;

        let proposal = DaoProposal {
            id: proposal_id,
            proposer: proposer.clone(),
            description: description.clone(),
            new_wasm_hash,
            votes_for: 0,
            votes_against: 0,
            status: DaoProposalStatus::Voting,
            created_at,
            voting_deadline,
        };

        env.storage()
            .persistent()
            .set(&DataKey::DaoProposal(proposal_id), &proposal);
        env.storage()
            .persistent()
            .set(&DataKey::DaoProposalCount, &proposal_id);

        DaoProposalCreatedEvent {
            proposal_id,
            proposer,
            description,
            voting_deadline,
        }
        .publish(&env);

        proposal_id
    }

    pub fn get_dao_proposal(env: Env, proposal_id: u64) -> DaoProposal {
        env.storage()
            .persistent()
            .get(&DataKey::DaoProposal(proposal_id))
            .unwrap_or_else(|| panic_with_error!(&env, FactoryError::DaoProposalNotFound))
    }

    pub fn get_dao_proposal_count(env: Env) -> u64 {
        get_dao_proposal_count(&env)
    }

    /// Cast a single-weight vote on a proposal still in its voting window.
    pub fn cast_dao_vote(env: Env, voter: Address, proposal_id: u64, support: bool) {
        voter.require_auth();
        if !is_dao_member(&env, &voter) {
            panic_with_error!(&env, FactoryError::NotDaoMember);
        }

        let mut proposal: DaoProposal = env
            .storage()
            .persistent()
            .get(&DataKey::DaoProposal(proposal_id))
            .unwrap_or_else(|| panic_with_error!(&env, FactoryError::DaoProposalNotFound));

        if proposal.status != DaoProposalStatus::Voting {
            panic_with_error!(&env, FactoryError::VotingClosed);
        }
        if env.ledger().timestamp() > proposal.voting_deadline {
            panic_with_error!(&env, FactoryError::VotingClosed);
        }

        let vote_key = DataKey::DaoVote(proposal_id, voter.clone());
        if env.storage().persistent().has(&vote_key) {
            panic_with_error!(&env, FactoryError::AlreadyVoted);
        }
        env.storage().persistent().set(&vote_key, &support);

        if support {
            proposal.votes_for += 1;
        } else {
            proposal.votes_against += 1;
        }
        env.storage()
            .persistent()
            .set(&DataKey::DaoProposal(proposal_id), &proposal);

        DaoVoteCastEvent {
            proposal_id,
            voter,
            support,
            votes_for: proposal.votes_for,
            votes_against: proposal.votes_against,
        }
        .publish(&env);
    }

    /// Finalise a proposal after its voting window closes: executes (updates
    /// the platform wasm hash) if quorum was met and votes_for is a strict
    /// majority of votes cast, otherwise rejects it. Callable by anyone.
    pub fn execute_dao_proposal(env: Env, proposal_id: u64) {
        let mut proposal: DaoProposal = env
            .storage()
            .persistent()
            .get(&DataKey::DaoProposal(proposal_id))
            .unwrap_or_else(|| panic_with_error!(&env, FactoryError::DaoProposalNotFound));

        if proposal.status != DaoProposalStatus::Voting {
            panic_with_error!(&env, FactoryError::DaoProposalAlreadyExecuted);
        }
        if env.ledger().timestamp() <= proposal.voting_deadline {
            panic_with_error!(&env, FactoryError::VotingNotClosed);
        }

        let total_votes = proposal.votes_for + proposal.votes_against;
        let member_count = get_dao_member_count(&env);
        let quorum_bps: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::DaoQuorumBps)
            .unwrap_or_else(|| panic_with_error!(&env, FactoryError::DaoNotInitialized));

        let quorum_met =
            (total_votes as u64) * 10_000 >= (member_count as u64) * (quorum_bps as u64);

        if quorum_met && proposal.votes_for > proposal.votes_against {
            proposal.status = DaoProposalStatus::Executed;
            env.storage()
                .persistent()
                .set(&DataKey::DaoProposal(proposal_id), &proposal);
            env.storage()
                .persistent()
                .set(&DataKey::CrowdfundWasmHash, &proposal.new_wasm_hash);

            DaoProposalExecutedEvent {
                proposal_id,
                new_wasm_hash: proposal.new_wasm_hash,
            }
            .publish(&env);
        } else {
            proposal.status = DaoProposalStatus::Rejected;
            env.storage()
                .persistent()
                .set(&DataKey::DaoProposal(proposal_id), &proposal);

            DaoProposalRejectedEvent { proposal_id }.publish(&env);
        }
    }
}
