//! Handles the UI creation for the `/saga` command menu.

use crate::database::models::{MapNode, SagaProfile};
use crate::ui::style::*;
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
            format!("{} Action Points", EMOJI_AP),
            stat_pair(saga_profile.current_ap, saga_profile.max_ap),
            true,
        )
        .field(
            format!("{} Training Points", EMOJI_TP),
            stat_pair(saga_profile.current_tp, saga_profile.max_tp),
            true,
        )
        .color(COLOR_SAGA_MAIN);

    // Primary action row
    let mut components: Vec<CreateActionRow> = Vec::new();
    components.push(CreateActionRow::Buttons(vec![
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
    ]));

    // Navigation / utility row: Back (disabled at root) + Refresh.
    components.push(CreateActionRow::Buttons(vec![
        CreateButton::new("saga_back")
            .label(format!("{} Back", EMOJI_BACK))
            .style(ButtonStyle::Danger)
            .disabled(true), // root menu has no back target
        CreateButton::new("saga_refresh")
            .label(format!("{} Refresh", EMOJI_REFRESH))
            .style(ButtonStyle::Secondary),
        CreateButton::new("saga_play")
            .label("Play Alias")
            .style(ButtonStyle::Secondary),
    ]));

    // Append global nav row (active = saga) at end.
    components.push(global_nav_row("saga"));
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
        .color(COLOR_SAGA_MAP); // Styled constant

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

    // Append global nav row for consistency
    components.push(global_nav_row("saga"));
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
    .color(COLOR_SAGA_TUTORIAL); // Styled constant

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

    let mut v = vec![row, play_row];
    v.push(global_nav_row("saga"));
    (embed, v)
}

/// Builds a Back + Refresh control row when depth > 1 (navigation inside a stack).
pub fn back_refresh_row(depth: usize) -> Option<CreateActionRow> {
    if depth > 1 {
        Some(CreateActionRow::Buttons(vec![
            CreateButton::new("saga_back")
                .label(format!("{} Back", EMOJI_BACK))
                .style(serenity::model::application::ButtonStyle::Danger),
            CreateButton::new("saga_refresh")
                .label(format!("{} Refresh", EMOJI_REFRESH))
                .style(serenity::model::application::ButtonStyle::Secondary),
        ]))
    } else {
        None
    }
}

/// Universal navigation row allowing quick jumps between core game menus.
/// To be appended by other command UIs (party, train, etc.).
pub fn global_nav_row(active: &'static str) -> CreateActionRow {
    // helper closure for consistency
    let mk = |id: &str, label: &str, style: ButtonStyle, on: bool| {
        let mut b = CreateButton::new(id).label(label).style(style);
        if on {
            b = b.disabled(true);
        }
        b
    };
    CreateActionRow::Buttons(vec![
        mk("nav_saga", "Saga", ButtonStyle::Primary, active == "saga"),
        mk(
            "nav_party",
            "Party",
            ButtonStyle::Secondary,
            active == "party",
        ),
        mk(
            "nav_train",
            "Train",
            ButtonStyle::Secondary,
            active == "train",
        ),
    ])
}

/// Convenience helper to append the global nav row if not already present.
pub fn add_nav(components: &mut Vec<CreateActionRow>, active: &'static str) {
    // Simple check: look for any button row whose first button custom_id starts with "nav_saga".
    let has_nav = components.iter().any(|row| {
        // Serenity doesn't expose direct introspection of buttons here without matching variants; rely on Debug fallback.
        // Fallback heuristic: format row and look for "nav_saga" substring (cheap & fine for low frequency calls).
        format!("{:?}", row).contains("nav_saga")
    });
    if !has_nav {
        components.push(global_nav_row(active));
    }
}
