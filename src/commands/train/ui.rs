//! Handles the UI creation for the `/train` command.

use crate::database::models::{PlayerUnit, SagaProfile};
use crate::ui::style::pad_label;
use serenity::builder::{
    CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter, CreateSelectMenu,
    CreateSelectMenuKind, CreateSelectMenuOption,
};
use serenity::model::application::ButtonStyle;

/// Creates the main training menu, showing a list of units.
pub fn create_training_menu(
    units: &[PlayerUnit],
    saga_profile: &SagaProfile,
) -> (CreateEmbed, Vec<CreateActionRow>) {
    let mut embed = CreateEmbed::new()
    .title("Unit Training Grounds")
    .description("Select a unit from the dropdown menu to begin training. Training costs **1 TP** and takes **2 hours**.")
        .field("⚡ Your Training Points", format!("`{}/{}`", saga_profile.current_tp, saga_profile.max_tp), false)
        .color(0xDAA520); // Goldenrod

    if units.is_empty() {
        embed = embed.description(
            "You don't have any units to train yet! Visit the Tavern in the `/saga` menu to recruit some.",
        );
        return (embed, vec![]);
    }

    let mut select_menu_options = Vec::new();
    let mut unit_list_lines = Vec::new();

    for unit in units {
        let unit_name = unit.nickname.as_deref().unwrap_or(&unit.name);
        if unit.is_training {
            if let Some(ends_at) = unit.training_ends_at {
                let timestamp = format!("<t:{}:R>", ends_at.timestamp());
                unit_list_lines.push(format!(
                    "💪 **{}** is training **{}** (finishes {})",
                    unit_name,
                    unit.training_stat.as_deref().unwrap_or("a stat"),
                    timestamp
                ));
            } else {
                unit_list_lines.push(format!("💪 **{}** is training...", unit_name));
            }
        } else {
            unit_list_lines.push(format!("✅ **{}** is idle and ready to train.", unit_name));
            select_menu_options.push(CreateSelectMenuOption::new(
                unit_name,
                unit.player_unit_id.to_string(),
            ));
        }
    }

    embed = embed.field("Your Army", unit_list_lines.join("\n"), false);

    let mut components = Vec::new();
    if !select_menu_options.is_empty() && saga_profile.current_tp > 0 {
        let menu = CreateSelectMenu::new(
            "train_select_unit",
            CreateSelectMenuKind::String {
                options: select_menu_options,
            },
        )
        .placeholder("Select an idle unit to train...");
        components.push(CreateActionRow::SelectMenu(menu));
    } else if saga_profile.current_tp == 0 {
        // (✓) FIXED: The footer now creates the struct directly, which is the correct syntax.
        embed = embed.footer(CreateEmbedFooter::new(
            "You are out of Training Points. They recharge over time.",
        ));
    }

    // Append global nav row for cross-command navigation.
    crate::commands::saga::ui::add_nav(&mut components, "train");
    (embed, components)
}

/// Creates the stat selection menu after a unit has been chosen.
pub fn create_stat_selection_menu(player_unit_id: i32) -> (CreateEmbed, Vec<CreateActionRow>) {
    let embed = CreateEmbed::new()
        .title("Choose a Stat to Train")
        .description("Which stat would you like to improve? This will cost **1 TP** and complete in 2 hours.")
        .color(0xDAA520);

    let components = vec![CreateActionRow::Buttons(vec![
        CreateButton::new(format!("train_stat_attack_{}", player_unit_id))
            .label(pad_label("⚔️ Attack", 12))
            .style(ButtonStyle::Danger)
            // (✓) FIXED: Use a single `char` for the emoji as required by the builder.
            .emoji('⚔'),
        CreateButton::new(format!("train_stat_defense_{}", player_unit_id))
            .label(pad_label("🛡️ Defense", 12))
            .style(ButtonStyle::Primary)
            // (✓) FIXED: Use a single `char` for the emoji as required by the builder.
            .emoji('🛡'),
    ])];

    let mut rows = components;
    crate::commands::saga::ui::add_nav(&mut rows, "train");
    (embed, rows)
}
