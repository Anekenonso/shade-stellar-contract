use soroban_sdk::contracterror;

/// Error codes that moved out of the hub's `ContractError`.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ContractError {
    /// This external-chain deposit has already been credited (idempotency guard).
    BridgeDepositProcessed = 49,
}

/// DAO governance errors. Codes 100–119.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum GovernanceError {
    /// Caller is not a registered governance council member.
    NotGovMember = 100,
    /// Governance voting parameters have not been configured.
    GovNotConfigured = 101,
    /// Supplied governance config is invalid (zero period or quorum > 100%).
    InvalidGovConfig = 102,
    /// No proposal exists for the supplied id.
    ProposalNotFound = 103,
    /// The proposal is no longer open (already executed or defeated).
    ProposalNotActive = 104,
    /// The voting window for this proposal has closed.
    VotingClosed = 105,
    /// The voting window is still open; the proposal cannot be finalized yet.
    VotingStillOpen = 106,
    /// This member has already voted on the proposal.
    AlreadyVoted = 107,
}

/// Multi-sig massive-withdrawal errors. Codes 140–159.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum MultiSigError {
    /// The withdrawal amount is below the configured threshold; no multi-sig needed.
    BelowMultiSigThreshold = 140,
    /// No signers have been registered for multi-sig.
    MultiSigSignersNotSet = 141,
    /// The quorum value must be > 0 and <= the number of registered signers.
    InvalidQuorum = 142,
    /// The caller is not in the registered signer list.
    NotASigner = 143,
    /// This signer has already approved the given proposal.
    AlreadyApproved = 144,
    /// The referenced withdrawal proposal does not exist.
    ProposalNotFound = 145,
    /// The proposal is not in the Pending state and cannot be acted on.
    ProposalNotPending = 146,
    /// The proposal has not yet collected enough approvals for execution.
    QuorumNotReached = 147,
    /// Only the original proposer (merchant) may cancel their own proposal.
    NotProposer = 148,
    /// The multi-sig threshold has not been configured for this token.
    ThresholdNotSet = 149,
}
