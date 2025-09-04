//! Implements the run logic for the `/party` command.

use super::ui::create_party_view_with_bonds;
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

    let Some(app_state) = AppState::from_ctx(ctx).await else { return; };

    // First, run the main saga update function. This is crucial because it will
    // apply any completed training sessions before we fetch the unit list.
    let _ = database::saga::update_and_get_saga_profile(&app_state.db, interaction.user.id).await;

    // Now, fetch the fresh list of all units the player owns.
    match database::units::get_player_units(&app_state.db, interaction.user.id).await {
        Ok(_p) => _p,
        Err(e) => {
        println!("[PARTY CMD] DB error getting player units: {:?}", e);
            interaction
                .edit_response(
                    &ctx.http,
            EditInteractionResponse::new().content("Could not retrieve your units."),
                )
                .await
                .ok();
            return;
        }
    };

    // Generate the UI with the fetched unit data.
    let (embed, components) = create_party_view_with_bonds(&app_state, interaction.user.id).await;
    let builder = EditInteractionResponse::new()
        .embed(embed)
        .components(components);

    interaction.edit_response(&ctx.http, builder).await.ok();
}

pub async fn run_prefix(ctx: &Context, msg: &Message, _args: Vec<&str>) {
    let Some(app_state) = AppState::from_ctx(ctx).await else { return; };

    // Same logic as the slash command: update first, then fetch.
    let _ = database::saga::update_and_get_saga_profile(&app_state.db, msg.author.id).await;
    match database::units::get_player_units(&app_state.db, msg.author.id).await {
        Ok(_p) => _p,
        Err(e) => {
            println!("[PARTY CMD] DB error getting player units: {:?}", e);
            msg.reply(ctx, "Could not retrieve your units.").await.ok();
            return;
        }
    };

    let (embed, components) = create_party_view_with_bonds(&app_state, msg.author.id).await;
    let builder = CreateMessage::new()
        .embed(embed)
        .components(components)
        .reference_message(msg);
    msg.channel_id.send_message(&ctx.http, builder).await.ok();
}
