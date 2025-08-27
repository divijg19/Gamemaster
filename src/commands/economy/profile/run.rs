//! Implements the `/profile` command.

use super::ui::create_profile_embed;
use crate::{AppState, database};
use serenity::builder::{CreateInteractionResponseFollowup, CreateMessage};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::*;

pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    interaction.defer_ephemeral(&ctx.http).await.ok();
    let pool = { ctx.data.read().await.get::<AppState>().unwrap().db.clone() };

    // This correctly handles fetching the user from the slash command option or defaulting to the interaction user.
    let user_to_fetch = if let Some(option) = interaction.data.options.first() {
        if let Some(user_id) = option.value.as_user_id() {
            user_id.to_user(&ctx.http).await.ok()
        } else {
            None
        }
    } else {
        None
    }
    .unwrap_or_else(|| interaction.user.clone());

    let profile = database::profile::get_or_create_profile(&pool, user_to_fetch.id).await;
    let inventory = database::profile::get_inventory(&pool, user_to_fetch.id).await;

    let embed = create_profile_embed(&user_to_fetch, profile, inventory);
    let builder = CreateInteractionResponseFollowup::new().embed(embed);
    interaction.create_followup(&ctx.http, builder).await.ok();
}

pub async fn run_prefix(ctx: &Context, msg: &Message, _args: Vec<&str>) {
    let pool = { ctx.data.read().await.get::<AppState>().unwrap().db.clone() };

    // (✓) MODIFIED: Allow fetching a mentioned user's profile, or default to the author.
    let user_to_fetch = if let Some(mentioned_user) = msg.mentions.first() {
        mentioned_user.clone()
    } else {
        msg.author.clone()
    };

    let profile = database::profile::get_or_create_profile(&pool, user_to_fetch.id).await;
    let inventory = database::profile::get_inventory(&pool, user_to_fetch.id).await;

    let embed = create_profile_embed(&user_to_fetch, profile, inventory);
    let builder = CreateMessage::new().embed(embed).reference_message(msg);
    msg.channel_id.send_message(&ctx.http, builder).await.ok();
}
