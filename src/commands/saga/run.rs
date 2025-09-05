//! Implements the run logic for the `/saga` command.

use super::ui::{create_first_time_tutorial, create_saga_menu};
use crate::{AppState, database, services};
use serenity::builder::CreateEmbed;
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

/// Internal shared execution that returns either the unified menu/tutorial or a rich error string.
async fn execute_saga(
    app_state: &AppState,
    user_id: serenity::model::id::UserId,
) -> Result<(CreateEmbed, Vec<serenity::builder::CreateActionRow>), String> {
    // Ensure base profile (economy) exists (FK for saga profile).
    if let Err(e) = database::economy::get_or_create_profile(&app_state.db, user_id).await {
        use sqlx::Error::*;
        let mut msg = format!("Failed to create base profile (user {}). ", user_id.get());
        match &e {
            Database(db_err) => {
                let code_cow = db_err.code().unwrap_or(std::borrow::Cow::Borrowed("?"));
                let code = code_cow.as_ref();
                if code == "42P01" {
                    msg.push_str("Missing tables. Run migrations: `sqlx migrate run`.");
                } else {
                    msg.push_str(&format!("Database error ({}): {}", code, db_err));
                }
            }
            Io(_) | PoolTimedOut | Tls(_) => {
                msg.push_str("Connectivity issue (check DATABASE_URL).")
            }
            _ => msg.push_str(&format!("Unexpected error: {e}")),
        }
        println!(
            "[SAGA CMD] base profile ensure failed user={} err={:?}",
            user_id.get(),
            e
        );
        return Err(msg);
    }
    match services::saga::get_profile_and_units_debug(app_state, user_id).await {
        Ok((profile, units)) => {
            let has_party = units.iter().any(|u| u.is_in_party);
            let (embed, components) = if !has_party && profile.story_progress == 0 {
                create_first_time_tutorial()
            } else {
                create_saga_menu(&profile, has_party)
            };
            Ok((embed, components))
        }
        Err(e) => {
            use sqlx::Error::*;
            println!(
                "[SAGA CMD] retrieval failed user={} err={:?}",
                user_id.get(),
                e
            );
            let mut msg = String::from("Could not retrieve your game profile. ");
            match &e {
                Database(db_err) => {
                    let code_cow = db_err.code().unwrap_or(std::borrow::Cow::Borrowed("?"));
                    let code = code_cow.as_ref();
                    if code == "42P01" {
                        msg.push_str(
                            "Migration missing: run `sqlx migrate run` to create saga tables.",
                        );
                    } else {
                        msg.push_str(&format!("Database error ({}): {}", code, db_err));
                    }
                }
                Io(_) | PoolTimedOut | Tls(_) => {
                    msg.push_str("Database connectivity issue (check DATABASE_URL / network).")
                }
                RowNotFound => msg.push_str("Profile row not found right after creation attempt."),
                _ => msg.push_str(&format!("Unexpected error: {e}")),
            }
            Err(msg)
        }
    }
}

pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    let _ = interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new()),
        )
        .await;
    let Some(app_state) = AppState::from_ctx(ctx).await else {
        return;
    };
    match execute_saga(&app_state, interaction.user.id).await {
        Ok((embed, components)) => {
            let builder = serenity::builder::EditInteractionResponse::new()
                .embed(embed)
                .components(components);
            interaction.edit_response(&ctx.http, builder).await.ok();
        }
        Err(err) => {
            interaction
                .edit_response(
                    &ctx.http,
                    serenity::builder::EditInteractionResponse::new().content(err),
                )
                .await
                .ok();
        }
    }
}

pub async fn run_prefix(ctx: &Context, msg: &Message, _args: Vec<&str>) {
    let Some(app_state) = AppState::from_ctx(ctx).await else {
        return;
    };
    match execute_saga(&app_state, msg.author.id).await {
        Ok((embed, components)) => {
            let builder = CreateMessage::new()
                .embed(embed)
                .components(components)
                .reference_message(msg);
            msg.channel_id.send_message(&ctx.http, builder).await.ok();
        }
        Err(err) => {
            msg.reply(ctx, err).await.ok();
        }
    }
}
