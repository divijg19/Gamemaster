//! This module defines shared database types.
//! In a Shuttle-based deployment, the creation of the connection pool itself
//! is handled by the `#[shuttle_shared_db::Postgres]` macro in `main.rs`.

use sqlx::{Pool, Postgres};

/// A type alias for the database connection pool (`Pool<Postgres>`).
/// This is used throughout the application to provide a consistent, clear name
/// for the shared database connection state.
pub type DbPool = Pool<Postgres>;
