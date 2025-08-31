//! Implements the run logic for the `/party` command.

use super::ui::create_party_view;
use crate::{AppState, database};
use serenity::builder::{
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage,
    EditInteractionResponse,
};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::*;

pub fn register() -> CreateCommand {
    CreateCommand::new("party").description("Manage your active battle party and army.")
}

pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    // Defer the response ephemerally so only the user sees their party.
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

    // First, run the main saga update function. This is crucial because it will
    // apply any completed training sessions before we fetch the pet list.
    let _ = database::saga::update_and_get_saga_profile(&pool, interaction.user.id).await;

    // Now, fetch the fresh list of all pets the player owns.
    let pets = match database::pets::get_player_pets(&pool, interaction.user.id).await {
        Ok(p) => p,
        Err(e) => {
            println!("[PARTY CMD] DB error getting player pets: {:?}", e);
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

    // Generate the UI with the fetched pet data.
    let (embed, components) = create_party_view(&pets);
    let builder = EditInteractionResponse::new()
        .embed(embed)
        .components(components);

    interaction.edit_response(&ctx.http, builder).await.ok();
}

pub async fn run_prefix(ctx: &Context, msg: &Message, _args: Vec<&str>) {
    let pool = { ctx.data.read().await.get::<AppState>().unwrap().db.clone() };

    // Same logic as the slash command: update first, then fetch.
    let _ = database::saga::update_and_get_saga_profile(&pool, msg.author.id).await;
    let pets = match database::pets::get_player_pets(&pool, msg.author.id).await {
        Ok(p) => p,
        Err(e) => {
            println!("[PARTY CMD] DB error getting player pets: {:?}", e);
            msg.reply(ctx, "Could not retrieve your pets.").await.ok();
            return;
        }
    };

    let (embed, components) = create_party_view(&pets);
    let builder = CreateMessage::new()
        .embed(embed)
        .components(components)
        .reference_message(msg);
    msg.channel_id.send_message(&ctx.http, builder).await.ok();
}
