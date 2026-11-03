use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum FactoryError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    WasmHashNotSet = 3,
    CampaignNotFound = 4,
    // Governance has not been initialised via `init_governance` (#358).
    GovernanceNotInitialized = 5,
    // Caller is neither the governance admin nor a granted reviewer.
    NotReviewer = 6,
    InvalidGoal = 7,
    InvalidDeadline = 8,
    ProposalNotFound = 9,
    // Proposal is not in `Pending` status.
    ProposalNotPending = 10,
    // Proposal is not in `Approved` status.
    ProposalNotApproved = 11,
    // DAO governance has not been initialised via `init_dao` (#359).
    DaoNotInitialized = 12,
    DaoAlreadyInitialized = 13,
    NotDaoAdmin = 14,
    AlreadyDaoMember = 15,
    NotDaoMember = 16,
    // Quorum must be > 0 and <= 10_000 basis points.
    InvalidQuorum = 17,
    InvalidVotingPeriod = 18,
    DaoProposalNotFound = 19,
    // Proposal voting window has closed, or proposal is no longer in `Voting` status.
    VotingClosed = 20,
    // Proposal voting window has not yet closed.
    VotingNotClosed = 21,
    AlreadyVoted = 22,
    DaoProposalAlreadyExecuted = 23,
}
