//! The command module for the player quest board system.

pub mod run;
pub mod ui;

use serenity::builder::CreateCommand;
pub fn register() -> CreateCommand {
    CreateCommand::new("quests").description("View the quest board to accept new quests.")
}
