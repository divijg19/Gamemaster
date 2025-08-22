//! This module contains the complete, self-contained implementation for the Blackjack game.
//!
//! It declares the sub-modules for the core game logic (`game`) and the command
//! entry points (`run`), and then publicly re-exports the `run` functions for
//! use by the central command handler.

// 1. Declare the sub-modules that make up the Blackjack command.
pub mod game;
pub mod run;

// 2. Publicly re-export the `run` functions.
// This allows the handler to call `commands::blackjack::run_slash(...)`
// without needing to know about the internal file structure.
pub use run::{register, run_prefix, run_slash};
