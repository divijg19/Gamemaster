//! Implements the run logic for the `/saga` command.

use super::ui::create_saga_menu;
use crate::{AppState, database};
use serenity::builder::{
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage,
};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::*;

pub fn register() -> CreateCommand {
    CreateCommand::new("saga").description("Open the main menu for the Gamemaster Saga.")
}

pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    // Defer the response immediately to give us time to fetch from the database.
    let _ = interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new()),
        )
        .await;

    let pool = { ctx.data.read().await.get::<AppState>().unwrap().db.clone() };

    // This function automatically updates AP/TP before fetching, ensuring the data is always current.
    let saga_profile =
        match database::profile::update_and_get_saga_profile(&pool, interaction.user.id).await {
            Ok(profile) => profile,
            Err(e) => {
                println!("[SAGA CMD] Database error: {:?}", e);
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

    let (embed, components) = create_saga_menu(&saga_profile);
    let builder = serenity::builder::EditInteractionResponse::new()
        .embed(embed)
        .components(components);

    interaction.edit_response(&ctx.http, builder).await.ok();
}

pub async fn run_prefix(ctx: &Context, msg: &Message, _args: Vec<&str>) {
    let pool = { ctx.data.read().await.get::<AppState>().unwrap().db.clone() };

    let saga_profile =
        match database::profile::update_and_get_saga_profile(&pool, msg.author.id).await {
            Ok(profile) => profile,
            Err(e) => {
                println!("[SAGA CMD] Database error: {:?}", e);
                msg.reply(ctx, "Could not retrieve your game profile.")
                    .await
                    .ok();
                return;
            }
        };

    let (embed, components) = create_saga_menu(&saga_profile);
    let builder = CreateMessage::new()
        .embed(embed)
        .components(components)
        .reference_message(msg);
    msg.channel_id.send_message(&ctx.http, builder).await.ok();
}
