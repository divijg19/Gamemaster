//! The command module for the player's quest log.

pub mod run;
pub mod ui;

use serenity::builder::CreateCommand;

pub fn register() -> CreateCommand {
    CreateCommand::new("questlog").description("View your active and completed quests.")
}
