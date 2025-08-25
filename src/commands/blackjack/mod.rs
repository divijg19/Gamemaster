//! This module contains the complete, self-contained implementation for the Blackjack game.

// (✓) MODIFIED: Declare all the new sub-modules.
// The code for the game is now split across these files based on responsibility.
pub mod game;
pub mod handlers;
pub mod run;
pub mod state;
pub mod ui;

// Publicly re-export the functions needed by the central command handler.
pub use run::{register, run_prefix, run_slash};
