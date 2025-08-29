//! Handles all UI and embed creation for the `work` command.

use super::jobs::Job;
use crate::database::profile::WorkRewards;
use chrono::Duration;
use serenity::builder::CreateEmbed;

/// Creates the embed for a successful work session.
pub fn create_success_embed(
    job: &Job,
    rewards: &WorkRewards,
    reward_lines: Vec<String>,
    streak_bonus: i64,
    level_up_info: Option<(i32, i64)>,
) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title(format!("Work Complete: {}!", job.display_name))
        .color(0x00FF00); // Green

    if streak_bonus > 0 {
        embed = embed.description(format!("💰 **Streak Bonus:** +`{}` coins!", streak_bonus));
    }

    embed = embed.field("Rewards", reward_lines.join("\n"), false);

    if let Some((new_level, xp_needed)) = level_up_info {
        // This XP would be the remainder after leveling up. For now, we'll show it as full.
        let xp_bar = create_xp_bar(rewards.xp, xp_needed);
        embed = embed.field(
            "🎉 Level Up! 🎉", // Field name must be a String or &str
            format!(
                "Congratulations! You've reached **Level {}** in {}!\n{}",
                new_level, job.display_name, xp_bar
            ),
            false,
        );
    }

    embed
}

/// Creates the embed for when a user is on cooldown.
pub fn create_cooldown_embed(remaining: Duration) -> CreateEmbed {
    CreateEmbed::new()
        .title("On Cooldown")
        .description(format!(
            "You can work again in **{}**.",
            format_duration(remaining)
        ))
        .color(0xFF0000) // Red
}

/// Creates a generic error embed.
pub fn create_error_embed(error_message: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title("Error")
        .description(error_message)
        .color(0xFF0000) // Red
}

/// Creates a text-based progress bar for XP.
fn create_xp_bar(current_xp: i64, xp_needed: i64) -> String {
    // (✓) FIXED: Replaced the manual min/max pattern with the more idiomatic `clamp`.
    let progress = (current_xp as f64 / xp_needed as f64).clamp(0.0, 1.0);
    let filled_blocks = (progress * 10.0).round() as usize;
    let empty_blocks = 10 - filled_blocks;
    format!(
        "`[{}{}]` {} / {} XP",
        "█".repeat(filled_blocks),
        "─".repeat(empty_blocks),
        current_xp,
        xp_needed
    )
}

/// Formats a `chrono::Duration` into a user-friendly string.
pub fn format_duration(dur: Duration) -> String {
    let hours = dur.num_hours();
    let minutes = dur.num_minutes() % 60;
    let seconds = dur.num_seconds() % 60;

    let mut parts = Vec::new();
    if hours > 0 {
        parts.push(format!("{}h", hours));
    }
    if minutes > 0 {
        parts.push(format!("{}m", minutes));
    }
    if seconds > 0 && hours == 0 {
        parts.push(format!("{}s", seconds));
    }

    if parts.is_empty() {
        "less than a second".to_string()
    } else {
        parts.join(" ")
    }
}
