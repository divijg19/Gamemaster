//! Implements the `/party` command for managing a player's army and active party.

pub mod run;
pub mod ui;

pub use run::{register, run_prefix, run_slash};
