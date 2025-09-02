//! Contains the run logic for the `/tasks` command.

use crate::commands::tasks::ui;
use crate::{AppState, database};
// (✓) NEW: Import `CreateActionRow` to handle the new return type.
use serenity::all::{
    CommandInteraction, Context, CreateActionRow, CreateInteractionResponse,
    CreateInteractionResponseMessage, EditInteractionResponse, Message,
};
// (✓) NEW: Import `CreateEmbed` directly.
use serenity::builder::CreateEmbed;

/// A shared helper function containing the core logic for the tasks command.
/// This function is responsible for fetching data and preparing the visual response.
// (✓) MODIFIED: The function now returns a tuple with the embed and its components.
async fn get_tasks_response(
    ctx: &Context,
    user_id: serenity::model::id::UserId,
) -> Result<(CreateEmbed, Vec<CreateActionRow>), String> {
    let db = {
        let data = ctx.data.read().await;
        let app_state = data
            .get::<AppState>()
            .expect("Expected AppState in TypeMap.");
        app_state.db.clone()
    };

    match database::tasks::get_or_assign_player_tasks(&db, user_id).await {
        // (✓) MODIFIED: The call to `create_tasks_embed` now returns a tuple, which we pass on.
        Ok(tasks) => Ok(ui::create_tasks_embed(&tasks)),
        Err(e) => {
            println!(
                "[DB ERROR] Failed to get/assign tasks for {}: {:?}",
                user_id, e
            );
            Err("❌ Could not retrieve your tasks. Please try again later.".to_string())
        }
    }
}

/// The entry point for the slash command `/tasks`.
pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    // Defer the response ephemerally as fetching tasks is not instant.
    if let Err(why) = interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
    {
        println!("Error deferring /tasks interaction: {:?}", why);
        return;
    }

    // Call the shared logic to get the response.
    // (✓) MODIFIED: The builder now handles both the embed and the components.
    let response_builder = match get_tasks_response(ctx, interaction.user.id).await {
        Ok((embed, components)) => EditInteractionResponse::new()
            .embed(embed)
            .components(components),
        Err(content) => EditInteractionResponse::new().content(content),
    };

    // Send the final response.
    if let Err(why) = interaction.edit_response(&ctx.http, response_builder).await {
        println!("Error sending /tasks response: {:?}", why);
    }
}

/// The entry point for the prefix command `!tasks`.
pub async fn run_prefix(ctx: &Context, msg: &Message) {
    // Call the shared logic to get the response.
    // (✓) MODIFIED: The builder now handles both the embed and the components.
    match get_tasks_response(ctx, msg.author.id).await {
        Ok((embed, components)) => {
            if let Err(why) = msg
                .channel_id
                .send_message(
                    &ctx.http,
                    serenity::all::CreateMessage::new()
                        .embed(embed)
                        .components(components),
                )
                .await
            {
                println!("Error sending !tasks prefix response: {:?}", why);
            }
        }
        Err(content) => {
            if let Err(why) = msg.channel_id.say(&ctx.http, content).await {
                println!("Error sending !tasks prefix error response: {:?}", why);
            }
        }
    };
}
