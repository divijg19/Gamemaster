//! This module acts as a central hub for all database-related logic.
//! It declares the specialized submodules so they can be accessed from
//! elsewhere in the application via their full path.
//!
//! NOTE: Legacy `pets` module has been deprecated; all logic consolidated into `units`.

pub mod battle;
pub mod crafting;
pub mod economy;
pub mod human;
pub mod leaderboard;
pub mod models;
pub mod quests;
pub mod saga;
pub mod settings;
pub mod tasks;
pub mod tavern;
pub mod units; // final home
pub mod world;
