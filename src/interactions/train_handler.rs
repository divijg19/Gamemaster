//! Handles all component interactions for the `train` command family.

use crate::{AppState, commands, database};
use serenity::builder::EditInteractionResponse;
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;
use std::sync::Arc;

pub async fn handle(
    // (✓) FIXED: Renamed `_ctx` to `ctx` so it can be used in the function.
    ctx: &Context,
    component: &mut ComponentInteraction,
    app_state: Arc<AppState>,
) {
    let db = app_state.db.clone();
    component.defer_ephemeral(&ctx.http).await.ok();

    let custom_id_parts: Vec<&str> = component.data.custom_id.split('_').collect();

    match custom_id_parts.get(1) {
        // This handles the first step: the user selects a pet from the dropdown.
        Some(&"select") => {
            let pet_id_str =
                if let serenity::model::application::ComponentInteractionDataKind::StringSelect {
                    values,
                } = &component.data.kind
                {
                    &values[0]
                } else {
                    return;
                };
            let pet_id = pet_id_str.parse::<i32>().unwrap();

            // We respond by showing the stat selection menu.
            let (embed, components) = commands::train::ui::create_stat_selection_menu(pet_id);
            let builder = EditInteractionResponse::new()
                .embed(embed)
                .components(components);
            component.edit_response(&ctx.http, builder).await.ok();
        }
        // This handles the second step: the user clicks a "Train Stat" button.
        Some(&"stat") => {
            let stat_to_train = custom_id_parts[2];
            let player_pet_id = custom_id_parts[3].parse::<i32>().unwrap();

            // Call the database function to start the training session.
            let success = database::pets::start_training(
                &db,
                component.user.id,
                player_pet_id,
                stat_to_train,
                2, // Duration in hours
                1, // TP cost
            )
            .await
            .unwrap_or(false);

            // Respond with a confirmation or error message.
            let mut builder = EditInteractionResponse::new().components(vec![]);
            if success {
                builder = builder.content(format!(
                    "Training has begun! Your pet will gain +1 {} in 2 hours.",
                    stat_to_train
                ));
            } else {
                builder = builder.content("Failed to start training. You may not have enough Training Points, or the pet does not belong to you.");
            }
            component.edit_response(&ctx.http, builder).await.ok();
        }
        _ => {}
    }
}
