//! Implements the `/open` command.
//! This command will be used for items that can be opened, like geodes.

pub mod run;

pub use run::{register, run_prefix, run_slash};
