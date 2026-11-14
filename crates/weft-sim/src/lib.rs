//! Deterministic simulator for Weft IR.
//!
//! The whole system (every replica of every location, the network between them,
//! and behavioral models of every extern) runs in one thread on one seeded PRNG
//! and one event queue ordered by `(time, seq)`. Same IR + same seed = same run,
//! byte for byte. That is what makes `sweep` useful: a failing seed is a complete,
//! replayable bug report.

pub mod rng;
pub mod sim;

pub use sim::{run, Outcome, Stats, Violation};
