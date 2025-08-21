//! This module contains the full implementation of the Blackjack game,
//! including its state, rules, multiplayer support, and adherence to the `Game` trait.

use crate::commands::games::card::Card;
use crate::commands::games::deck::Deck;
use crate::commands::games::{Game, GameUpdate};
use serenity::async_trait;
use serenity::builder::{CreateActionRow, CreateButton, CreateEmbed};
use serenity::model::application::{ButtonStyle, ComponentInteraction};
use serenity::model::user::User;
use serenity::prelude::Context;
use std::any::Any;
use std::sync::Arc;

/// Represents the current phase of a Blackjack game.
#[derive(Debug, PartialEq, Eq)]
enum GamePhase {
    WaitingForPlayers,
    PlayerTurns,
    DealerTurn,
    GameOver,
}

/// Represents a player's current state within an active game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayerStatus {
    Playing,
    Stood,
    Busted,
}

/// A helper struct to manage a hand of cards and calculate its score.
struct Hand {
    cards: Vec<Card>,
}

impl Hand {
    fn new() -> Self {
        Self { cards: Vec::new() }
    }
    fn add_card(&mut self, card: Card) {
        self.cards.push(card);
    }

    /// Calculates the best possible score for a Blackjack hand.
    fn score(&self) -> u8 {
        let mut score: u8 = 0;
        let mut ace_count: u8 = 0;
        for card in &self.cards {
            let (val1, val2_opt) = card.rank.value();
            score = score.saturating_add(val1);
            if val2_opt.is_some() {
                ace_count += 1;
            }
        }
        while ace_count > 0 && score.saturating_add(10) <= 21 {
            score += 10;
            ace_count -= 1;
        }
        score
    }

    /// Formats the hand into a string like "A♥, 10♠ (21)".
    fn display(&self) -> String {
        let cards_str = self
            .cards
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join("  ");
        format!("**Cards:** {}\n**Score:** `{}`", cards_str, self.score())
    }
}

/// A struct to hold the state for a single player in the game.
struct Player {
    user: Arc<User>,
    hand: Hand,
    status: PlayerStatus,
}

/// The main struct for the Blackjack game state, now supporting multiplayer.
pub struct BlackjackGame {
    host_id: u64,
    players: Vec<Player>,
    dealer_hand: Hand,
    deck: Deck,
    phase: GamePhase,
    current_player_index: usize,
}

impl BlackjackGame {
    /// Creates a new game lobby with the host as the first player.
    pub fn new(host: Arc<User>) -> Self {
        Self {
            host_id: host.id.get(),
            players: vec![Player {
                user: host,
                hand: Hand::new(),
                status: PlayerStatus::Playing,
            }],
            dealer_hand: Hand::new(),
            deck: Deck::new(),
            phase: GamePhase::WaitingForPlayers,
            current_player_index: 0,
        }
    }

    /// Transitions the game from the lobby to active play.
    fn start_game(&mut self) {
        self.deck.shuffle();
        // Deal two cards to each player and the dealer.
        for _ in 0..2 {
            for player in self.players.iter_mut() {
                if let Some(card) = self.deck.deal_one() {
                    player.hand.add_card(card);
                }
            }
            if let Some(card) = self.deck.deal_one() {
                self.dealer_hand.add_card(card);
            }
        }
        self.phase = GamePhase::PlayerTurns;
    }

    /// Finds the next player who is still 'Playing' and advances the turn.
    /// If no players are left, it triggers the dealer's turn.
    fn advance_turn(&mut self) {
        // Find the index of the next player who is still in the 'Playing' status.
        if let Some(next_player_pos) = self.players.iter().position(|p| {
            if let Some(current_pos) = self
                .players
                .iter()
                .position(|p2| p2.user.id == self.players[self.current_player_index].user.id)
            {
                p.status == PlayerStatus::Playing
                    && self
                        .players
                        .iter()
                        .position(|p3| p3.user.id == p.user.id)
                        .unwrap()
                        > current_pos
            } else {
                false
            }
        }) {
            self.current_player_index = next_player_pos;
        } else {
            self.play_dealer_turn();
        }
    }

    /// Plays the dealer's turn according to standard rules.
    fn play_dealer_turn(&mut self) {
        self.phase = GamePhase::DealerTurn;
        while self.dealer_hand.score() < 17 {
            if let Some(card) = self.deck.deal_one() {
                self.dealer_hand.add_card(card);
            } else {
                break;
            }
        }
        self.phase = GamePhase::GameOver;
    }

    /// Determines the final outcome for each player and returns a result string.
    fn get_game_result(&self) -> String {
        let dealer_score = self.dealer_hand.score();
        self.players
            .iter()
            .map(|player| {
                let player_score = player.hand.score();
                let result = if player_score > 21 {
                    "Busted!".to_string()
                } else if dealer_score > 21 {
                    "**Wins!** (Dealer Busted)".to_string()
                } else if player_score == 21 && player.hand.cards.len() == 2 {
                    "**Blackjack!**".to_string()
                } else if player_score > dealer_score {
                    "**Wins!**".to_string()
                } else if player_score == dealer_score {
                    "Push.".to_string()
                } else {
                    "Loses.".to_string()
                };
                format!("<@{}>: {}", player.user.id, result)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[async_trait]
impl Game for BlackjackGame {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    async fn handle_interaction(
        &mut self,
        _ctx: &Context,
        interaction: &mut ComponentInteraction,
    ) -> GameUpdate {
        let user = &interaction.user;
        let custom_id = interaction.data.custom_id.as_str();

        match self.phase {
            GamePhase::WaitingForPlayers => match custom_id {
                "bj_join" => {
                    if !self.players.iter().any(|p| p.user.id == user.id) {
                        self.players.push(Player {
                            user: Arc::new(user.clone()),
                            hand: Hand::new(),
                            status: PlayerStatus::Playing,
                        });
                        GameUpdate::ReRender
                    } else {
                        GameUpdate::NoOp
                    } // Already joined
                }
                "bj_start" => {
                    if user.id.get() == self.host_id {
                        self.start_game();
                        GameUpdate::ReRender
                    } else {
                        GameUpdate::NoOp
                    } // Not the host
                }
                _ => GameUpdate::NoOp,
            },
            GamePhase::PlayerTurns => {
                if user.id != self.players[self.current_player_index].user.id {
                    return GameUpdate::NoOp;
                } // Not their turn
                match custom_id {
                    "bj_hit" => {
                        if let Some(card) = self.deck.deal_one() {
                            self.players[self.current_player_index].hand.add_card(card);
                        }
                        if self.players[self.current_player_index].hand.score() >= 21 {
                            self.players[self.current_player_index].status = PlayerStatus::Busted;
                            self.advance_turn();
                        }
                    }
                    "bj_stand" => {
                        self.players[self.current_player_index].status = PlayerStatus::Stood;
                        self.advance_turn();
                    }
                    _ => return GameUpdate::NoOp,
                }
                GameUpdate::ReRender
            }
            _ => GameUpdate::NoOp, // No interactions during dealer/game over phase
        }
    }

    fn render(&self) -> (CreateEmbed, Vec<CreateActionRow>) {
        match self.phase {
            GamePhase::WaitingForPlayers => self.render_lobby(),
            _ => self.render_table(),
        }
    }
}

// Helper functions for rendering different game phases.
impl BlackjackGame {
    fn render_lobby(&self) -> (CreateEmbed, Vec<CreateActionRow>) {
        let players_list = self
            .players
            .iter()
            .map(|p| format!("<@{}>", p.user.id))
            .collect::<Vec<_>>()
            .join("\n");
        let embed = CreateEmbed::new()
            .title("Blackjack Lobby")
            .description("Waiting for players to join...")
            .field("Players", players_list, false)
            .color(0xFFA500); // PENDING_COLOR

        let buttons = vec![
            CreateButton::new("bj_join")
                .label("Join Game")
                .style(ButtonStyle::Success),
            CreateButton::new("bj_start")
                .label("Start Game (Host Only)")
                .style(ButtonStyle::Primary),
        ];
        (embed, vec![CreateActionRow::Buttons(buttons)])
    }

    fn render_table(&self) -> (CreateEmbed, Vec<CreateActionRow>) {
        let mut embed = CreateEmbed::new().title("Blackjack Table");
        let mut components = Vec::new();

        let dealer_display = if self.phase == GamePhase::PlayerTurns {
            format!(
                "**Cards:** {}  **?**\n**Score:** `{}`",
                self.dealer_hand.cards[0],
                self.dealer_hand.cards[0].rank.value().0
            )
        } else {
            self.dealer_hand.display()
        };
        embed = embed.field("Dealer's Hand", dealer_display, false);

        for (i, player) in self.players.iter().enumerate() {
            let turn_indicator =
                if self.phase == GamePhase::PlayerTurns && i == self.current_player_index {
                    "▶️ "
                } else {
                    ""
                };
            let status_indicator = match player.status {
                PlayerStatus::Stood => " (Stood)",
                PlayerStatus::Busted => " (Busted)",
                _ => "",
            };
            embed = embed.field(
                format!(
                    "{}{}'s Hand{}",
                    turn_indicator, player.user.name, status_indicator
                ),
                player.hand.display(),
                true,
            );
        }

        if self.phase == GamePhase::GameOver {
            embed = embed.description(self.get_game_result()).color(0x00FF00);
        } else {
            embed = embed
                .description(format!(
                    "It's <@{}>'s turn.",
                    self.players[self.current_player_index].user.id
                ))
                .color(0x5865F2);
            let buttons = vec![
                CreateButton::new("bj_hit")
                    .label("Hit")
                    .style(ButtonStyle::Success),
                CreateButton::new("bj_stand")
                    .label("Stand")
                    .style(ButtonStyle::Danger),
            ];
            components.push(CreateActionRow::Buttons(buttons));
        }
        (embed, components)
    }
}
