//! Implements the `/shop` command.

pub mod game;
pub mod logic;
pub mod run;
pub mod state;
pub mod ui;

// (✓) FIXED: Export both prefix and slash run functions.
pub use run::register;
