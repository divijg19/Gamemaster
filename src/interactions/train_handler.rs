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
    // This handles the first step: the user selects a unit from the dropdown.
        Some(&"select") => {
            let unit_id_str =
                if let serenity::model::application::ComponentInteractionDataKind::StringSelect {
                    values,
                } = &component.data.kind
                {
                    &values[0]
                } else {
                    return;
                };
            let unit_id = match unit_id_str.parse::<i32>() {
                Ok(id) => id,
                Err(_) => {
                    let builder = EditInteractionResponse::new().content("Invalid unit id.");
                    component.edit_response(&ctx.http, builder).await.ok();
                    return;
                }
            };

            // We respond by showing the stat selection menu.
            let (embed, components) = commands::train::ui::create_stat_selection_menu(unit_id);
            let builder = EditInteractionResponse::new()
                .embed(embed)
                .components(components);
            component.edit_response(&ctx.http, builder).await.ok();
        }
        // This handles the second step: the user clicks a "Train Stat" button.
        Some(&"stat") => {
            let stat_to_train = custom_id_parts[2];
            let player_unit_id = match custom_id_parts.get(3).and_then(|s| s.parse::<i32>().ok()) {
                Some(id) => id,
                None => {
                    let builder = EditInteractionResponse::new().content("Invalid training target.");
                    component.edit_response(&ctx.http, builder).await.ok();
                    return;
                }
            };

            // Call the database function to start the training session.
            let success = database::units::start_training(
                &db,
                component.user.id,
                player_unit_id,
                stat_to_train,
                2, // Duration in hours
                1, // TP cost
            )
            .await
            .unwrap_or(false);

            // Respond with a confirmation or error message.
            let mut builder = EditInteractionResponse::new().components(vec![]);
            if success {
                // Invalidate caches so future bonus calculations reflect upcoming stat change once complete.
                app_state.invalidate_user_caches(component.user.id).await;
                builder = builder.content(format!("Training has begun! Your unit will gain +1 {} in 2 hours.", stat_to_train));
            } else {
                builder = builder.content("Failed to start training. You may not have enough Training Points, or the unit does not belong to you.");
            }
            component.edit_response(&ctx.http, builder).await.ok();
        }
        _ => {}
    }
}
