//! Handles the command logic for `/inventory` and `$inventory`.

use super::ui::create_inventory_embed;
use crate::{AppState, database};
use serenity::builder::{
    CreateCommand, CreateCommandOption, CreateInteractionResponseFollowup, CreateMessage,
};
use serenity::model::application::{CommandInteraction, CommandOptionType};
use serenity::model::channel::Message;
use serenity::prelude::*;

pub fn register() -> CreateCommand {
    CreateCommand::new("inventory")
        .description("View your or another user's inventory of collected resources.")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::User,
                "user",
                "The user whose inventory you want to see.",
            )
            .required(false),
        )
}

pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    interaction.defer_ephemeral(&ctx.http).await.ok();
    let pool = { ctx.data.read().await.get::<AppState>().unwrap().db.clone() };

    let user_to_fetch = if let Some(option) = interaction.data.options.first() {
        if let Some(user_id) = option.value.as_user_id() {
            user_id.to_user(&ctx.http).await.ok()
        } else {
            None
        }
    } else {
        None
    }
    .unwrap_or_else(|| interaction.user.clone());

    let inventory = database::profile::get_inventory(&pool, user_to_fetch.id).await;

    let embed = create_inventory_embed(&user_to_fetch, inventory);
    let builder = CreateInteractionResponseFollowup::new().embed(embed);
    interaction.create_followup(&ctx.http, builder).await.ok();
}

/// (✓) ADDED: Entry point for the `$inventory` prefix command.
pub async fn run_prefix(ctx: &Context, msg: &Message, _args: Vec<&str>) {
    let pool = { ctx.data.read().await.get::<AppState>().unwrap().db.clone() };

    // Default to the message author if no one is mentioned.
    let user_to_fetch = msg.mentions.first().unwrap_or(&msg.author).clone();

    let inventory = database::profile::get_inventory(&pool, user_to_fetch.id).await;

    let embed = create_inventory_embed(&user_to_fetch, inventory);
    let builder = CreateMessage::new().embed(embed).reference_message(msg);
    msg.channel_id.send_message(&ctx.http, builder).await.ok();
}
