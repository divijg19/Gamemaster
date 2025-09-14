//! Handles the UI creation for the `/leaderboard` command.

// (✓) FIXED: Corrected the path to the LeaderboardType enum.
use crate::database::leaderboard::LeaderboardEntry;
use crate::saga::leaderboard::LeaderboardType;
use crate::ui::buttons::Btn;
use serenity::builder::{CreateActionRow, CreateEmbed};
use serenity::prelude::Context;

/// Creates the main embed for a given leaderboard type and its data.
pub async fn create_leaderboard_embed(
    ctx: &Context,
    entries: &[LeaderboardEntry],
    board_type: LeaderboardType,
) -> CreateEmbed {
    let mut description_lines = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        let rank = i + 1;
        let user_id = serenity::model::id::UserId::new(entry.user_id as u64);
        let user_name = user_id
            .to_user(&ctx.http)
            .await
            .map_or_else(|_| "Unknown User".to_string(), |u| u.name);

        let medal = match rank {
            1 => "🥇",
            2 => "🥈",
            3 => "🥉",
            _ => "🔹",
        };

        description_lines.push(format!(
            "{} **{}**. {} - `{} {}`",
            medal,
            rank,
            user_name,
            entry.score,
            board_type.score_name()
        ));
    }

    let description = if description_lines.is_empty() {
        "The leaderboard is currently empty.".to_string()
    } else {
        description_lines.join("\n")
    };

    CreateEmbed::new()
        .title(board_type.title())
        .description(description)
        .color(0xFFD700) // Gold
}

/// Creates the row of buttons used to switch between leaderboards.
pub fn create_leaderboard_buttons(current_board: LeaderboardType) -> CreateActionRow {
    let gm = if current_board == LeaderboardType::Gamemaster {
        Btn::primary("leaderboard_gamemaster", "Gamemaster")
    } else {
        Btn::secondary("leaderboard_gamemaster", "Gamemaster")
    };

    let wealth = if current_board == LeaderboardType::Wealth {
        Btn::primary("leaderboard_wealth", "Wealth")
    } else {
        Btn::secondary("leaderboard_wealth", "Wealth")
    };

    let streak = if current_board == LeaderboardType::WorkStreak {
        Btn::primary("leaderboard_streak", "Work Streak")
    } else {
        Btn::secondary("leaderboard_streak", "Work Streak")
    };

    CreateActionRow::Buttons(vec![gm, wealth, streak])
}
