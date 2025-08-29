//! Handles the UI creation for the `/leaderboard` command.

// (✓) FIXED: Corrected the path to the LeaderboardType enum.
use crate::database::leaderboard::LeaderboardEntry;
use crate::saga::leaderboard::LeaderboardType;
use serenity::builder::{CreateActionRow, CreateButton, CreateEmbed};
use serenity::model::application::ButtonStyle;
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
    CreateActionRow::Buttons(vec![
        CreateButton::new("leaderboard_gamemaster")
            .label("Gamemaster")
            .style(if current_board == LeaderboardType::Gamemaster {
                ButtonStyle::Primary
            } else {
                ButtonStyle::Secondary
            }),
        CreateButton::new("leaderboard_wealth")
            .label("Wealth")
            .style(if current_board == LeaderboardType::Wealth {
                ButtonStyle::Primary
            } else {
                ButtonStyle::Secondary
            }),
        CreateButton::new("leaderboard_streak")
            .label("Work Streak")
            .style(if current_board == LeaderboardType::WorkStreak {
                ButtonStyle::Primary
            } else {
                ButtonStyle::Secondary
            }),
    ])
}
