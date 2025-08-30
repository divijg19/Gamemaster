//! Handles the UI creation for the `/saga` command menu.

use crate::database::profile::{MapNode, SagaProfile};
use serenity::builder::{CreateActionRow, CreateButton, CreateEmbed};
use serenity::model::application::ButtonStyle;

/// Creates the embed and components for the main saga menu.
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
            .disabled(saga_profile.current_ap < 1),
        CreateButton::new("saga_tavern")
            .label("Tavern")
            .style(ButtonStyle::Success),
        CreateButton::new("saga_team")
            .label("Manage Party")
            .style(ButtonStyle::Secondary),
    ])];

    (embed, components)
}

/// Creates the embed and components for the World Map view.
pub fn create_world_map_view(
    nodes: &[MapNode],
    saga_profile: &SagaProfile,
) -> (CreateEmbed, Vec<CreateActionRow>) {
    let mut embed = CreateEmbed::new()
        .title("Whispering Woods")
        .description("You look over the map, deciding where to go next.")
        .field(
            "⚔️ Action Points",
            format!("`{}/{}`", saga_profile.current_ap, saga_profile.max_ap),
            false,
        )
        .color(0x2ECC71); // Green

    if nodes.is_empty() {
        embed = embed.description("There are no available locations for you to explore right now. Come back after you've made more progress in the story!");
        return (embed, vec![]);
    }

    let buttons: Vec<_> = nodes
        .iter()
        .map(|node| {
            CreateButton::new(format!("saga_node_{}", node.node_id))
                .label(node.name.clone())
                .style(ButtonStyle::Secondary)
        })
        .collect();

    let mut components = Vec::new();
    for chunk in buttons.chunks(5) {
        components.push(CreateActionRow::Buttons(chunk.to_vec()));
    }

    (embed, components)
}
