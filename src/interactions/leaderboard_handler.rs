//! Handles all component interactions for the `leaderboard` command family.

use crate::saga::leaderboard::LeaderboardType;
use crate::{AppState, commands, database};
use serenity::builder::EditInteractionResponse;
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;
use std::sync::Arc;

pub async fn handle(ctx: &Context, component: &mut ComponentInteraction, app_state: Arc<AppState>) {
    let db = app_state.db.clone();
    component.defer(&ctx.http).await.ok();

    // Determine which leaderboard to show based on the button's custom_id.
    let board_type = match component.data.custom_id.as_str() {
        "leaderboard_wealth" => LeaderboardType::Wealth,
        "leaderboard_streak" => LeaderboardType::WorkStreak,
        _ => LeaderboardType::Gamemaster, // Default to the main leaderboard.
    };

    // Fetch the data for the selected leaderboard.
    let entries = match board_type {
        LeaderboardType::Gamemaster => {
            database::leaderboard::get_gamemaster_leaderboard(&db, 10).await
        }
        LeaderboardType::Wealth => database::leaderboard::get_wealth_leaderboard(&db, 10).await,
        LeaderboardType::WorkStreak => database::leaderboard::get_streak_leaderboard(&db, 10).await,
    }
    .unwrap_or_default();

    // Re-render the embed and buttons with the new data.
    let embed =
        commands::leaderboard::ui::create_leaderboard_embed(ctx, &entries, board_type).await;
    let components = vec![commands::leaderboard::ui::create_leaderboard_buttons(
        board_type,
    )];

    let builder = EditInteractionResponse::new()
        .embed(embed)
        .components(components);
    component.edit_response(&ctx.http, builder).await.ok();
}
