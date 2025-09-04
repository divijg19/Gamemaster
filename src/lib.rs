// Library entry so integration tests and external tools can reference internal modules.
// Re-export the same modules used by the binary (`main.rs`).
pub mod commands;
pub mod constants;
pub mod database;
pub mod handler;
pub mod interactions;
pub mod model;
pub mod saga;

// Convenient re-exports for frequently used types (optional expansion later).
pub use model::AppState;
