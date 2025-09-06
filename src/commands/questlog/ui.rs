//! Contains the UI rendering logic for the `/questlog` command.

use crate::commands::economy::core::item::Item;
use crate::database::models::PlayerQuestStatus;
use crate::database::quests::QuestBoardEntry;
use serenity::builder::{CreateActionRow, CreateButton, CreateEmbed};
use crate::ui::style::pad_label;
use serenity::model::Colour;
use serenity::model::prelude::ButtonStyle;

/// Creates the embed and interactive components for the Player Quest Log.
pub fn create_questlog_embed(
    quests: &[QuestBoardEntry],
    current_view: PlayerQuestStatus,
) -> (CreateEmbed, Vec<CreateActionRow>) {
    let (title, color) = match current_view {
        PlayerQuestStatus::Accepted => ("📖 Your Active Quests", Colour::ORANGE),
        PlayerQuestStatus::Completed => ("✅ Your Completed Quests", Colour::DARK_GREEN),
        _ => ("Quest Log", Colour::BLURPLE), // Fallback
    };

    let mut embed = CreateEmbed::new().title(title).color(color);

    if quests.is_empty() {
        let description = match current_view {
            PlayerQuestStatus::Accepted => {
                "You have no active quests. Visit the `/quests` board to accept one!"
            }
            PlayerQuestStatus::Completed => "You have not completed any quests yet.",
            _ => "No quests to display.",
        };
        embed = embed.description(description);
    } else {
        for entry in quests {
            let mut reward_parts = Vec::new();
            for reward in &entry.rewards {
                if let Some(coins) = reward.reward_coins
                    && coins > 0
                {
                    reward_parts.push(format!("💰 **{}** Coins", coins));
                }
                if let (Some(_item_id), Some(quantity), Some(item)) = (
                    reward.reward_item_id,
                    reward.reward_item_quantity,
                    Item::from_i32(reward.reward_item_id.unwrap_or(0)),
                ) {
                    reward_parts.push(format!(
                        "{} **{}x** {}",
                        item.emoji(),
                        quantity,
                        item.display_name()
                    ));
                }
            }
            let reward_display = if reward_parts.is_empty() {
                "None specified.".to_string()
            } else {
                reward_parts.join("\n")
            };

            embed = embed.field(
                format!("{} — [{}]", entry.details.title, entry.details.difficulty),
                format!(
                    "**Giver:** {}\n*{}*\n\n**Rewards:**\n{}",
                    entry.details.giver_name, entry.details.description, reward_display
                ),
                false,
            );
        }
    }

    // Create the view-switcher buttons
    let active_button = CreateButton::new("questlog_view_Accepted")
        .label(pad_label("📖 View Active", 18))
        .style(ButtonStyle::Primary)
        .disabled(current_view == PlayerQuestStatus::Accepted);

    let completed_button = CreateButton::new("questlog_view_Completed")
        .label(pad_label("✅ View Completed", 20))
        .style(ButtonStyle::Secondary)
        .disabled(current_view == PlayerQuestStatus::Completed);

    let mut rows = vec![crate::commands::saga::ui::play_button_row(&crate::ui::style::pad_label("Play / Menu", 14))];
    rows.push(CreateActionRow::Buttons(vec![
        active_button,
        completed_button,
    ]));
    (embed, rows)
}
