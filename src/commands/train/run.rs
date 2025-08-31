//! Implements the run logic for the `/train` command.

use super::ui::create_training_menu;
use crate::{AppState, database};
use serenity::builder::{
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage,
    EditInteractionResponse,
};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::*;

pub fn register() -> CreateCommand {
    CreateCommand::new("train").description("Train your pets to improve their stats.")
}

pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    // Ephemeral defer ensures only the user sees the menu.
    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
        .ok();

    let pool = { ctx.data.read().await.get::<AppState>().unwrap().db.clone() };

    // First, get the player's up-to-date saga profile.
    let saga_profile = match database::saga::update_and_get_saga_profile(&pool, interaction.user.id)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            println!("[TRAIN CMD] DB error getting saga profile: {:?}", e);
            interaction
                .edit_response(
                    &ctx.http,
                    EditInteractionResponse::new().content("Could not retrieve your game profile."),
                )
                .await
                .ok();
            return;
        }
    };

    // Next, get the list of all pets the player owns.
    let pets = match database::pets::get_player_pets(&pool, interaction.user.id).await {
        Ok(p) => p,
        Err(e) => {
            println!("[TRAIN CMD] DB error getting player pets: {:?}", e);
            interaction
                .edit_response(
                    &ctx.http,
                    EditInteractionResponse::new().content("Could not retrieve your pets."),
                )
                .await
                .ok();
            return;
        }
    };

    let (embed, components) = create_training_menu(&pets, &saga_profile);

    let builder = EditInteractionResponse::new()
        .embed(embed)
        .components(components);

    interaction.edit_response(&ctx.http, builder).await.ok();
}

pub async fn run_prefix(ctx: &Context, msg: &Message, _args: Vec<&str>) {
    let pool = { ctx.data.read().await.get::<AppState>().unwrap().db.clone() };

    let saga_profile = match database::saga::update_and_get_saga_profile(&pool, msg.author.id).await
    {
        Ok(p) => p,
        Err(e) => {
            println!("[TRAIN CMD] DB error getting saga profile: {:?}", e);
            msg.reply(ctx, "Could not retrieve your game profile.")
                .await
                .ok();
            return;
        }
    };

    let pets = match database::pets::get_player_pets(&pool, msg.author.id).await {
        Ok(p) => p,
        Err(e) => {
            println!("[TRAIN CMD] DB error getting player pets: {:?}", e);
            msg.reply(ctx, "Could not retrieve your pets.").await.ok();
            return;
        }
    };

    let (embed, components) = create_training_menu(&pets, &saga_profile);
    let builder = CreateMessage::new()
        .embed(embed)
        .components(components)
        .reference_message(msg);
    msg.channel_id.send_message(&ctx.http, builder).await.ok();
}
