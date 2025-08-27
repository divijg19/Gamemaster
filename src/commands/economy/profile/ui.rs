//! Handles all UI and embed creation for the `/profile` command.

// (✓) ADDED: Import the new core profile logic to calculate XP needed.
use crate::commands::economy::core;
use crate::database;
use serenity::builder::CreateEmbed;
use serenity::model::user::User;

pub fn create_profile_embed(
    user: &User,
    profile_result: Result<database::profile::Profile, sqlx::Error>,
    inventory_result: Result<Vec<database::profile::InventoryItem>, sqlx::Error>,
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

            // (✓) MODIFIED: Activated the job level display.
            // This now shows the user's current level and XP progress for each job.
            embed = embed.field("\u{200B}", "\u{200B}", false); // Spacer

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
