//! Contains the UI and logic for the Tavern.

use crate::database::models::Unit;
use crate::ui::buttons::Btn;
use crate::ui::style::{COLOR_SAGA_TAVERN, EMOJI_COIN};
use serenity::builder::{CreateActionRow, CreateEmbed};

// For now, the tavern has a static list of recruits.
// These IDs MUST match the unit_ids from your migration.
pub const TAVERN_RECRUITS: [i32; 3] = [1, 2, 3];
pub const HIRE_COST: i64 = 250;

/// Helper embed for a successful recruit hire (DRY for interaction handlers)
pub fn recruit_success_embed(unit_name: &str, player_balance_after: i64) -> CreateEmbed {
    // Reuse generic success styling then append contextual field.
    let mut embed = crate::ui::style::success_embed(
        "Recruit Hired",
        format!("**{}** joins your forces!", unit_name),
    );
    embed = embed.field(
        "Cost",
        format!(
            "{} {} (Remaining: {} {})",
            EMOJI_COIN, HIRE_COST, EMOJI_COIN, player_balance_after
        ),
        true,
    );
    embed
}

/// Creates the embed and components for the Tavern menu.
pub fn create_tavern_menu(
    recruits: &[Unit],
    player_balance: i64,
) -> (CreateEmbed, Vec<CreateActionRow>) {
    let mut embed = CreateEmbed::new()
        .title("The Weary Dragon Tavern")
        .description("The air is thick with the smell of stale ale and adventure. A few sturdy-looking mercenaries are looking for work.")
        .field("Your Balance", format!("{} {}", EMOJI_COIN, player_balance), false)
        .color(COLOR_SAGA_TAVERN);

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
            Btn::success(
                &format!("saga_hire_{}", unit.unit_id),
                &format!("➕ Hire {}", unit.name),
            )
            .disabled(player_balance < HIRE_COST),
        );
    }

    let action_row = CreateActionRow::Buttons(components);
    // Include global nav row and action buttons.
    let rows = vec![
        crate::commands::saga::ui::global_nav_row("saga"),
        action_row,
    ];
    (embed, rows)
}
