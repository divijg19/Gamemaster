//! Handles all rendering and UI logic for the Blackjack game.

use super::state::{BlackjackGame, GamePhase, HandStatus};
use serenity::builder::{CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter};
use serenity::model::application::ButtonStyle;

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
        let embed = CreateEmbed::new()
            .title("♦️ Blackjack Lobby ♥️")
            .description(desc)
            .field("Players Joined", players_list, false)
            .color(0x71368A) // (✓) MODIFIED: Purple color for lobby
            .footer(CreateEmbedFooter::new("Lobby expires in 2 minutes."));
        let buttons = vec![
            CreateButton::new("bj_join")
                .label("Join")
                .style(ButtonStyle::Success),
            CreateButton::new("bj_cancel")
                .label("Cancel (Host)")
                .style(ButtonStyle::Danger),
            CreateButton::new("bj_start")
                .label("Start Game (Host)")
                .style(ButtonStyle::Primary),
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
            .description(format!("Minimum Bet: **💰{}**", self.min_bet))
            .field("Betting Status", betting_status, false)
            .color(0xFFA500) // (✓) MODIFIED: Orange color for betting
            .footer(CreateEmbedFooter::new(
                "Round starts when all players confirm. | Timer: 60s",
            ));
        let buttons1 = vec![
            CreateButton::new("bj_bet_10")
                .label("+10")
                .style(ButtonStyle::Secondary),
            CreateButton::new("bj_bet_100")
                .label("+100")
                .style(ButtonStyle::Secondary),
            CreateButton::new("bj_bet_1000")
                .label("+1K")
                .style(ButtonStyle::Secondary),
        ];
        let buttons2 = vec![
            CreateButton::new("bj_bet_all_in")
                .label("All In")
                .style(ButtonStyle::Danger),
            CreateButton::new("bj_bet_clear")
                .label("Reset Bet")
                .style(ButtonStyle::Secondary),
            CreateButton::new("bj_bet_confirm")
                .label("Confirm Bet")
                .style(ButtonStyle::Success),
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
            embed = embed.field("Total Pot", format!("💰{}", self.pot), false);
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
                CreateButton::new("bj_insure_yes")
                    .label("Insure (0.5x bet)")
                    .style(ButtonStyle::Success),
                CreateButton::new("bj_insure_no")
                    .label("No Insurance")
                    .style(ButtonStyle::Danger),
            ]));
        } else {
            // PlayerTurns
            let footer_text = format!(
                "It's <@{}>'s turn. You have 60 seconds to act.",
                self.players[self.current_player_index].user.id
            );
            embed = embed.footer(CreateEmbedFooter::new(footer_text));

            let mut buttons = vec![
                CreateButton::new("bj_hit")
                    .label("Hit")
                    .style(ButtonStyle::Success),
                CreateButton::new("bj_stand")
                    .label("Stand")
                    .style(ButtonStyle::Danger),
                CreateButton::new("bj_pass")
                    .label("Pass")
                    .style(ButtonStyle::Secondary), // (✓) ADDED: Pass button
            ];

            let current_hand =
                &self.players[self.current_player_index].hands[self.current_hand_index];
            if current_hand.can_double_down() {
                buttons.push(
                    CreateButton::new("bj_double")
                        .label("Double")
                        .style(ButtonStyle::Primary),
                );
            }
            if current_hand.can_split() {
                buttons.push(
                    CreateButton::new("bj_split")
                        .label("Split")
                        .style(ButtonStyle::Secondary),
                );
            }
            if current_hand.can_surrender() {
                buttons.push(
                    CreateButton::new("bj_surrender")
                        .label("Surrender")
                        .style(ButtonStyle::Secondary),
                );
            }

            components.push(CreateActionRow::Buttons(buttons));
        }

        (embed, components)
    }

    fn render_game_over(&self) -> (CreateEmbed, Vec<CreateActionRow>) {
        let mut embed = CreateEmbed::new()
            .title("♠️ Blackjack - Final Results ♦️")
            .color(0x00FF00); // Green
        let mut components = Vec::new();
        let (results_str, _) = self.calculate_payouts();
        embed = embed.description(format!("**--- Round Over ---**\n\n{}", results_str));
        if self.min_bet > 0 {
            components.push(CreateActionRow::Buttons(vec![
                CreateButton::new("bj_next_round")
                    .label("Next Round (Host)")
                    .style(ButtonStyle::Primary),
            ]));
            embed = embed.footer(CreateEmbedFooter::new(
                "The host has 60 seconds to start the next round.",
            ));
        }
        (embed, components)
    }
}
