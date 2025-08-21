//! This module contains the logic for the Rock, Paper, Scissors game.
//! It is the first game to be implemented using the generic Game Engine.

// The module now simply declares its parts and re-exports the main `run` function.
pub mod game;
pub mod run;
pub mod state;

pub use run::run;
