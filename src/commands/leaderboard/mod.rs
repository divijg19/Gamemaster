//! Implements the `/leaderboard` command.

pub mod run;
pub mod ui;

pub use run::{register, run_prefix, run_slash};
