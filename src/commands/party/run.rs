//! Implements the run logic for the `/party` command.

use super::ui::create_party_view_with_bonds;
use crate::{AppState, services};
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

    let Some(app_state) = AppState::from_ctx(ctx).await else {
        return;
    };

    // Batched update + units retrieval
    if services::saga::get_profile_and_units(&app_state, interaction.user.id)
        .await
        .is_none()
    {
        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().content("Could not retrieve your units."),
            )
            .await
            .ok();
        return;
    }

    // Generate the UI with the fetched unit data.
    let (embed, components) = create_party_view_with_bonds(&app_state, interaction.user.id).await;
    let builder = EditInteractionResponse::new()
        .embed(embed)
        .components(components);

    interaction.edit_response(&ctx.http, builder).await.ok();
}

pub async fn run_prefix(ctx: &Context, msg: &Message, _args: Vec<&str>) {
    let Some(app_state) = AppState::from_ctx(ctx).await else {
        return;
    };

    if services::saga::get_profile_and_units(&app_state, msg.author.id)
        .await
        .is_none()
    {
        msg.reply(ctx, "Could not retrieve your units.").await.ok();
        return;
    }

    let (embed, components) = create_party_view_with_bonds(&app_state, msg.author.id).await;
    let builder = CreateMessage::new()
        .embed(embed)
        .components(components)
        .reference_message(msg);
    msg.channel_id.send_message(&ctx.http, builder).await.ok();
}
