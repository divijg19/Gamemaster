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
            "Use Refresh to update AP/TP • Manage your Party via the nav row",
        ));

    // Primary action row
    let mut components: Vec<CreateActionRow> = Vec::new();
    // Unified width target for primary saga buttons
    // Use global BTN_W_PRIMARY width via helper
    let mut primary_buttons = Vec::new();
    if has_party {
        let map_label = if saga_profile.current_ap < 1 {
            "🗺 Map (No AP)"
        } else {
            "🗺 Map (1 AP)"
        };
        primary_buttons
            .push(Btn::primary(SAGA_MAP, map_label).disabled(saga_profile.current_ap < 1));
        primary_buttons.push(Btn::success(SAGA_TAVERN, "🍺 Tavern"));
    } else {
        primary_buttons.push(Btn::secondary(SAGA_MAP_LOCKED, "🗺 Map (Need Party)").disabled(true));
        primary_buttons.push(Btn::success(SAGA_RECRUIT, "➕ Recruit"));
    }
    // Removed duplicate Party button (accessible via global nav row) to reduce clutter.
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
    // Separate unlocked vs locked (based on story progress requirement)
    let (unlocked, locked): (Vec<&MapNode>, Vec<&MapNode>) = nodes
        .iter()
        .partition(|n| n.story_progress_required <= saga_profile.story_progress);

    let mut embed = CreateEmbed::new()
        .title("World Map")
        .description(format!(
            "Story Progress: **{}** | Unlocked Nodes: **{}** | Locked: **{}**\nSelect a destination to spend 1 AP and engage the encounter.",
            saga_profile.story_progress,
            unlocked.len(),
            locked.len()
        ))
        .field(
            "⚔️ Action Points",
            format!("`{}/{}`", saga_profile.current_ap, saga_profile.max_ap),
            true,
        )
    .field("Legend", "E Easy • = Even • M Moderate • H Hard", true)
        .color(COLOR_SAGA_MAP)
        .footer(serenity::builder::CreateEmbedFooter::new(
            "Use Back to return • Refresh to update AP/TP • Locked nodes show their required SP",
        ));

    if unlocked.is_empty() {
        embed = embed.field(
            "No Battles Available",
            "Advance the story by completing currently available encounters to unlock more nodes.",
            false,
        );
    } else {
        use std::collections::BTreeMap;
        let mut by_area: BTreeMap<i32, Vec<&MapNode>> = BTreeMap::new();
        for n in &unlocked {
            by_area.entry(n.area_id).or_default().push(*n);
        }
        let mut area_fields_added = 0usize;
        for (area, list) in by_area.iter() {
            if area_fields_added >= 8 {
                break;
            } // cap to avoid too many embed fields
            let mut lines = Vec::new();
            for node in list.iter().take(6) {
                // cap nodes per area field
                let diff =
                    difficulty_tag(node.story_progress_required, saga_profile.story_progress);
                let diff_symbol = match diff {
                    "EASY" => "E",
                    "EVEN" => "=",
                    "MOD" => "M",
                    "HARD" => "H",
                    _ => "?",
                };
                let rewards_part = if node.reward_coins > 0 || node.reward_unit_xp > 0 {
                    format!(" (💰{} / XP {})", node.reward_coins, node.reward_unit_xp)
                } else {
                    String::new()
                };
                let desc_snip = truncate(node.description.as_deref().unwrap_or("No details"), 40);
                lines.push(format!(
                    "{} #{:02} **{}**{} – {}",
                    diff_symbol, node.node_id, node.name, rewards_part, desc_snip
                ));
            }
            if list.len() > 6 {
                lines.push("…".into());
            }
            embed = embed.field(
                format!("Area A{} ({} nodes)", area, list.len()),
                lines.join("\n"),
                false,
            );
            area_fields_added += 1;
        }
        if by_area.len() > area_fields_added {
            // collapse remaining areas into summary
            let remaining = by_area.len() - area_fields_added;
            embed = embed.field(
                "More Areas",
                format!("{} additional area groups hidden for brevity", remaining),
                false,
            );
        }
    }

    if !locked.is_empty() {
        let mut lines = Vec::new();
        for node in &locked {
            lines.push(format!(
                "`SP {:02}` {} (#{})",
                node.story_progress_required, node.name, node.node_id
            ));
            if lines.len() >= 6 {
                break;
            }
        }
        if locked.len() > lines.len() {
            lines.push("… more locked".into());
        }
        embed = embed.field("Locked", lines.join("\n"), false);
    }

    // Build button rows: unlocked nodes are clickable; locked shown disabled (first few only)
    let mut components: Vec<CreateActionRow> = Vec::new();
    let can_start = saga_profile.current_ap > 0;
    // Cap unlocked rows to avoid exceeding Discord's 5-row limit (leave room for locked + nav).
    for (unlocked_rows, chunk) in unlocked.chunks(5).enumerate() {
        if unlocked_rows >= 3 {
            break;
        }
        let row = CreateActionRow::Buttons(
            chunk
                .iter()
                .map(|n| map_node_button(n, saga_profile.story_progress, can_start))
                .collect(),
        );
        components.push(row);
    }
    // Show up to one row of locked nodes (disabled) for foreshadowing
    if !locked.is_empty() && unlocked.chunks(5).take(3).count() <= 3 {
        let mut locked_buttons = Vec::new();
        for node in locked.iter().take(5) {
            let mut label = format!("SP{} {}", node.story_progress_required, node.name);
            label.truncate(20);
            locked_buttons.push(
                CreateButton::new("locked_node")
                    .label(label)
                    .style(ButtonStyle::Secondary)
                    .disabled(true)
                    .emoji('🔒'),
            );
        }
        components.push(CreateActionRow::Buttons(locked_buttons));
    }
    components.push(global_nav_row("saga"));
    (embed, components)
}

/// Render a single-area focused map view showing only nodes for that area (unlocked + locked) with an area nav bar.
pub fn create_world_map_area_view(
    all_nodes: &[MapNode],
    saga_profile: &SagaProfile,
    area_id: i32,
) -> (CreateEmbed, Vec<CreateActionRow>) {
    let current_area_nodes: Vec<&MapNode> =
        all_nodes.iter().filter(|n| n.area_id == area_id).collect();
    let (unlocked, locked): (Vec<&MapNode>, Vec<&MapNode>) = current_area_nodes
        .into_iter()
        .partition(|n| n.story_progress_required <= saga_profile.story_progress);
    let mut embed = CreateEmbed::new()
        .title(format!("Area A{}", area_id))
        .description(format!(
            "Story Progress **{}** • Unlocked **{}** • Locked **{}**\nUse area buttons below to switch regions.",
            saga_profile.story_progress,
            unlocked.len(),
            locked.len()
        ))
        .field(
            "AP",
            format!("`{}/{}`", saga_profile.current_ap, saga_profile.max_ap),
            true,
        )
    .field("Legend", "E Easy • = Even • M Moderate • H Hard", true)
        .color(COLOR_SAGA_MAP)
        .footer(serenity::builder::CreateEmbedFooter::new(
            "Switch areas or pick a node to battle.",
        ));
    if unlocked.is_empty() {
        embed = embed.field(
            "No Unlocked Nodes",
            "Progress the story to unlock nodes in this area.",
            false,
        );
    } else {
        let mut lines = Vec::new();
        for n in &unlocked {
            let diff = difficulty_tag(n.story_progress_required, saga_profile.story_progress);
            let diff_symbol = match diff {
                "EASY" => "E",
                "EVEN" => "=",
                "MOD" => "M",
                "HARD" => "H",
                _ => "?",
            };
            let rewards_part = if n.reward_coins > 0 || n.reward_unit_xp > 0 {
                format!(" (💰{} / XP {})", n.reward_coins, n.reward_unit_xp)
            } else {
                String::new()
            };
            lines.push(format!(
                "{} #{:02} **{}**{}",
                diff_symbol, n.node_id, n.name, rewards_part
            ));
            if lines.len() >= 12 {
                break;
            }
        }
        embed = embed.field("Unlocked", lines.join("\n"), false);
    }
    if !locked.is_empty() {
        let mut lines = Vec::new();
        for n in &locked {
            lines.push(format!("SP{} {}", n.story_progress_required, n.name));
            if lines.len() >= 6 {
                break;
            }
        }
        embed = embed.field("Locked", lines.join("\n"), false);
    }
    // Build components: area nav row + node rows + global nav
    use std::collections::BTreeSet;
    let mut area_ids: BTreeSet<i32> = BTreeSet::new();
    for n in all_nodes {
        area_ids.insert(n.area_id);
    }
    let mut area_buttons = Vec::new();
    for id in area_ids.iter().take(5) {
        // cap nav buttons
        let mut label = format!("A{}", id);
        if *id == area_id {
            label.push('*');
        }
        let mut btn = Btn::secondary(
            &format!("{}{}", crate::interactions::ids::SAGA_AREA_PREFIX, id),
            &label,
        );
        if *id == area_id {
            btn = btn.disabled(true);
        }
        area_buttons.push(btn);
    }
    let mut components: Vec<CreateActionRow> = Vec::new();
    components.push(CreateActionRow::Buttons(area_buttons));
    let can_start = saga_profile.current_ap > 0;
    // In area view we already used 1 row for area nav; keep total <= 5.
    let locked_present = !locked.is_empty();
    // If we plan to show a locked row, allow up to 2 unlocked rows; else up to 3.
    let max_unlocked_rows = if locked_present { 2 } else { 3 };
    for (unlocked_rows, chunk) in unlocked.chunks(5).enumerate() {
        if unlocked_rows >= max_unlocked_rows {
            break;
        }
        components.push(CreateActionRow::Buttons(
            chunk
                .iter()
                .map(|n| map_node_button(n, saga_profile.story_progress, can_start))
                .collect(),
        ));
    }
    if locked_present {
        let mut locked_buttons = Vec::new();
        for node in locked.iter().take(5) {
            let mut label = format!("SP{} {}", node.story_progress_required, node.name);
            label.truncate(20);
            locked_buttons.push(
                CreateButton::new("locked_node")
                    .label(label)
                    .style(ButtonStyle::Secondary)
                    .disabled(true)
                    .emoji('🔒'),
            );
        }
        components.push(CreateActionRow::Buttons(locked_buttons));
    }
    components.push(global_nav_row("saga"));
    (embed, components)
}

/// Rough difficulty heuristic compared to player's current story progress.
fn difficulty_tag(req: i32, current: i32) -> &'static str {
    if req > current + 2 {
        "HARD"
    } else if req > current {
        "MOD"
    } else if req + 2 < current {
        "EASY"
    } else {
        "EVEN"
    }
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
fn map_node_button(node: &MapNode, player_sp: i32, can_start: bool) -> CreateButton {
    let mut base = format!("{} •1AP", node.name);
    base.truncate(20);
    let diff = difficulty_tag(node.story_progress_required, player_sp);
    let style = match diff {
        "HARD" => ButtonStyle::Danger,
        "MOD" => ButtonStyle::Primary,
        "EVEN" => ButtonStyle::Success,
        "EASY" => ButtonStyle::Secondary,
        _ => ButtonStyle::Secondary,
    };
    CreateButton::new(format!("{}{}", SAGA_NODE_PREFIX, node.node_id))
        .label(base)
        .style(style)
        .emoji('🗺')
        .disabled(!can_start)
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

// Small helper to truncate text with ellipsis.
fn truncate(s: &str, max: usize) -> String {
    // Truncate by characters (not bytes) to avoid slicing on a UTF-8 boundary.
    let mut chars = s.chars();
    let mut out = String::new();
    for _ in 0..max {
        if let Some(c) = chars.next() {
            out.push(c);
        } else {
            break;
        }
    }
    if out.len() < s.len() {
        out.push('…');
    }
    out
}
