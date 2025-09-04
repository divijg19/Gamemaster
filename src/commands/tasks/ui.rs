//! Contains the UI rendering logic for the `/tasks` command.

use crate::commands::economy::core::item::Item;
use crate::database::models::{PlayerTaskDetails, TaskType};
use serenity::builder::{CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter};
use serenity::model::Colour;
use serenity::model::prelude::ButtonStyle;

/// Creates the embed and interactive components for the player's tasks view.
///
/// # Returns
/// A tuple containing the `CreateEmbed` and a `Vec<CreateActionRow>` with claim buttons.
pub fn create_tasks_embed(tasks: &[PlayerTaskDetails]) -> (CreateEmbed, Vec<CreateActionRow>) {
    let mut daily_tasks_str = String::new();
    let mut weekly_tasks_str = String::new();
    let mut claim_buttons = Vec::new();

    for task in tasks {
        let status_icon = if task.is_completed { "✅" } else { "⬜" };
        let progress_str = format!("{}/{}", task.progress, task.objective_goal);

        let mut reward_parts = Vec::new();
        // (✓) FIXED: Collapsed nested `if` statements as recommended by clippy.
        if let Some(coins) = task.reward_coins
            && coins > 0
        {
            reward_parts.push(format!("💰 **{}** coins", coins));
        }
        if let (Some(_item_id), Some(quantity), Some(item)) = (
            task.reward_item_id,
            task.reward_item_quantity,
            Item::from_i32(task.reward_item_id.unwrap_or(0)),
        ) {
            reward_parts.push(format!(
                "{} **{}x** {}",
                item.emoji(),
                quantity,
                item.display_name()
            ));
        }
        let reward_display = if reward_parts.is_empty() {
            "No reward specified.".to_string()
        } else {
            reward_parts.join(" and ")
        };

        let task_line = format!(
            "{} **{}**\n*{}* `({})`\n> **Reward:** {}\n\n",
            status_icon, task.title, task.description, progress_str, reward_display
        );

        match task.task_type {
            TaskType::Daily => daily_tasks_str.push_str(&task_line),
            TaskType::Weekly => weekly_tasks_str.push_str(&task_line),
        }

        if task.is_completed {
            let button = CreateButton::new(format!("task_claim_{}", task.player_task_id))
                .label(format!("Claim {}", task.title))
                .style(ButtonStyle::Success);
            claim_buttons.push(button);
        }
    }

    if daily_tasks_str.is_empty() {
        daily_tasks_str = "No daily tasks available.".to_string();
    }
    if weekly_tasks_str.is_empty() {
        weekly_tasks_str = "No weekly tasks available.".to_string();
    }

    // Build action row with claim buttons (using enum variant since `from_buttons` is unavailable).
    let mut rows: Vec<CreateActionRow> = Vec::new();
    rows.push(crate::commands::saga::ui::play_button_row("Play / Menu"));
    if !claim_buttons.is_empty() {
        rows.push(CreateActionRow::Buttons(claim_buttons));
    }

    let embed = CreateEmbed::new()
        .title("Your Tasks")
        .color(Colour::BLURPLE)
        .field("Daily Tasks", daily_tasks_str, false)
        .field("Weekly Tasks", weekly_tasks_str, false)
        .footer(CreateEmbedFooter::new(
            "Daily tasks reset at 00:00 UTC. Weekly tasks reset on Monday.",
        ));

    (embed, rows)
}
