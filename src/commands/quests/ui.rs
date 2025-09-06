//! Contains the UI rendering logic for the `/quests` command.

use crate::commands::economy::core::item::Item;
use crate::database::models::PlayerQuestStatus;
use crate::database::quests::QuestBoardEntry;
use serenity::builder::{CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter};
use crate::ui::style::pad_primary;
use serenity::model::Colour;
use serenity::model::prelude::ButtonStyle;

/// Creates the embed and interactive components for the Quest Board.
/// # Returns
/// A tuple containing the `CreateEmbed` and a `Vec<CreateActionRow>` with "Accept" buttons.
pub fn create_quest_board_embed(quests: &[QuestBoardEntry]) -> (CreateEmbed, Vec<CreateActionRow>) {
    let mut embed = CreateEmbed::new()
        .title("📜 Guild Quest Board")
        .color(Colour::from_rgb(150, 111, 51)) // Brown color
        .description(
            "Here are the latest postings from around the realm. Choose a quest to accept.",
        )
        .footer(CreateEmbedFooter::new(
            "The board refreshes when all current quests are completed.",
        ));

    let mut buttons = Vec::new();

    if quests.is_empty() {
        embed =
            embed.description("There are no new quests available at the moment. Check back later!");
    } else {
        for entry in quests {
            let mut reward_parts = Vec::new();
            for reward in &entry.rewards {
                // (✓) FIXED: Collapsed nested `if` statements as recommended by clippy.
                if let Some(coins) = reward.reward_coins
                    && coins > 0
                {
                    reward_parts.push(format!("💰 **{}** Coins", coins));
                }
                // (✓) FIXED: Collapsed nested `if let` and ignored unused `_item_id`.
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

            // (✓) IMPROVED: Display the quest status to make the `status` field "live".
            let status_text = match entry.details.status {
                PlayerQuestStatus::Offered => "Status: **Offered**",
                _ => "Status: Unknown", // This case should not appear on the quest board
            };

            embed = embed.field(
                format!("{} — [{}]", entry.details.title, entry.details.difficulty),
                format!(
                    "{}\n**Giver:** {}\n*{}*\n\n**Rewards:**\n{}",
                    status_text,
                    entry.details.giver_name,
                    entry.details.description,
                    reward_display
                ),
                false,
            );

            buttons.push(
                CreateButton::new(format!("quest_accept_{}", entry.details.player_quest_id))
                    .label(pad_primary("🆗 Accept"))
                    .style(ButtonStyle::Primary),
            );
        }
    }

    let mut rows: Vec<CreateActionRow> = Vec::new();
    rows.push(crate::commands::saga::ui::global_nav_row("saga"));
    if !buttons.is_empty() {
        rows.push(CreateActionRow::Buttons(buttons));
    }
    (embed, rows)
}
