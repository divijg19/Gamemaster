//! The command module for the player's quest log.

pub mod run;
pub mod ui;

use serenity::builder::{CreateCommand, CreateCommandOption};
use serenity::model::application::CommandOptionType;

pub fn register() -> CreateCommand {
    CreateCommand::new("questlog")
        .description("View your active and completed quests.")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "quest_id",
                "Quest id for raw detail",
            )
            .required(false),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Boolean,
                "verbose",
                "Verbose quest board data (debug)",
            )
            .required(false),
        )
}
