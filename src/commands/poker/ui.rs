//! Handles all rendering and UI logic for the Poker game.

use super::state::{GamePhase, PlayerStatus, PokerGame};
use crate::ui::buttons::Btn;
use crate::ui::style::{COLOR_SAGA_MAP, COLOR_SAGA_TAVERN};
use serenity::builder::{CreateActionRow, CreateEmbed, CreateEmbedFooter};

impl PokerGame {
    pub(super) fn render_lobby(&self) -> (CreateEmbed, Vec<CreateActionRow>) {
        let players_list = self
            .players
            .iter()
            .map(|p| format!("<@{}>", p.user.id))
            .collect::<Vec<_>>()
            .join("\n");
        let desc = if self.min_bet > 0 {
            format!(
                "<@{}> has started a Five Card Poker table with an ante of **💰{}**!",
                self.host_id, self.min_bet
            )
        } else {
            format!(
                "<@{}> has started a friendly (no betting) game of Poker!",
                self.host_id
            )
        };

        let player_count = self.players.len();
        let embed = CreateEmbed::new()
            .title("♦️ Poker Lobby ♥️")
            .description(format!(
                "{}\n\n**Players ({}):**\n{}",
                desc, player_count, players_list
            ))
            .color(COLOR_SAGA_TAVERN)
            .footer(CreateEmbedFooter::new(
                "Lobby expires in 2 minutes. Max 5 players.",
            ));

        let buttons = vec![
            Btn::success("poker_join", "Join"),
            Btn::danger("poker_cancel", "Cancel (Host)"),
            Btn::primary("poker_start", "Start Game (Host)"),
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
            .description(format!(
                "Ante: **💰{}**\nPress the button to lock in.",
                self.min_bet
            ))
            .field("Player Status", ante_status, false)
            .color(COLOR_SAGA_TAVERN)
            .footer(CreateEmbedFooter::new(
                "Round starts when all are ready (60s timeout).",
            ));

        let buttons = vec![Btn::success(
            "poker_ante",
            &format!("Place Ante (💰{})", self.min_bet),
        )];
        (embed, vec![CreateActionRow::Buttons(buttons)])
    }

    pub(super) fn render_table(&self) -> (CreateEmbed, Vec<CreateActionRow>) {
        // (✓) FIXED: Added `DealerTurn` to the match arms to make them exhaustive.
        let title = match self.phase {
            GamePhase::PlayerTurns => "♥️ Poker - Your Turn to Act ♣️",
            GamePhase::GameOver | GamePhase::DealerTurn => "♠️ Poker - Final Results ♦️",
            _ => "♦️ Poker Table ♣️", // Covers Ante and WaitingForPlayers (though unused here)
        };
        let color = match self.phase {
            GamePhase::PlayerTurns => COLOR_SAGA_TAVERN, // Bronze for ongoing game
            GamePhase::GameOver | GamePhase::DealerTurn => COLOR_SAGA_MAP, // Green for completion
            _ => COLOR_SAGA_TAVERN,
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

        let dealer_rank_str =
            if self.phase == GamePhase::GameOver || self.phase == GamePhase::DealerTurn {
                if let Some(rank) = self.dealer_rank {
                    format!("({:?})", rank)
                } else {
                    "".to_string()
                }
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
                _ => {
                    let rank_str = if let Some(rank) = player.hand_rank {
                        format!("`{:?}`", rank)
                    } else {
                        "".to_string()
                    };
                    let bet_str = if self.min_bet > 0 {
                        format!(
                            "\nAnte: `💰{}` | Play: `💰{}`",
                            player.ante_bet, player.play_bet
                        )
                    } else {
                        "".to_string()
                    };
                    format!("**Rank:** {} {}", rank_str, bet_str)
                }
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
            embed = embed.description(format!("**Final Results**\n\n{}", results_str));
            // Row: quick return to Tavern
            components.push(CreateActionRow::Buttons(vec![
                crate::ui::buttons::Btn::secondary(
                    crate::interactions::ids::SAGA_TAVERN_HOME,
                    "🏰 Tavern",
                ),
            ]));
            // Global nav row for consistency across mini-games
            components.push(crate::commands::saga::ui::global_nav_row("saga"));
            if self.min_bet > 0 {
                components.push(CreateActionRow::Buttons(vec![Btn::primary(
                    "poker_next_round",
                    "Next Round (Host)",
                )]));
                embed = embed.footer(CreateEmbedFooter::new(
                    "The host has 60 seconds to start the next round.",
                ));
            }
        } else {
            // PlayerTurns
            let footer_text = format!(
                "It's <@{}>'s turn to act. You have 60 seconds to act.",
                self.players[self.current_player_index].user.id
            );
            embed = embed.footer(CreateEmbedFooter::new(footer_text));
            let buttons = vec![
                Btn::success("poker_bet", &format!("Bet ({}x Ante)", 2)),
                Btn::danger("poker_fold", "Fold"),
            ];
            components.push(CreateActionRow::Buttons(buttons));
        }

        (embed, components)
    }
}
