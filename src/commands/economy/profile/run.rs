//! Implements the `/profile` command.

use super::ui::create_profile_embed;
use crate::{AppState, database, services};
use serenity::builder::{CreateInteractionResponseFollowup, CreateMessage};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::*;

use serenity::builder::{CreateCommand, CreateCommandOption};
use serenity::model::application::CommandOptionType;

pub fn register() -> CreateCommand {
    CreateCommand::new("profile")
        .description("View your or another user's economy profile.")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "The user to view.")
                .required(false),
        )
}

pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    interaction.defer_ephemeral(&ctx.http).await.ok();
    let Some(app_state) = AppState::from_ctx(ctx).await else {
        return;
    };
    let pool = app_state.db.clone();

    let user_to_fetch = if let Some(option) = interaction.data.options.first() {
        match option.value.as_user_id() {
            Some(id) => id
                .to_user(&ctx.http)
                .await
                .unwrap_or_else(|_| interaction.user.clone()),
            None => interaction.user.clone(),
        }
    } else {
        interaction.user.clone()
    };

    let profile = database::economy::get_or_create_profile(&pool, user_to_fetch.id).await;
    let inventory = database::economy::get_inventory(&pool, user_to_fetch.id).await;
    // (✓) MODIFIED: Call the new, intelligent update function to ensure AP/TP are always current.
    let saga_profile = services::saga::get_saga_profile(&app_state, user_to_fetch.id, false)
        .await
        .ok_or_else(|| sqlx::Error::RowNotFound);

    let embed = create_profile_embed(&user_to_fetch, profile, inventory, saga_profile);
    let builder = CreateInteractionResponseFollowup::new().embed(embed);
    interaction.create_followup(&ctx.http, builder).await.ok();
}

pub async fn run_prefix(ctx: &Context, msg: &Message, _args: Vec<&str>) {
    let Some(app_state) = AppState::from_ctx(ctx).await else {
        return;
    };
    let pool = app_state.db.clone();

    let user_to_fetch = msg
        .mentions
        .first()
        .cloned()
        .unwrap_or_else(|| msg.author.clone());

    let profile = database::economy::get_or_create_profile(&pool, user_to_fetch.id).await;
    let inventory = database::economy::get_inventory(&pool, user_to_fetch.id).await;
    // (✓) MODIFIED: Call the new, intelligent update function here as well for the prefix command.
    let saga_profile = services::saga::get_saga_profile(&app_state, user_to_fetch.id, false)
        .await
        .ok_or_else(|| sqlx::Error::RowNotFound);

    let embed = create_profile_embed(&user_to_fetch, profile, inventory, saga_profile);
    let builder = CreateMessage::new().embed(embed).reference_message(msg);
    msg.channel_id.send_message(&ctx.http, builder).await.ok();
}
