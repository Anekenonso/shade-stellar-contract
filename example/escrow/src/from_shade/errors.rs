use soroban_sdk::contracterror;

/// Error codes that moved out of the hub's `ContractError`.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ContractError {
    /// The referenced escrow record does not exist.
    EscrowNotFound = 46,
    /// The escrow is not in a state that permits this operation.
    InvalidEscrowStatus = 47,
}
