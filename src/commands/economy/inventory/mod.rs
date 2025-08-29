//! Implements the `/inventory` command.

pub mod run;
pub mod ui;

// (✓) FIXED: Export both run functions.
pub use run::register;
