//! Test-suite every game must pass (feature `conformance`, enabled in game dev-dependencies).
//! A game implements [`Conformance`] and calls [`run_all`] from a test.

mod contract;
mod setup;
mod suite;

pub use contract::Conformance;
pub use setup::Setup;
pub use suite::{rejects_invalid_setups, run_all, scripted_hash, simulate};
