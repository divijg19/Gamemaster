//! The command module for the player task system.

pub mod run;
pub mod ui;

use serenity::builder::CreateCommand;
pub fn register() -> CreateCommand {
    CreateCommand::new("tasks").description("View your daily and weekly tasks.")
}
