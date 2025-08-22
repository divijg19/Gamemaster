//! This module contains the shared game engine and common utilities for all games.
//!
//! It declares the sub-modules for the core engine, cards, and decks, and then
//! publicly re-exports the essential traits and structs that all game implementations
//! will need to use.

// 1. Declare the sub-modules that make up the engine and its utilities.
pub mod card;
pub mod deck;
pub mod engine;

// 2. Publicly re-export the most important components from the engine.
//    This allows other parts of the code to write `use crate::commands::games::Game;`
//    instead of the more verbose `use crate::commands::games::engine::Game;`.
//    The `unused_imports` warning from clippy on this line is expected and can be ignored,
//    as the purpose of this file is to export these items for external use.
#[allow(unused_imports)]
pub use engine::{Game, GameManager, GamePayout, GameUpdate};
