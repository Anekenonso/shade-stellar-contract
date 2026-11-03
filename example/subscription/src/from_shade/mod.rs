//! Code moved out of the Shade hub, wired to the `shade` crate. Build with `--features from-shade`.

pub mod errors;
pub mod events;
pub mod types;
pub mod entrypoints;
pub mod search;
pub mod subscription;

#[cfg(test)]
mod tests;
