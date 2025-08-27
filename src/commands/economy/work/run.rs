//! This module implements the `work` command, supporting both prefix and slash commands.

use super::logic::perform_work;
use crate::AppState;
use serenity::builder::{CreateInteractionResponseFollowup, CreateMessage};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::*;

pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    interaction.defer_ephemeral(&ctx.http).await.ok();

    let pool = { ctx.data.read().await.get::<AppState>().unwrap().db.clone() };
    let job_name = interaction
        .data
        .options
        .iter()
        .find(|opt| opt.name == "job")
        .and_then(|opt| opt.value.as_str())
        .unwrap_or("fishing");

    let embed = perform_work(&pool, &interaction.user, job_name).await;
    let builder = CreateInteractionResponseFollowup::new().embed(embed);
    interaction.create_followup(&ctx.http, builder).await.ok();
}

pub async fn run_prefix(ctx: &Context, msg: &Message, args: Vec<&str>) {
    let pool = { ctx.data.read().await.get::<AppState>().unwrap().db.clone() };
    let job_name = args.first().cloned().unwrap_or("fishing");

    let embed = perform_work(&pool, &msg.author, job_name).await;
    let builder = CreateMessage::new().embed(embed).reference_message(msg);
    msg.channel_id.send_message(&ctx.http, builder).await.ok();
}
