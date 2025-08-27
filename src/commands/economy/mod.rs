//! This module contains all commands related to the server economy.

pub mod core;
pub mod give;
pub mod inventory;
pub mod profile;
pub mod sell;
pub mod shop;
pub mod work;

// (✓) MODIFIED: Add `give` to the public API and ensure all are exported correctly.
pub use give::run::{run_prefix as give_prefix, run_slash as give_slash};
pub use inventory::run::{run_prefix as inventory_prefix, run_slash as inventory_slash};
pub use profile::run::{run_prefix as profile_prefix, run_slash as profile_slash};
pub use sell::run::{run_prefix as sell_prefix, run_slash as sell_slash};
pub use shop::run::{run_prefix as shop_prefix, run_slash as shop_slash};
pub use work::run::{run_prefix as work_prefix, run_slash as work_slash};
