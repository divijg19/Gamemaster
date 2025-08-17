use std::collections::HashMap;
use std::sync::Arc;

use serenity::model::channel::Message;
use serenity::model::guild::Role;
use serenity::model::id::{RoleId, UserId};
use serenity::model::permissions::Permissions;
use serenity::prelude::*;
use tokio::sync::RwLock;

// This helper struct and function now live here, self-contained with the command that uses them.
struct GuildInfo {
    owner_id: UserId,
    roles: HashMap<RoleId, Role>,
}

fn get_guild_info_from_cache(ctx: &Context, msg: &Message) -> Option<GuildInfo> {
    let guild = ctx.cache.guild(msg.guild_id?)?;

    Some(GuildInfo {
        owner_id: guild.owner_id,
        roles: guild.roles.clone(),
    })
}

// The main function for the `prefix` command.
pub async fn run(ctx: &Context, msg: &Message, prefix: Arc<RwLock<String>>, args: Vec<&str>) {
    let guild_info = match get_guild_info_from_cache(ctx, msg) {
        Some(info) => info,
        None => {
            let _ = msg
                .reply(
                    ctx,
                    "Could not get server info from cache. Please try again.",
                )
                .await;
            return;
        }
    };

    let is_owner = msg.author.id == guild_info.owner_id;

    let has_admin_role = if let Some(member) = &msg.member {
        member.roles.iter().any(|role_id| {
            guild_info
                .roles
                .get(role_id)
                .is_some_and(|role| role.permissions.contains(Permissions::ADMINISTRATOR))
        })
    } else {
        false
    };

    if !is_owner && !has_admin_role {
        let _ = msg
            .reply(ctx, "You must be an administrator to use this command.")
            .await;
        return;
    }

    match args.first() {
        Some(&"set") => {
            if let Some(new_prefix) = args.get(1) {
                let mut prefix_guard = prefix.write().await;
                *prefix_guard = new_prefix.to_string();
                let response = format!("Prefix has been updated to `{}`", new_prefix);
                let _ = msg.reply(ctx, response).await;
            } else {
                let _ = msg.reply(ctx, "Usage: `!prefix set <new_prefix>`").await;
            }
        }
        _ => {
            let current_prefix = prefix.read().await;
            let response = format!(
                "The current prefix is `{}`. Use `!prefix set <new_prefix>` to change it.",
                current_prefix
            );
            let _ = msg.reply(ctx, response).await;
        }
    }
}
