//! Implements the run logic for the `/saga` command.

use super::ui::{create_first_time_tutorial, create_saga_menu};
use crate::{AppState, database, services};
use serenity::builder::{
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage,
};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::*;

pub fn register() -> CreateCommand {
    CreateCommand::new("saga").description("Open the main menu for the Gamemaster Saga.")
}

// Slash alias so users can type /play as well as /saga (prefix already supports !play)
pub fn register_play() -> CreateCommand {
    CreateCommand::new("play").description("Play the Gamemaster Saga (alias of /saga).")
}

pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    // Defer the response immediately to give us time to fetch from the database.
    let _ = interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new()),
        )
        .await;

    let Some(app_state) = AppState::from_ctx(ctx).await else {
        return;
    };
    let pool = app_state.db.clone();

    // Ensure a base economy profile exists first (foreign key for saga profile table).
    if let Err(e) = database::economy::get_or_create_profile(&pool, interaction.user.id).await {
        println!("[SAGA CMD] Failed to create base profile: {e:?}");
    }
    // This function automatically updates AP/TP before fetching, ensuring the data is always current.
    let (saga_profile, has_party) =
        match services::saga::get_profile_and_units(&app_state, interaction.user.id).await {
            Some((profile, units)) => {
                let has_party = units.iter().any(|u| u.is_in_party);
                (profile, has_party)
            }
            None => {
                interaction
                    .edit_response(
                        &ctx.http,
                        serenity::builder::EditInteractionResponse::new()
                            .content("Could not retrieve your game profile."),
                    )
                    .await
                    .ok();
                return;
            }
        };
    let (embed, components) = if !has_party && saga_profile.story_progress == 0 {
        create_first_time_tutorial()
    } else {
        create_saga_menu(&saga_profile, has_party)
    };
    let builder = serenity::builder::EditInteractionResponse::new()
        .embed(embed)
        .components(components);

    interaction.edit_response(&ctx.http, builder).await.ok();
}

pub async fn run_prefix(ctx: &Context, msg: &Message, _args: Vec<&str>) {
    let Some(app_state) = AppState::from_ctx(ctx).await else {
        return;
    };
    let pool = app_state.db.clone();

    if let Err(e) = database::economy::get_or_create_profile(&pool, msg.author.id).await {
        println!("[SAGA CMD] Failed to create base profile (prefix): {e:?}");
    }
    let (saga_profile, has_party) =
        match services::saga::get_profile_and_units(&app_state, msg.author.id).await {
            Some((profile, units)) => (profile, units.iter().any(|u| u.is_in_party)),
            None => {
                msg.reply(ctx, "Could not retrieve your game profile.")
                    .await
                    .ok();
                return;
            }
        };
    let (embed, components) = if !has_party && saga_profile.story_progress == 0 {
        create_first_time_tutorial()
    } else {
        create_saga_menu(&saga_profile, has_party)
    };
    let builder = CreateMessage::new()
        .embed(embed)
        .components(components)
        .reference_message(msg);
    msg.channel_id.send_message(&ctx.http, builder).await.ok();
}
