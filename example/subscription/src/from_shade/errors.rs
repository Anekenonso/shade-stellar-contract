use soroban_sdk::contracterror;

/// Error codes that moved out of the hub's `ContractError`.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ContractError {
    InvalidInterval = 21,
    PlanNotFound = 22,
    PlanNotActive = 23,
    SubscriptionNotFound = 24,
    SubscriptionNotActive = 25,
    ChargeTooEarly = 26,
}
