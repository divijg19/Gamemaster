//! This module implements the `prefix` command in two forms:
//! - A public slash command `/prefix` to view the current prefix.
//! - A prefix command `!prefix` to view, and `!prefix set` for admins to change the prefix.

use crate::AppState;
// (✓) FIXED: Import CreateCommand for the register function.
use serenity::builder::{
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::model::permissions::Permissions;
use serenity::prelude::*;

// (✓) NEW: Add a register function for the slash command.
pub fn register() -> CreateCommand {
    CreateCommand::new("prefix").description("Check the bot's current command prefix.")
}

/// The entry point for the public slash command `/prefix`.
pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    let prefix_lock = {
        let data = ctx.data.read().await;
        let app_state = data
            .get::<AppState>()
            .expect("Expected AppState in TypeMap.");
        app_state.prefix.clone()
    };

    let current_prefix = prefix_lock.read().await;
    let content = format!("The current command prefix is `{}`.", current_prefix);

    let response_builder = CreateInteractionResponseMessage::new()
        .content(content)
        .ephemeral(true);
    let response = CreateInteractionResponse::Message(response_builder);

    if let Err(e) = interaction.create_response(&ctx.http, response).await {
        println!("[PREFIX CMD] Error sending slash response: {:?}", e);
    }
}

/// The entry point for the prefix command `!prefix`.
pub async fn run_prefix(ctx: &Context, msg: &Message, args: Vec<&str>) {
    let prefix_lock = {
        let data = ctx.data.read().await;
        let app_state = data
            .get::<AppState>()
            .expect("Expected AppState in TypeMap.");
        app_state.prefix.clone()
    };

    match args.first().map(|s| s.as_ref()) {
        // Case: `!prefix set <new_prefix>` - ADMIN ONLY
        Some("set") => {
            let has_admin_perms = {
                let member = match msg.member(&ctx.http).await {
                    Ok(member) => member,
                    Err(_) => return,
                };

                let guild = match msg.guild(&ctx.cache) {
                    Some(guild) => guild,
                    None => return,
                };

                if member.user.id == guild.owner_id {
                    true
                } else {
                    member.roles.iter().any(|role_id| {
                        guild.roles.get(role_id).is_some_and(|role| {
                            role.permissions.contains(Permissions::ADMINISTRATOR)
                        })
                    })
                }
            };

            if !has_admin_perms {
                if let Err(e) = msg
                    .reply(
                        &ctx.http,
                        "❌ You must be an administrator to use this command.",
                    )
                    .await
                {
                    println!("[PREFIX CMD] Error sending permissions error: {:?}", e);
                }
                return;
            }

            if let Some(new_prefix) = args.get(1) {
                let mut prefix_guard = prefix_lock.write().await;
                *prefix_guard = new_prefix.to_string();
                let response = format!("✅ Prefix has been updated to `{}`", new_prefix);
                if let Err(e) = msg.reply(&ctx.http, response).await {
                    println!("[PREFIX CMD] Error sending confirmation response: {:?}", e);
                }
            } else if let Err(e) = msg
                .reply(&ctx.http, "Usage: `!prefix set <new_prefix>`")
                .await
            {
                println!("[PREFIX CMD] Error sending usage hint: {:?}", e);
            }
        }
        // Case: `!prefix` - PUBLIC
        _ => {
            let current_prefix = prefix_lock.read().await;
            let response = format!(
                "The current prefix is `{}`. Use `!prefix set <new_prefix>` to change it.",
                current_prefix
            );
            if let Err(e) = msg.reply(&ctx.http, response).await {
                println!("[PREFIX CMD] Error sending current prefix info: {:?}", e);
            }
        }
    }
}
