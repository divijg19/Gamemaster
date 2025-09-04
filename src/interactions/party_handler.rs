//! Handles all component interactions for the `party` command family.

use crate::{AppState, commands, database};
use serenity::builder::EditInteractionResponse;
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;
use std::sync::Arc;

pub async fn handle(ctx: &Context, component: &mut ComponentInteraction, app_state: Arc<AppState>) {
    let db = app_state.db.clone();
    component.defer_ephemeral(ctx.http.clone()).await.ok();

    let action = component.data.custom_id.split('_').nth(1).unwrap_or("");

    let unit_id_str =
        if let serenity::model::application::ComponentInteractionDataKind::StringSelect { values } =
            &component.data.kind
        {
            &values[0]
        } else {
            return;
        };
    let unit_id = match unit_id_str.parse::<i32>() {
        Ok(id) => id,
        Err(_) => {
            let builder = EditInteractionResponse::new().content("Invalid unit selected.");
            component
                .edit_response(ctx.http.clone(), builder)
                .await
                .ok();
            return;
        }
    };

    let mut confirmation_message = "".to_string();

    // (✓) MODIFIED: The logic is now in a match block to handle all party actions.
    match action {
        "add" => {
            let result =
                database::units::set_unit_party_status(&db, component.user.id, unit_id, true).await;
            if let Ok(false) = result {
                confirmation_message = format!(
                    "Could not add unit: Party full ({}/{}) or pet rarity below Legendary.",
                    crate::constants::MAX_PARTY_SIZE,
                    crate::constants::MAX_PARTY_SIZE
                );
            }
        }
        "remove" => {
            database::units::set_unit_party_status(&db, component.user.id, unit_id, false)
                .await
                .ok();
        }
        // (✓) ADDED: Handler for the dismiss action.
        "dismiss" => {
            let success = database::units::dismiss_unit(&db, component.user.id, unit_id)
                .await
                .unwrap_or(false);
            if success {
                confirmation_message = "The unit has been dismissed from your army.".to_string();
            } else {
                confirmation_message = "Failed to dismiss the unit.".to_string();
            }
        }
        _ => {}
    }

    // After any action, always re-fetch the unit list and re-render the UI.
    let (embed, components) =
        commands::party::ui::create_party_view_with_bonds(&app_state, component.user.id).await;

    let builder = EditInteractionResponse::new()
        .embed(embed)
        .components(components)
        .content(confirmation_message); // Display the confirmation message.

    component
        .edit_response(ctx.http.clone(), builder)
        .await
        .ok();
}
