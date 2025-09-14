//! Handles all rendering and UI logic for the Blackjack game.

use super::state::{BlackjackGame, GamePhase, HandStatus};
use crate::ui::buttons::Btn;
use crate::ui::style::{COLOR_SAGA_MAP, COLOR_SAGA_TAVERN};
use serenity::builder::{CreateActionRow, CreateEmbed, CreateEmbedFooter};

impl BlackjackGame {
    pub(super) fn render_lobby(&self) -> (CreateEmbed, Vec<CreateActionRow>) {
        let players_list = self
            .players
            .iter()
            .map(|p| format!("<@{}>", p.user.id))
            .collect::<Vec<_>>()
            .join("\n");
        let desc = if self.min_bet > 0 {
            format!(
                "<@{}> has started a Blackjack table with a minimum bet of **💰{}**!",
                self.host_id, self.min_bet
            )
        } else {
            format!(
                "<@{}> has started a friendly (no betting) game of Blackjack!",
                self.host_id
            )
        };
        let player_count = self.players.len();
        let embed = CreateEmbed::new()
            .title("♦️ Blackjack Lobby ♥️")
            .description(format!(
                "{}\n\n**Players ({}):**\n{}",
                desc, player_count, players_list
            ))
            .color(COLOR_SAGA_TAVERN)
            .footer(CreateEmbedFooter::new(
                "Lobby expires in 2 minutes. Use Start when ready.",
            ));
        let buttons = vec![
            Btn::success("bj_join", "Join"),
            Btn::danger("bj_cancel", "Cancel (Host)"),
            Btn::primary("bj_start", "Start Game (Host)"),
        ];
        (embed, vec![CreateActionRow::Buttons(buttons)])
    }

    pub(super) fn render_betting(&self) -> (CreateEmbed, Vec<CreateActionRow>) {
        let betting_status = self
            .players
            .iter()
            .map(|p| {
                let status_icon = if self.ready_players.contains(&p.user.id) {
                    "✅"
                } else {
                    "🤔"
                };
                format!(
                    "{} <@{}> — Bet: **💰{}**",
                    status_icon, p.user.id, p.current_bet
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let embed = CreateEmbed::new()
            .title("♦️ Place Your Bets ♠️")
            .description(format!(
                "Minimum Bet: **💰{}**\nUse the buttons below to adjust. Confirm to lock in.",
                self.min_bet
            ))
            .field("Betting Status", betting_status, false)
            .color(COLOR_SAGA_TAVERN)
            .footer(CreateEmbedFooter::new(
                "Round starts when all players confirm (60s timeout).",
            ));
        let buttons1 = vec![
            Btn::secondary("bj_bet_10", "+10"),
            Btn::secondary("bj_bet_100", "+100"),
            Btn::secondary("bj_bet_1000", "+1K"),
        ];
        let buttons2 = vec![
            Btn::danger("bj_bet_all_in", "All In"),
            Btn::secondary("bj_bet_clear", "Reset Bet"),
            Btn::success("bj_bet_confirm", "Confirm Bet"),
        ];
        (
            embed,
            vec![
                CreateActionRow::Buttons(buttons1),
                CreateActionRow::Buttons(buttons2),
            ],
        )
    }

    // (✓) REFACTORED: render_table is now the main entry point for post-lobby UI.
    pub(super) fn render_table(&self) -> (CreateEmbed, Vec<CreateActionRow>) {
        if self.phase == GamePhase::GameOver {
            self.render_game_over()
        } else {
            self.render_game_in_progress()
        }
    }

    // (✓) ADDED: A dedicated renderer for the main game table UI.
    fn render_game_in_progress(&self) -> (CreateEmbed, Vec<CreateActionRow>) {
        let title = match self.phase {
            GamePhase::Insurance => "♦️ Blackjack - Insurance ♦️",
            _ => "♥️ Blackjack - In Progress ♣️",
        };
        let color = match self.phase {
            GamePhase::Insurance => 0x5865F2, // Blue
            _ => 0x5865F2,                    // Blue
        };
        let mut embed = CreateEmbed::new().title(title).color(color);
        let mut components = Vec::new();

        let dealer_display =
            if self.phase == GamePhase::PlayerTurns || self.phase == GamePhase::Insurance {
                if let Some(card) = self.dealer_hand.cards.first() {
                    format!("[ {}  **?** ]", card)
                } else {
                    "Dealing...".to_string()
                }
            } else {
                format!(
                    "[ {} ]",
                    self.dealer_hand
                        .cards
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join("  ")
                )
            };
        embed = embed.field(
            format!(
                "🤵 Dealer's Hand (`{}`)",
                if self.phase == GamePhase::PlayerTurns || self.phase == GamePhase::Insurance {
                    self.dealer_hand.cards[0].rank.value().0
                } else {
                    self.dealer_hand.score()
                }
            ),
            dealer_display,
            false,
        );

        if self.pot > 0 {
            embed = embed.field("Total Pot", format!("💰{}", self.pot), true);
        }

        for (p_idx, player) in self.players.iter().enumerate() {
            let turn_indicator =
                if self.phase == GamePhase::PlayerTurns && p_idx == self.current_player_index {
                    "▶️ "
                } else {
                    ""
                };
            let hands_display = player
                .hands
                .iter()
                .enumerate()
                .map(|(h_idx, hand)| {
                    let hand_indicator = if player.hands.len() > 1 {
                        format!("(Hand {})", h_idx + 1)
                    } else {
                        "".to_string()
                    };
                    let status_indicator = match hand.status {
                        HandStatus::Stood => " ✅",
                        HandStatus::Blackjack => " ⭐",
                        HandStatus::Busted => " ❌",
                        HandStatus::Surrendered => " 🏳️",
                        HandStatus::Playing => "",
                    };
                    let current_hand_marker = if p_idx == self.current_player_index
                        && h_idx == self.current_hand_index
                        && self.phase == GamePhase::PlayerTurns
                    {
                        "**>** "
                    } else {
                        ""
                    };
                    format!(
                        "{}{}{}: {}",
                        current_hand_marker,
                        hand_indicator,
                        status_indicator,
                        hand.display(self.min_bet)
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            embed = embed.field(
                format!("{}👤 {}", turn_indicator, player.user.name),
                hands_display,
                true,
            );
        }

        if self.phase == GamePhase::Insurance {
            embed =
                embed.description("The dealer is showing an Ace. **Place your insurance bets!**");
            components.push(CreateActionRow::Buttons(vec![
                Btn::success("bj_insure_yes", "Insure (0.5x bet)"),
                Btn::danger("bj_insure_no", "No Insurance"),
            ]));
        } else {
            // PlayerTurns
            let footer_text = format!(
                "It's <@{}>'s turn. You have 60 seconds to act.",
                self.players[self.current_player_index].user.id
            );
            embed = embed.footer(CreateEmbedFooter::new(footer_text));

            let mut buttons = vec![
                Btn::success("bj_hit", "Hit"),
                Btn::danger("bj_stand", "Stand"),
                Btn::secondary("bj_pass", "Pass"), // (✓) ADDED: Pass button
            ];

            let current_hand =
                &self.players[self.current_player_index].hands[self.current_hand_index];
            if current_hand.can_double_down() {
                buttons.push(Btn::primary("bj_double", "Double"));
            }
            if current_hand.can_split() {
                buttons.push(Btn::secondary("bj_split", "Split"));
            }
            if current_hand.can_surrender() {
                buttons.push(Btn::secondary("bj_surrender", "Surrender"));
            }

            components.push(CreateActionRow::Buttons(buttons));
        }

        (embed, components)
    }

    fn render_game_over(&self) -> (CreateEmbed, Vec<CreateActionRow>) {
        let mut embed = CreateEmbed::new()
            .title("♠️ Blackjack - Final Results ♦️")
            .color(COLOR_SAGA_MAP); // Green for success/completion
        let (results_str, _) = self.calculate_payouts();
        embed = embed.description(format!("**--- Round Over ---**\n\n{}", results_str));

        let mut rows: Vec<CreateActionRow> = Vec::new();
        // Quick return to Tavern after the game ends
        rows.push(CreateActionRow::Buttons(vec![
            crate::ui::buttons::Btn::secondary(
                crate::interactions::ids::SAGA_TAVERN_HOME,
                "🏰 Tavern",
            ),
        ]));
        // Global nav row for consistency across mini-games
        rows.push(crate::commands::saga::ui::global_nav_row("saga"));

        if self.min_bet > 0 {
            rows.push(CreateActionRow::Buttons(vec![Btn::primary(
                "bj_next_round",
                "Next Round (Host)",
            )]));
            embed = embed.footer(CreateEmbedFooter::new(
                "The host has 60 seconds to start the next round.",
            ));
        }

        (embed, rows)
    }
}
