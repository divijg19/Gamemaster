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

    let pet_id_str =
        if let serenity::model::application::ComponentInteractionDataKind::StringSelect { values } =
            &component.data.kind
        {
            &values[0]
        } else {
            return;
        };
    let pet_id = pet_id_str.parse::<i32>().unwrap();

    let mut confirmation_message = "".to_string();

    // (✓) MODIFIED: The logic is now in a match block to handle all party actions.
    match action {
        "add" => {
            let result =
                database::pets::set_pet_party_status(&db, component.user.id, pet_id, true).await;
            if let Ok(false) = result {
                confirmation_message = "Could not add pet: Your party is full (5/5).".to_string();
            }
        }
        "remove" => {
            database::pets::set_pet_party_status(&db, component.user.id, pet_id, false)
                .await
                .ok();
        }
        // (✓) ADDED: Handler for the dismiss action.
        "dismiss" => {
            let success = database::pets::dismiss_pet(&db, component.user.id, pet_id)
                .await
                .unwrap_or(false);
            if success {
                confirmation_message = "The pet has been dismissed from your army.".to_string();
            } else {
                confirmation_message = "Failed to dismiss the pet.".to_string();
            }
        }
        _ => {}
    }

    // After any action, always re-fetch the pet list and re-render the UI.
    let pets = database::pets::get_player_pets(&db, component.user.id)
        .await
        .unwrap_or_default();
    let (embed, components) = commands::party::ui::create_party_view(&pets);

    let builder = EditInteractionResponse::new()
        .embed(embed)
        .components(components)
        .content(confirmation_message); // Display the confirmation message.

    component
        .edit_response(ctx.http.clone(), builder)
        .await
        .ok();
}
