//! Handles all component interactions for the `party` command family.

use crate::{AppState, commands, database};
use serenity::builder::EditInteractionResponse;
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;
use std::sync::Arc;

pub async fn handle(ctx: &Context, component: &mut ComponentInteraction, app_state: Arc<AppState>) {
    let db = app_state.db.clone();
    component.defer_ephemeral(&ctx.http).await.ok();

    // Determine if we are adding or removing a pet.
    let action = component.data.custom_id.split('_').nth(1).unwrap_or("");
    let is_adding = action == "add";

    // Get the selected pet's ID from the dropdown menu.
    let pet_id_str =
        if let serenity::model::application::ComponentInteractionDataKind::StringSelect { values } =
            &component.data.kind
        {
            &values[0]
        } else {
            return;
        };
    let pet_id = pet_id_str.parse::<i32>().unwrap();

    // (✓) FIXED: Updated to the new database module path.
    let result = database::set_pet_party_status(&db, component.user.id, pet_id, is_adding).await;

    // After any change, always re-fetch the full list of pets and re-render the UI.
    // (✓) FIXED: Updated to the new database module path.
    let pets = database::get_player_pets(&db, component.user.id)
        .await
        .unwrap_or_default();
    let (embed, components) = commands::party::ui::create_party_view(&pets);

    let mut builder = EditInteractionResponse::new()
        .embed(embed)
        .components(components);

    // If the database function reported a failure (e.g., party was full), add a small error message.
    if let Ok(false) = result
        && is_adding
    {
        builder = builder.content("Could not add pet: Your party is full (5/5).");
    }

    component.edit_response(&ctx.http, builder).await.ok();
}
