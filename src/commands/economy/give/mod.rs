//! Implements the `/give` command for trading items.

pub mod logic;
pub mod run;

// (✓) MODIFIED: The `pub use` line was redundant because `economy/mod.rs` handles the exports.
// We only need to declare the `run` module as public.
pub use run::register;
