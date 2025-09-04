//! Contains the UI and logic for the Tavern.

use crate::database::models::Unit;
use serenity::builder::{CreateActionRow, CreateButton, CreateEmbed};
use serenity::model::application::ButtonStyle;

// For now, the tavern has a static list of recruits.
// These IDs MUST match the unit_ids from your migration.
pub const TAVERN_RECRUITS: [i32; 3] = [1, 2, 3];
pub const HIRE_COST: i64 = 250;

/// Creates the embed and components for the Tavern menu.
pub fn create_tavern_menu(
    recruits: &[Unit],
    player_balance: i64,
) -> (CreateEmbed, Vec<CreateActionRow>) {
    let mut embed = CreateEmbed::new()
        .title("The Weary Dragon Tavern")
        .description("The air is thick with the smell of stale ale and adventure. A few sturdy-looking mercenaries are looking for work.")
        .field("Your Balance", format!("💰 {}", player_balance), false)
        .color(0xCD7F32); // Bronze

    let mut components = Vec::new();
    for unit in recruits {
        embed = embed.field(
            &unit.name,
            format!(
                "{} Atk: `{}` | Def: `{}` | HP: `{}`\n*Cost: {} coins*",
                unit.description.as_deref().unwrap_or(""),
                unit.base_attack,
                unit.base_defense,
                unit.base_health,
                HIRE_COST
            ),
            false,
        );
        components.push(
            CreateButton::new(format!("saga_hire_{}", unit.unit_id))
                .label(format!("Hire {}", unit.name))
                .style(ButtonStyle::Success)
                .disabled(player_balance < HIRE_COST),
        );
    }

    let action_row = CreateActionRow::Buttons(components);
    // Prepend Play row for consistent navigation.
    let mut rows = vec![crate::commands::saga::ui::play_button_row("Play / Menu")];
    rows.push(action_row);
    (embed, rows)
}
