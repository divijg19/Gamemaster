//! This module acts as a central router for all component interactions.
//!
//! The main `handler.rs` file will delegate to this module. This module then
//! delegates to a more specialized handler based on the component's "family"
//! (e.g., "saga", "party", "train"). This keeps the main handler clean and
//! organizes all interaction logic in one place.

pub mod craft_handler;
pub mod game_handler;
pub mod leaderboard_handler;
pub mod party_handler;
pub mod saga_handler;
pub mod train_handler;
