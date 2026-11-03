use soroban_sdk::contracterror;

/// Ticketing / event errors. Codes 120–139.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum EventError {
    EventNotFound = 120,
    EventSoldOut = 121,
    InvalidCapacity = 122,
    InvalidEventDate = 123,
    InvalidRoyaltyBps = 124,
    TicketNotFound = 125,
    NotTicketOwner = 126,
    InvalidResalePrice = 127,
    TicketNotListed = 128,
    TicketAlreadyListed = 129,
}
