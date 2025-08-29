//! Implements the `/sell` command.

pub mod logic;
pub mod run;
pub mod ui;

// (✓) FIXED: Export all necessary functions.
pub use run::register;
