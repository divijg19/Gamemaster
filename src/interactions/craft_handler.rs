//! Handles all component interactions for the `craft` command family.

use crate::{AppState, database};
use serenity::builder::EditInteractionResponse;
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;
use std::sync::Arc;

pub async fn handle(ctx: &Context, component: &mut ComponentInteraction, app_state: Arc<AppState>) {
    let db = &app_state.db;
    component.defer_ephemeral(&ctx.http).await.ok();

    // Get the recipe ID that the user selected from the dropdown.
    let recipe_id_str =
        if let serenity::model::application::ComponentInteractionDataKind::StringSelect { values } =
            &component.data.kind
        {
            &values[0]
        } else {
            return;
        };

    // Safely parse the recipe ID.
    let recipe_id: i32 = match recipe_id_str.parse() {
        Ok(id) => id,
        Err(_) => {
            let builder =
                EditInteractionResponse::new().content("❌ Error: Invalid recipe ID selected.");
            component.edit_response(&ctx.http, builder).await.ok();
            return;
        }
    };

    // Call the database function to attempt the craft.
    let result = database::crafting::craft_item(db, component.user.id, recipe_id).await;

    let mut builder = EditInteractionResponse::new().components(vec![]); // Clear components after crafting.

    match result {
        Ok(item) => {
            // (✓) FIXED: Use the correct function signature with a formatted objective key.
            database::tasks::update_task_progress(
                db,
                component.user.id,
                &format!("CraftItem:{}", item.id()), // e.g., "CraftItem:11"
                1,
            )
            .await
            .ok();

            builder = builder.content(format!(
                "✅ You successfully crafted **1x {}**!",
                item.display_name()
            ));
        }
        Err(e) => {
            builder = builder.content(format!("❌ Crafting failed: {}", e));
        }
    }

    component.edit_response(&ctx.http, builder).await.ok();
}
