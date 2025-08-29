//! Implements the `/train` command for pet progression.

pub mod run;
pub mod ui;

pub use run::{register, run_prefix, run_slash};
