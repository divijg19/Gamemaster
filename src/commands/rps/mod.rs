//! This module contains the logic for the Rock, Paper, Scissors game.
//! It is the first game to be implemented using the generic Game Engine.

pub mod game;
pub mod run;
pub mod state;

// Re-export the main functions for easy access from the handler.
pub use run::{register, run, run_slash};
