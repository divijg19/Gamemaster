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
    let (saga_profile, has_party) = match services::saga::get_profile_and_units_debug(
        &app_state,
        interaction.user.id,
    )
    .await
    {
        Ok((profile, units)) => (profile, units.iter().any(|u| u.is_in_party)),
        Err(e) => {
            println!(
                "[SAGA CMD] debug retrieval failed user={} error={:?}",
                interaction.user.id.get(),
                e
            );
            // Provide targeted guidance based on error category
            let mut msg = String::from("Could not retrieve your game profile. ");
            use sqlx::Error::*;
            match &e {
                Database(db_err) => {
                    let code_cow = db_err.code().unwrap_or(std::borrow::Cow::Borrowed("?"));
                    let code = code_cow.as_ref();
                    if code == "42P01" {
                        // undefined_table
                        msg.push_str("Migration missing: run migrations (\n`sqlx migrate run`\n) to create saga tables.");
                    } else {
                        msg.push_str(&format!("Database error ({}): {}", code, db_err));
                    }
                }
                Io(_) | PoolTimedOut | Tls(_) => {
                    msg.push_str("Database connection issue. Check DATABASE_URL and connectivity.");
                }
                RowNotFound => {
                    msg.push_str("Profile row not found after creation attempt.");
                }
                _ => {
                    msg.push_str(&format!("Unexpected error: {e}"));
                }
            }
            interaction
                .edit_response(
                    &ctx.http,
                    serenity::builder::EditInteractionResponse::new().content(msg),
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
        match services::saga::get_profile_and_units_debug(&app_state, msg.author.id).await {
            Ok((profile, units)) => (profile, units.iter().any(|u| u.is_in_party)),
            Err(e) => {
                println!(
                    "[SAGA CMD][prefix] debug retrieval failed user={} error={:?}",
                    msg.author.id.get(),
                    e
                );
                let mut content = String::from("Could not retrieve your game profile. ");
                use sqlx::Error::*;
                match &e {
                    Database(db_err) => {
                        let code_cow = db_err.code().unwrap_or(std::borrow::Cow::Borrowed("?"));
                        let code = code_cow.as_ref();
                        if code == "42P01" {
                            content.push_str("Migration missing. Run `sqlx migrate run`. ");
                        } else {
                            content.push_str(&format!("DB error ({}): {}", code, db_err));
                        }
                    }
                    Io(_) | PoolTimedOut | Tls(_) => {
                        content.push_str("Connection issue (check DATABASE_URL).")
                    }
                    RowNotFound => {
                        content.push_str("Profile row not found after creation attempt.")
                    }
                    _ => content.push_str(&format!("Unexpected error: {e}")),
                }
                msg.reply(ctx, content).await.ok();
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
