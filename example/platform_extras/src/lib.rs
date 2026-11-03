#![no_std]
//! Hub modules with no standalone contract of their own: DAO upgrade
//! governance, bridge listeners and multi-sig withdrawal proposals.

#[cfg(feature = "from-shade")]
pub mod from_shade;
