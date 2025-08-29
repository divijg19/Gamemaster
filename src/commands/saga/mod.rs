//! Implements the `/saga` command, the main hub for the game.

pub mod run;
pub mod tavern;
pub mod ui;

// (✓) FIXED: This module should only declare its own submodules.
// The `leaderboard` is a separate top-level command, not part of the saga command.
// The `core` logic is in a top-level `saga` module, not here.
pub use run::{register, run_prefix, run_slash};
