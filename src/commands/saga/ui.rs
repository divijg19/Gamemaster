//! Handles the UI creation for the `/saga` command menu.

use crate::database::models::{MapNode, SagaProfile};
use crate::interactions::ids::*;
use crate::ui::buttons::Btn;
use crate::ui::style::{
    COLOR_SAGA_MAIN, COLOR_SAGA_MAP, COLOR_SAGA_TUTORIAL, EMOJI_AP, EMOJI_BACK, EMOJI_REFRESH,
    EMOJI_TP, stat_pair,
};
use chrono::{Duration, Utc};
use serenity::builder::{CreateActionRow, CreateButton, CreateEmbed};
use serenity::model::application::ButtonStyle;

/// Creates the embed and components for the main saga menu.
pub fn create_saga_menu(
    saga_profile: &SagaProfile,
    has_party: bool,
) -> (CreateEmbed, Vec<CreateActionRow>) {
    let mut desc = String::from("Your daily adventure awaits. Choose your action wisely.");
    if !has_party {
        desc.push_str("\n\nYou don't have a party yet. Recruit units in the Tavern first.");
    } else if saga_profile.current_ap == 0 {
        // Provide a rough ETA assuming full daily reset at UTC day boundary for AP (since AP model currently resets daily)
        let now = Utc::now();
        let midnight = (now + Duration::days(1))
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let secs = (midnight - now.naive_utc()).num_seconds();
        if secs > 0 {
            let hrs = secs / 3600;
            let mins = (secs % 3600) / 60;
            desc.push_str(&format!(
                "\n\nYou are out of Action Points. Daily reset in ~{}h {}m.",
                hrs, mins
            ));
        } else {
            desc.push_str("\n\nYou are out of Action Points. They replenish on daily reset.");
        }
    }
    let embed = CreateEmbed::new()
        .title("The Gamemaster Saga")
        .description(desc)
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
        .color(COLOR_SAGA_MAIN)
        .footer(serenity::builder::CreateEmbedFooter::new(
            "Use Refresh to update AP/TP • Manage Party before exploring",
        ));

    // Primary action row
    let mut components: Vec<CreateActionRow> = Vec::new();
    // Unified width target for primary saga buttons
    // Use global BTN_W_PRIMARY width via helper
    let mut primary_buttons = Vec::new();
    if has_party {
        primary_buttons
            .push(Btn::primary(SAGA_MAP, "🗺 Map (1 AP)").disabled(saga_profile.current_ap < 1));
        primary_buttons.push(Btn::success(SAGA_TAVERN, "🍺 Tavern"));
    } else {
        primary_buttons.push(Btn::secondary(SAGA_MAP_LOCKED, "🗺 Map (Need Party)").disabled(true));
        primary_buttons.push(Btn::success(SAGA_RECRUIT, "➕ Recruit"));
    }
    primary_buttons.push(Btn::secondary(SAGA_TEAM, "👥 Party"));
    components.push(CreateActionRow::Buttons(primary_buttons));

    // Navigation / utility row: Back (disabled at root) + Refresh. Removed redundant Play Alias button.
    components.push(CreateActionRow::Buttons(vec![
        Btn::danger(SAGA_BACK, &format!("{} Back", EMOJI_BACK)).disabled(true),
        Btn::secondary(SAGA_REFRESH, &format!("{} Refresh", EMOJI_REFRESH)),
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
        .color(COLOR_SAGA_MAP)
        .footer(serenity::builder::CreateEmbedFooter::new(
            "Spend AP to battle nodes • Back + Refresh available",
        )); // Styled constant

    if nodes.is_empty() {
        embed = embed.description("There are no available locations for you to explore right now. Come back after you've made more progress in the story!");
        return (embed, vec![]);
    }

    // Build descriptive labels including area id & required story progress (activates previously unused fields)
    // Also surface node description + rewards (coins/xp) in a single consolidated field.

    // Compose a compact locations listing; truncate to avoid exceeding field limits (<= 1024 chars).
    let mut lines = Vec::new();
    let mut total_len = 0usize;
    for node in nodes {
        let desc_snip = node
            .description
            .as_deref()
            .map(|d| {
                if d.len() > 40 {
                    format!("{}…", &d[..40])
                } else {
                    d.to_string()
                }
            })
            .unwrap_or_else(|| "No details".to_string());
        let rewards_part = if node.reward_coins > 0 || node.reward_unit_xp > 0 {
            format!(" (💰{} / XP {})", node.reward_coins, node.reward_unit_xp)
        } else {
            String::new()
        };
        let line = format!(
            "[#{:02}] {} (SP {}){} - {}",
            node.node_id, node.name, node.story_progress_required, rewards_part, desc_snip
        );
        // Stop if adding would exceed ~950 chars (leave headroom for formatting).
        if total_len + line.len() > 950 {
            break;
        }
        total_len += line.len();
        lines.push(line);
    }
    if lines.len() < nodes.len() {
        lines.push("… more locations available".to_string());
    }
    if !lines.is_empty() {
        embed = embed.field("Locations", lines.join("\n"), false);
    }

    let mut components: Vec<CreateActionRow> = Vec::new();
    // Build rows with main node buttons and a paired preview row beneath each.
    components.clear();
    for chunk_nodes in nodes.chunks(5) {
        let main: Vec<CreateButton> = chunk_nodes.iter().map(map_node_button).collect();
        components.push(CreateActionRow::Buttons(main));
        let preview: Vec<CreateButton> = chunk_nodes
            .iter()
            .map(|n| {
                CreateButton::new(format!("saga_preview_{}", n.node_id))
                    .label(format!(
                        "👁 {}",
                        &n.name.chars().take(10).collect::<String>()
                    ))
                    .style(ButtonStyle::Secondary)
            })
            .collect();
        components.push(CreateActionRow::Buttons(preview));
    }
    // Append global nav row; back/refresh row injected by interaction handler based on stack depth.
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
    .color(COLOR_SAGA_TUTORIAL)
    .footer(serenity::builder::CreateEmbedFooter::new("Choose Get Starter Unit to begin your adventure")); // Styled constant

    let row = CreateActionRow::Buttons(vec![
        Btn::success(SAGA_TUTORIAL_HIRE, "➕ Starter Unit"),
        Btn::secondary(SAGA_TUTORIAL_SKIP, "⏭ Skip Tutorial"),
    ]);
    // Remove legacy play/menu button; global nav row already provides Saga entry.
    let mut v = vec![row];
    v.push(global_nav_row("saga"));
    (embed, v)
}

// --- helpers ---
fn map_node_button(node: &MapNode) -> CreateButton {
    let mut label = format!(
        "[A{}|SP{}] {}",
        node.area_id, node.story_progress_required, node.name
    );
    label.truncate(20);
    CreateButton::new(format!("{}{}", SAGA_NODE_PREFIX, node.node_id))
        .label(label)
        .style(ButtonStyle::Secondary)
        .emoji('🗺')
        .disabled(false)
}

/// Builds a Back + Refresh control row when depth > 1 (navigation inside a stack).
pub fn back_refresh_row(depth: usize) -> Option<CreateActionRow> {
    if depth > 1 {
        Some(CreateActionRow::Buttons(vec![
            Btn::danger(SAGA_BACK, &format!("{} Back", EMOJI_BACK)),
            Btn::secondary(SAGA_REFRESH, &format!("{} Refresh", EMOJI_REFRESH)),
        ]))
    } else {
        None
    }
}

/// Universal navigation row allowing quick jumps between core game menus.
/// To be appended by other command UIs (party, train, etc.).
pub fn global_nav_row(active: &'static str) -> CreateActionRow {
    // helper closure for consistency
    let mut saga_btn = Btn::primary(NAV_SAGA, "Saga");
    if active == "saga" {
        saga_btn = saga_btn.disabled(true);
    }
    let mut party_btn = Btn::secondary(NAV_PARTY, "Party");
    if active == "party" {
        party_btn = party_btn.disabled(true);
    }
    let mut train_btn = Btn::secondary(NAV_TRAIN, "Train");
    if active == "train" {
        train_btn = train_btn.disabled(true);
    }
    CreateActionRow::Buttons(vec![saga_btn, party_btn, train_btn])
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
