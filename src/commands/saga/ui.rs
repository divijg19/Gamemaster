//! Handles the UI creation for the `/saga` command menu.

use crate::database::models::{MapNode, SagaProfile};
use serenity::builder::{CreateActionRow, CreateButton, CreateEmbed};
use serenity::model::application::ButtonStyle;

/// Reusable first-row Play button to ensure consistent navigation back to main Saga menu.
pub fn play_button_row(label: &str) -> CreateActionRow {
    CreateActionRow::Buttons(vec![
        CreateButton::new("saga_play")
            .label(label)
            .style(ButtonStyle::Primary),
    ])
}
// End of play_button_row function
/// Creates the embed and components for the main saga menu.
pub fn create_saga_menu(
    saga_profile: &SagaProfile,
    has_party: bool,
) -> (CreateEmbed, Vec<CreateActionRow>) {
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

    let components = vec![
        CreateActionRow::Buttons(vec![
            CreateButton::new("saga_map")
                .label("World Map (1 AP)")
                .style(ButtonStyle::Primary)
                .disabled(saga_profile.current_ap < 1 || !has_party),
            CreateButton::new("saga_tavern")
                .label("Tavern")
                .style(ButtonStyle::Success),
            CreateButton::new("saga_team")
                .label("Manage Party")
                .style(ButtonStyle::Secondary),
        ]),
        // Secondary row with a dedicated Play alias button for consistency across entry points.
        CreateActionRow::Buttons(vec![
            CreateButton::new("saga_play")
                .label("Play / Refresh")
                .style(ButtonStyle::Secondary),
        ]),
    ];

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

    // Build descriptive labels including area id & required story progress (activates previously unused fields)
    let buttons: Vec<_> = nodes
        .iter()
        .map(|node| {
            let mut label = format!(
                "[A{}|SP{}] {}",
                node.area_id, node.story_progress_required, node.name
            );
            label.truncate(20);
            let _desc_snippet = node
                .description
                .as_ref()
                .map(|d| d.chars().take(25).collect::<String>())
                .unwrap_or_else(|| "No description".into());
            CreateButton::new(format!("saga_node_{}", node.node_id))
                .label(label)
                .style(ButtonStyle::Secondary)
                .emoji('🗺')
                .custom_id(format!("saga_node_{}", node.node_id))
                .disabled(false)
        })
        .collect();

    let mut components = vec![play_button_row("Play / Menu")];
    for chunk in buttons.chunks(5) {
        components.push(CreateActionRow::Buttons(chunk.to_vec()));
    }

    // Add a final action row with a Back button to return to the main saga menu.
    components.push(CreateActionRow::Buttons(vec![
        CreateButton::new("saga_main")
            .label("⬅ Back")
            .style(ButtonStyle::Danger),
    ]));

    (embed, components)
}

/// Creates the first-time player tutorial view.
pub fn create_first_time_tutorial() -> (CreateEmbed, Vec<CreateActionRow>) {
    let embed = CreateEmbed::new()
        .title("Welcome to the Gamemaster Saga!")
        .description("It looks like this is your first time adventuring.\n\nStart by recruiting a starter unit so you can form a party and explore the world map. You can always recruit more from the Tavern later.\n\nReady to begin?")
        .field("Step 1","Recruit a starter unit (free).", false)
        .field("Step 2","Use 'Manage Party' later to adjust your lineup.", false)
        .field("Step 3","Spend Action Points on the World Map to battle and earn rewards.", false)
        .color(0x3498DB); // Blue

    let row = CreateActionRow::Buttons(vec![
        CreateButton::new("saga_tutorial_hire")
            .label("Get Starter Unit")
            .style(ButtonStyle::Success),
        CreateButton::new("saga_tutorial_skip")
            .label("Skip Tutorial")
            .style(ButtonStyle::Secondary),
    ]);
    // Add a play button so user can always refresh to the main menu easily.
    let play_row = CreateActionRow::Buttons(vec![
        CreateButton::new("saga_play")
            .label("Open Main Menu")
            .style(ButtonStyle::Primary),
    ]);

    (embed, vec![row, play_row])
}
