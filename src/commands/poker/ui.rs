//! Handles all rendering and UI logic for the Poker game.

use super::state::PokerGame;
use super::state::{BlackjackGame, GamePhase, HandStatus}; // Note: Renamed in `game.rs` to PokerGame
use serenity::builder::{CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter};
use serenity::model::application::ButtonStyle;

impl PokerGame {
    pub(super) fn render_lobby(&self) -> (CreateEmbed, Vec<CreateActionRow>) {
        let players_list = self
            .players
            .iter()
            .map(|p| format!("<@{}>", p.user.id))
            .collect::<Vec<_>>()
            .join("\n");
        let desc = format!(
            "<@{}> has started a Five Card Poker table with an ante of **💰{}**!",
            self.host_id, self.min_bet
        );

        let embed = CreateEmbed::new()
            .title("♦️ Poker Lobby ♥️")
            .description(desc)
            .field("Players Joined", players_list, false)
            .color(0x71368A) // Purple
            .footer(CreateEmbedFooter::new(
                "Lobby expires in 2 minutes. Max 5 players.",
            ));

        let buttons = vec![
            CreateButton::new("poker_join")
                .label("Join")
                .style(ButtonStyle::Success),
            CreateButton::new("poker_cancel")
                .label("Cancel (Host)")
                .style(ButtonStyle::Danger),
            CreateButton::new("poker_start")
                .label("Start Game (Host)")
                .style(ButtonStyle::Primary),
        ];
        (embed, vec![CreateActionRow::Buttons(buttons)])
    }

    pub(super) fn render_ante_phase(&self) -> (CreateEmbed, Vec<CreateActionRow>) {
        let ante_status = self
            .players
            .iter()
            .map(|p| {
                let status_icon = if self.ready_players.contains(&p.user.id) {
                    "✅"
                } else {
                    "🤔"
                };
                format!("{} <@{}>", status_icon, p.user.id)
            })
            .collect::<Vec<_>>()
            .join("\n");

        let embed = CreateEmbed::new()
            .title("♦️ Place Your Antes ♠️")
            .description(format!("The ante for this table is **💰{}**. Click the button to confirm your ante and ready up.", self.min_bet))
            .field("Player Status", ante_status, false)
            .color(0xFFA500) // Orange
            .footer(CreateEmbedFooter::new("The round will begin once all players have placed their ante. | Timer: 60s"));

        let buttons = vec![
            CreateButton::new("poker_ante")
                .label(format!("Place Ante (💰{})", self.min_bet))
                .style(ButtonStyle::Success),
        ];
        (embed, vec![CreateActionRow::Buttons(buttons)])
    }

    pub(super) fn render_table(&self) -> (CreateEmbed, Vec<CreateActionRow>) {
        let title = match self.phase {
            GamePhase::PlayerTurns => "♥️ Poker - Your Turn to Act ♣️",
            GamePhase::GameOver | GamePhase::DealerTurn => "♠️ Poker - Final Results ♦️",
            _ => "♦️ Poker Table ♣️",
        };
        let color = match self.phase {
            GamePhase::PlayerTurns => 0x5865F2,                      // Blue
            GamePhase::GameOver | GamePhase::DealerTurn => 0x00FF00, // Green
            _ => 0x5865F2,
        };
        let mut embed = CreateEmbed::new().title(title).color(color);
        let mut components = Vec::new();

        let dealer_up_card = if self.phase == GamePhase::PlayerTurns {
            format!("[ {} **? ? ? ?** ]", self.dealer_hand[0])
        } else {
            format!(
                "[ {} ]",
                self.dealer_hand
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        };

        let dealer_rank_str = if let Some(rank) = self.dealer_rank {
            format!("({:?})", rank) // A simple Debug print for rank
        } else {
            "".to_string()
        };

        embed = embed.field(
            format!("🤵 Dealer's Hand {}", dealer_rank_str),
            dealer_up_card,
            false,
        );

        for (p_idx, player) in self.players.iter().enumerate() {
            let turn_indicator =
                if self.phase == GamePhase::PlayerTurns && p_idx == self.current_player_index {
                    "▶️ "
                } else {
                    ""
                };
            let hand_str = format!(
                "[ {} ]",
                player
                    .hand
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            );

            let status_text = match player.status {
                PlayerStatus::Folded => "**Folded**".to_string(),
                _ => format!("**Rank:** `{:?}`", player.hand_rank.unwrap()),
            };

            let field_value = format!("{}\n{}", hand_str, status_text);
            embed = embed.field(
                format!("{}👤 {}", turn_indicator, player.user.name),
                field_value,
                true,
            );
        }

        if self.phase == GamePhase::GameOver {
            let (results_str, _) = self.calculate_payouts();
            embed = embed.description(format!("**--- Final Results ---**\n\n{}", results_str));
            if self.min_bet > 0 {
                components.push(CreateActionRow::Buttons(vec![
                    CreateButton::new("poker_next_round")
                        .label("Next Round (Host)")
                        .style(ButtonStyle::Primary),
                ]));
                embed = embed.footer(CreateEmbedFooter::new(
                    "The host has 60 seconds to start the next round.",
                ));
            }
        } else {
            // PlayerTurns
            let footer_text = format!(
                "It's <@{}>'s turn to act. You have 60 seconds.",
                self.players[self.current_player_index].user.id
            );
            embed = embed.footer(CreateEmbedFooter::new(footer_text));
            let buttons = vec![
                CreateButton::new("poker_bet")
                    .label(format!("Bet ({}x Ante)", 2))
                    .style(ButtonStyle::Success),
                CreateButton::new("poker_fold")
                    .label("Fold")
                    .style(ButtonStyle::Danger),
            ];
            components.push(CreateActionRow::Buttons(buttons));
        }

        (embed, components)
    }
}
