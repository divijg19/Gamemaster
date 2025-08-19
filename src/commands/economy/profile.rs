//! This module implements the `profile` command in both prefix and slash formats.
//! It allows users to view their or another user's economic profile, including balance and inventory.

use crate::{AppState, database};
use serenity::builder::{CreateEmbed, CreateInteractionResponseFollowup, CreateMessage};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::model::user::User;
use serenity::prelude::*;

/// A shared helper function that creates the profile embed.
async fn create_profile_embed(pool: &database::init::DbPool, user: &User) -> CreateEmbed {
    match database::profile::get_or_create_profile(pool, user.id).await {
        Ok(profile) => {
            let mut inventory_parts = Vec::new();
            if profile.fish > 0 {
                inventory_parts.push(format!("🐟 Fish: `{}`", profile.fish));
            }
            if profile.ores > 0 {
                inventory_parts.push(format!("⛏️ Ores: `{}`", profile.ores));
            }
            if profile.gems > 0 {
                inventory_parts.push(format!("💎 Gems: `{}`", profile.gems));
            }
            if profile.rare_finds > 0 {
                inventory_parts.push(format!("🌟 Rare Finds: `{}`", profile.rare_finds));
            }

            let inventory_string = if inventory_parts.is_empty() {
                "Your inventory is empty.".to_string()
            } else {
                inventory_parts.join("\n")
            };

            CreateEmbed::new()
                .title(format!("{}'s Profile", user.name))
                .thumbnail(user.avatar_url().unwrap_or_default())
                .field("💰 Balance", format!("`{}` coins", profile.balance), true)
                .field("\u{200B}", "\u{200B}", true) // Blank inline field for spacing
                .field("📦 Inventory", inventory_string, false)
                .color(0x5865F2)
        }
        Err(e) => {
            println!(
                "[PROFILE CMD] Error retrieving profile for user {}: {:?}",
                user.id, e
            );
            CreateEmbed::new()
                .title("Error")
                .description("Could not retrieve the profile. Please try again later.")
                .color(0xFF0000)
        }
    }
}

/// The entry point for the slash command `/profile`.
pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    if let Err(e) = interaction.defer_ephemeral(&ctx.http).await {
        println!("[PROFILE CMD] Failed to defer slash interaction: {:?}", e);
        return;
    }

    let pool = {
        let data = ctx.data.read().await;
        let app_state = data
            .get::<AppState>()
            .expect("Expected AppState in TypeMap.");
        app_state.db.clone()
    };

    // (✓) DEFINITIVE FIX: This is the robust way to handle an optional, async operation.
    // We explicitly handle the Some/None cases and the Ok/Err cases.
    let user_to_check = {
        // First, get the optional UserId from the command's arguments.
        let optional_user_id = interaction
            .data
            .options
            .iter()
            .find(|opt| opt.name == "user")
            .and_then(|opt| opt.value.as_user_id());

        match optional_user_id {
            // If a user was mentioned...
            Some(user_id) => {
                // ...await the async call to fetch the full User object.
                match user_id.to_user(&ctx.http).await {
                    // If fetching is successful, that's our user.
                    Ok(user) => user,
                    // If fetching fails, log it and fall back to the author.
                    Err(e) => {
                        println!(
                            "[PROFILE CMD] Failed to fetch user object from slash command: {:?}",
                            e
                        );
                        interaction.user.clone()
                    }
                }
            }
            // If no user was mentioned, default to the author of the command.
            None => interaction.user.clone(),
        }
    };

    let embed = create_profile_embed(&pool, &user_to_check).await;
    let builder = CreateInteractionResponseFollowup::new().embed(embed);

    if let Err(e) = interaction.create_followup(&ctx.http, builder).await {
        println!("[PROFILE CMD] Failed to send slash followup: {:?}", e);
    }
}

/// The entry point for the prefix command `!profile`.
pub async fn run_prefix(ctx: &Context, msg: &Message) {
    let pool = {
        let data = ctx.data.read().await;
        let app_state = data
            .get::<AppState>()
            .expect("Expected AppState in TypeMap.");
        app_state.db.clone()
    };

    let user_to_check = msg.mentions.first().map_or(&msg.author, |u| u);
    let embed = create_profile_embed(&pool, user_to_check).await;
    let builder = CreateMessage::new().embed(embed).reference_message(msg);

    if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
        println!("[PROFILE CMD] Failed to send prefix response: {:?}", e);
    }
}
