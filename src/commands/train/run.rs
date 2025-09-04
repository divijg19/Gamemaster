//! Implements the run logic for the `/train` command.

use super::ui::create_training_menu;
use crate::{AppState, services};
use serenity::builder::{
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage,
    EditInteractionResponse,
};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::*;

pub fn register() -> CreateCommand {
    CreateCommand::new("train").description("Train your units to improve their stats.")
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

    let Some(app_state) = AppState::from_ctx(ctx).await else {
        return;
    };
    // DB pool accessible via app_state if needed for future expansion.

    // Batched profile + units retrieval (saves a DB round trip; includes training completion logic)
    let (saga_profile, pets) =
        match services::saga::get_profile_and_units(&app_state, interaction.user.id).await {
            Some((profile, units)) => (profile, units),
            None => {
                println!("[TRAIN CMD] failed combined profile+units fetch");
                interaction
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new()
                            .content("Could not retrieve your game data."),
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
    let Some(app_state) = AppState::from_ctx(ctx).await else {
        return;
    };
    // DB pool accessible via app_state if needed for future expansion.

    let (saga_profile, pets) =
        match services::saga::get_profile_and_units(&app_state, msg.author.id).await {
            Some((profile, units)) => (profile, units),
            None => {
                println!("[TRAIN CMD] failed combined profile+units fetch (prefix)");
                msg.reply(ctx, "Could not retrieve your game data.")
                    .await
                    .ok();
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
