//! This module contains the full implementation of the Blackjack game,
//! including its state, rules, and adherence to the `Game` trait.

// (✓) CORRECTED: Removed the unused `Rank` enum from the import statement.
use crate::commands::games::card::Card;
use crate::commands::games::deck::Deck;
use crate::commands::games::{Game, GameUpdate};
use serenity::async_trait;
use serenity::builder::{CreateActionRow, CreateButton, CreateEmbed};
use serenity::model::application::{ButtonStyle, ComponentInteraction};
use serenity::prelude::Context;
use std::any::Any;

/// Represents the current phase of a Blackjack game.
#[derive(Debug, PartialEq, Eq)]
enum GamePhase {
    PlayerTurn,
    DealerTurn,
    GameOver,
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
            score += val1;
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

/// The main struct for the Blackjack game state.
pub struct BlackjackGame {
    deck: Deck,
    player_hand: Hand,
    dealer_hand: Hand,
    phase: GamePhase,
    // TODO: bet: i64,
}

impl BlackjackGame {
    /// Creates a new game, shuffles the deck, and deals the initial hands.
    pub fn new() -> Self {
        let mut deck = Deck::new();
        deck.shuffle();
        let mut player_hand = Hand::new();
        let mut dealer_hand = Hand::new();

        player_hand.add_card(deck.deal_one().unwrap());
        dealer_hand.add_card(deck.deal_one().unwrap());
        player_hand.add_card(deck.deal_one().unwrap());
        dealer_hand.add_card(deck.deal_one().unwrap());

        let phase = if player_hand.score() == 21 {
            GamePhase::DealerTurn
        } else {
            GamePhase::PlayerTurn
        };

        let mut game = Self {
            deck,
            player_hand,
            dealer_hand,
            phase,
        };

        if game.phase == GamePhase::DealerTurn {
            game.play_dealer_turn();
        }

        game
    }

    /// Plays the dealer's turn according to standard rules (hit until 17 or more).
    fn play_dealer_turn(&mut self) {
        while self.dealer_hand.score() < 17 {
            if let Some(card) = self.deck.deal_one() {
                self.dealer_hand.add_card(card);
            } else {
                break;
            }
        }
        self.phase = GamePhase::GameOver;
    }

    /// Determines the final outcome of the game and returns a result string.
    fn get_game_result(&self) -> String {
        let player_score = self.player_hand.score();
        let dealer_score = self.dealer_hand.score();

        if player_score > 21 {
            return "You busted! **Dealer wins.**".to_string();
        }
        if dealer_score > 21 {
            return "Dealer busted! **You win!**".to_string();
        }
        if player_score == 21 && self.player_hand.cards.len() == 2 {
            return "**Blackjack! You win!**".to_string();
        }
        if player_score == dealer_score {
            return "**It's a push!** (Tie)".to_string();
        }
        if player_score > dealer_score {
            return "**You win!**".to_string();
        }
        "**Dealer wins.**".to_string()
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
        if self.phase != GamePhase::PlayerTurn {
            return GameUpdate::NoOp;
        }

        let custom_id = interaction.data.custom_id.as_str();
        match custom_id {
            "bj_hit" => {
                if let Some(card) = self.deck.deal_one() {
                    self.player_hand.add_card(card);
                }
                if self.player_hand.score() >= 21 {
                    self.play_dealer_turn();
                }
            }
            "bj_stand" => {
                self.play_dealer_turn();
            }
            _ => return GameUpdate::NoOp,
        }
        GameUpdate::ReRender
    }

    fn render(&self) -> (CreateEmbed, Vec<CreateActionRow>) {
        let mut embed = CreateEmbed::new().title("Blackjack");
        let mut components = Vec::new();

        let dealer_display = if self.phase == GamePhase::PlayerTurn {
            format!(
                "**Cards:** {}  **?**\n**Score:** `{}`",
                self.dealer_hand.cards[0],
                self.dealer_hand.cards[0].rank.value().0
            )
        } else {
            self.dealer_hand.display()
        };

        embed = embed.field("Dealer's Hand", dealer_display, false).field(
            "Your Hand",
            self.player_hand.display(),
            false,
        );

        if self.phase == GamePhase::GameOver {
            embed = embed.description(self.get_game_result()).color(0x00FF00);
        } else {
            embed = embed
                .description("It's your turn. Hit or Stand?")
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
