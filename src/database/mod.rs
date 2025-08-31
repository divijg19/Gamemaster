//! This module acts as a central hub for all database-related logic.
//! It declares the specialized submodules and publicly exports their functions
//! so they can be conveniently called from elsewhere in the application
//! (e.g., `database::get_player_pets(...)` instead of `database::pets::get_player_pets(...)`).

// Declare all the new, specialized modules.
pub mod economy;
pub mod leaderboard;
pub mod models;
pub mod pets;
pub mod saga;
pub mod world;

// Publicly export all the functions from the new modules.
pub use economy::*;
pub use pets::*;
pub use saga::*;
pub use world::*;
