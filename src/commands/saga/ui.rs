//! Handles the UI creation for the `/saga` command menu.

use crate::database::profile::SagaProfile;
use serenity::builder::{CreateActionRow, CreateButton, CreateEmbed};
use serenity::model::application::ButtonStyle;

pub fn create_saga_menu(saga_profile: &SagaProfile) -> (CreateEmbed, Vec<CreateActionRow>) {
    let embed = CreateEmbed::new()
        .title("The Gamemaster Saga")
        .description("Your daily adventure awaits. Choose your action wisely.")
        .field(
            "⚔️ Action Points",
            format!("`{}/{}`", saga_profile.current_ap, saga_profile.max_ap),
            true,
        )
        .field(
            "⚡ Training Points",
            format!("`{}/{}`", saga_profile.current_tp, saga_profile.max_tp),
            true,
        )
        .color(0x9B59B6); // Purple

    let components = vec![CreateActionRow::Buttons(vec![
        CreateButton::new("saga_map")
            .label("World Map (1 AP)")
            .style(ButtonStyle::Primary)
            .disabled(saga_profile.current_ap < 1), // Disable if they can't afford it
        CreateButton::new("saga_tavern")
            .label("Tavern")
            .style(ButtonStyle::Success),
        // (✓) MODIFIED: The button is now enabled, renamed for clarity, and fully functional.
        CreateButton::new("saga_team")
            .label("Manage Party")
            .style(ButtonStyle::Secondary),
    ])];

    (embed, components)
}
