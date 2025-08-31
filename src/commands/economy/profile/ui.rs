//! Handles all UI and embed creation for the `/profile` command.

use crate::commands::economy::core;
use crate::database;
use crate::database::models::SagaProfile;
use serenity::builder::CreateEmbed;
use serenity::model::user::User;

pub fn create_profile_embed(
    user: &User,
    profile_result: Result<database::models::Profile, sqlx::Error>,
    inventory_result: Result<Vec<database::models::InventoryItem>, sqlx::Error>,
    saga_result: Result<SagaProfile, sqlx::Error>,
) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title(format!("{}'s Profile", user.name))
        .thumbnail(user.face());

    match profile_result {
        Ok(profile) => {
            embed = embed
                .color(0x5865F2) // Blue
                .field("💰 Balance", format!("`{}` coins", profile.balance), true)
                .field(
                    "📈 Work Streak",
                    format!("`{}` days", profile.work_streak),
                    true,
                );

            if let Ok(saga) = saga_result {
                embed = embed.field("\u{200B}", "\u{200B}", true); // Inline Spacer for alignment
                embed = embed.field(
                    "⚔️ Action Points",
                    format!("`{}/{}`", saga.current_ap, saga.max_ap),
                    true,
                );
                embed = embed.field(
                    "⚡ Training Points",
                    format!("`{}/{}`", saga.current_tp, saga.max_tp),
                    true,
                );
                // (✓) ALIVE: The story_progress field is now displayed to the user.
                embed = embed.field(
                    "🗺️ Story Progress",
                    format!("Chapter `{}`", saga.story_progress),
                    true,
                );
            }

            let inventory_display = match inventory_result {
                Ok(inventory) if inventory.is_empty() => "Nothing to see here!".to_string(),
                Ok(inventory) => inventory
                    .iter()
                    .map(|item| format!("- **{}**: `{}`", item.name, item.quantity))
                    .collect::<Vec<_>>()
                    .join("\n"),
                Err(_) => "_Could not load inventory._".to_string(),
            };
            embed = embed.field("🎒 Inventory", inventory_display, false);

            embed = embed.field("\u{200B}", "\u{200B}", false); // Full-width spacer

            let fishing_xp_needed = core::profile::xp_for_level(profile.fishing_level);
            embed = embed.field(
                "🎣 Fishing",
                format!(
                    "Level {} (`{}/{}` XP)",
                    profile.fishing_level, profile.fishing_xp, fishing_xp_needed
                ),
                true,
            );

            let mining_xp_needed = core::profile::xp_for_level(profile.mining_level);
            embed = embed.field(
                "⛏️ Mining",
                format!(
                    "Level {} (`{}/{}` XP)",
                    profile.mining_level, profile.mining_xp, mining_xp_needed
                ),
                true,
            );

            let coding_xp_needed = core::profile::xp_for_level(profile.coding_level);
            embed = embed.field(
                "💻 Coding",
                format!(
                    "Level {} (`{}/{}` XP)",
                    profile.coding_level, profile.coding_xp, coding_xp_needed
                ),
                true,
            );
        }
        Err(e) => {
            println!("[PROFILE CMD] Database error: {:?}", e);
            embed = embed
                .color(0xFF0000)
                .description("Could not retrieve profile data due to a database error.");
        }
    }

    embed
}
