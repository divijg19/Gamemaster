//! Handles the command logic for `/open` and `$open`.

use serenity::builder::{
    CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage,
};
use serenity::model::application::{CommandInteraction, CommandOptionType};
use serenity::model::channel::Message;
use serenity::prelude::*;

pub fn register() -> CreateCommand {
    CreateCommand::new("open")
        .description("Open an item from your inventory to see what's inside.")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "item",
                "The item you want to open.",
            )
            .required(true)
            .add_string_choice("Large Geode", "large_geode"),
        )
}

pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    let response_msg = CreateInteractionResponseMessage::new()
        .content("This feature is coming soon! For now, your geode remains a mystery...")
        .ephemeral(true);
    let response = CreateInteractionResponse::Message(response_msg);
    interaction.create_response(&ctx.http, response).await.ok();
}

pub async fn run_prefix(ctx: &Context, msg: &Message, _args: Vec<&str>) {
    let builder = CreateMessage::new()
        .content("This feature is coming soon! For now, your geode remains a mystery...")
        .reference_message(msg);
    msg.channel_id.send_message(&ctx.http, builder).await.ok();
}
