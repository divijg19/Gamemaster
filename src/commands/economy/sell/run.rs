//! Handles the command logic for `/sell` and `$sell`.

use super::logic::sell_items;
use crate::AppState;
use crate::commands::economy::core::item::Item;
use serenity::builder::{
    CreateCommand, CreateCommandOption, CreateInteractionResponseFollowup, CreateMessage,
};
use serenity::model::application::{CommandInteraction, CommandOptionType};
use serenity::model::channel::Message;
use serenity::prelude::*;

pub fn register() -> CreateCommand {
    CreateCommand::new("sell")
        .description("Sell your collected resources for coins.")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "item",
                "The item you want to sell.",
            )
            .required(true)
            .add_string_choice("Fish", "fish")
            .add_string_choice("Ore", "ore")
            .add_string_choice("Gem", "gem")
            .add_string_choice("Golden Fish", "goldenfish"),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "quantity",
                "The amount to sell. Sells all by default.",
            )
            .required(false)
            .min_int_value(1),
        )
}

pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    interaction.defer_ephemeral(&ctx.http).await.ok();
    let pool = { ctx.data.read().await.get::<AppState>().unwrap().db.clone() };

    let item_str = interaction
        .data
        .options
        .iter()
        .find(|opt| opt.name == "item")
        .and_then(|opt| opt.value.as_str())
        .unwrap_or_default();
    let item = match item_str {
        "fish" => Item::Fish,
        "ore" => Item::Ore,
        "gem" => Item::Gem,
        "goldenfish" => Item::GoldenFish,
        _ => {
            // This case is unreachable due to Discord's command choices
            return;
        }
    };

    let quantity = interaction
        .data
        .options
        .iter()
        .find(|opt| opt.name == "quantity")
        .and_then(|opt| opt.value.as_i64());

    let embed = sell_items(&pool, &interaction.user, item, quantity).await;
    let builder = CreateInteractionResponseFollowup::new().embed(embed);
    interaction.create_followup(&ctx.http, builder).await.ok();
}

pub async fn run_prefix(ctx: &Context, msg: &Message, args: Vec<&str>) {
    let pool = { ctx.data.read().await.get::<AppState>().unwrap().db.clone() };

    let item_name = match args.first() {
        Some(name) => *name,
        None => {
            msg.reply(
                ctx,
                "You need to specify what to sell! (e.g., `$sell fish 10`)",
            )
            .await
            .ok();
            return;
        }
    };

    let item = match item_name.to_lowercase().as_str() {
        "fish" => Item::Fish,
        "ore" => Item::Ore,
        "gem" => Item::Gem,
        "goldenfish" | "golden" => Item::GoldenFish,
        _ => {
            msg.reply(ctx, &format!("'{}' is not a sellable item.", item_name))
                .await
                .ok();
            return;
        }
    };

    // Parse the second argument for quantity, if it exists and is a valid number.
    let quantity = args.get(1).and_then(|q| q.parse::<i64>().ok());

    let embed = sell_items(&pool, &msg.author, item, quantity).await;
    let builder = CreateMessage::new().embed(embed).reference_message(msg);
    msg.channel_id.send_message(&ctx.http, builder).await.ok();
}
