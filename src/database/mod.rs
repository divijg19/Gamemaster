//! This module acts as a central hub for all database-related logic.
//! It declares the specialized submodules so they can be accessed from
//! elsewhere in the application via their full path, e.g., `database::pets::get_player_pets`.

pub mod crafting;
pub mod economy;
pub mod leaderboard;
pub mod models;
pub mod pets;
pub mod quests;
pub mod saga;
pub mod tasks;
pub mod world;
