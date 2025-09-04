//! Handles the command logic for `/give` and `$give`.

use super::logic::give_item;
use crate::AppState;
use crate::commands::economy::core::item::Item;
use serenity::builder::{
    CreateCommand, CreateCommandOption, CreateInteractionResponseFollowup, CreateMessage,
};
use serenity::model::application::{CommandInteraction, CommandOptionType};
use serenity::model::channel::Message;
use serenity::prelude::*;
use std::str::FromStr;

pub fn register() -> CreateCommand {
    CreateCommand::new("give")
        .description("Give an item from your inventory to another user.")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::User,
                "user",
                "The user to give the item to",
            )
            .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "item",
                "The item you want to give",
            )
            .required(true)
            .add_string_choice("Fish", "fish")
            .add_string_choice("Ore", "ore")
            .add_string_choice("Gem", "gem")
            .add_string_choice("Golden Fish", "goldenfish")
            .add_string_choice("Large Geode", "largegeode")
            .add_string_choice("Ancient Relic", "ancientrelic")
            .add_string_choice("XP Booster", "xpbooster"),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "quantity",
                "The amount to give. Defaults to 1.",
            )
            .required(false)
            .min_int_value(1),
        )
}

pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    interaction.defer_ephemeral(&ctx.http).await.ok();
    let Some(app_state) = AppState::from_ctx(ctx).await else { return };
    let pool = app_state.db.clone();

    let options = &interaction.data.options;
    let Some(user_id) = options.iter().find(|o| o.name == "user").and_then(|o| o.value.as_user_id()) else {
        interaction.edit_response(&ctx.http, serenity::builder::EditInteractionResponse::new().content("Missing user option" )).await.ok();
        return;
    };
    let Ok(receiver_user) = user_id.to_user(&ctx.http).await else {
        interaction.edit_response(&ctx.http, serenity::builder::EditInteractionResponse::new().content("Failed to resolve user" )).await.ok();
        return;
    };
    let Some(item_str) = options.iter().find(|o| o.name == "item").and_then(|o| o.value.as_str()) else {
        interaction.edit_response(&ctx.http, serenity::builder::EditInteractionResponse::new().content("Missing item option" )).await.ok();
        return;
    };
    let quantity = options
        .iter()
        .find(|opt| opt.name == "quantity")
        .and_then(|opt| opt.value.as_i64())
        .unwrap_or(1);

    let Ok(item) = Item::from_str(item_str) else {
        interaction.edit_response(&ctx.http, serenity::builder::EditInteractionResponse::new().content("Invalid item" )).await.ok();
        return;
    };

    let embed = give_item(&pool, &interaction.user, &receiver_user, item, quantity).await;
    let builder = CreateInteractionResponseFollowup::new().embed(embed);
    interaction.create_followup(&ctx.http, builder).await.ok();
}

pub async fn run_prefix(ctx: &Context, msg: &Message, args: Vec<&str>) {
    let Some(app_state) = AppState::from_ctx(ctx).await else { return };
    let pool = app_state.db.clone();

    let receiver = match msg.mentions.first() {
        Some(user) => user,
        None => {
            msg.reply(
                ctx,
                "You must mention a user to give an item to! (e.g., `$give @user fish 10`)",
            )
            .await
            .ok();
            return;
        }
    };

    let item_name = match args.get(1) {
        Some(name) => *name,
        None => {
            msg.reply(
                ctx,
                "You need to specify what to give! (e.g., `$give @user fish 10`)",
            )
            .await
            .ok();
            return;
        }
    };

    let item = match Item::from_str(item_name) {
        Ok(item) => item,
        Err(_) => {
            msg.reply(ctx, &format!("'{}' is not a valid item.", item_name))
                .await
                .ok();
            return;
        }
    };

    let quantity = args.get(2).and_then(|q| q.parse::<i64>().ok()).unwrap_or(1);

    let embed = give_item(&pool, &msg.author, receiver, item, quantity).await;
    let builder = CreateMessage::new().embed(embed).reference_message(msg);
    msg.channel_id.send_message(&ctx.http, builder).await.ok();
}
