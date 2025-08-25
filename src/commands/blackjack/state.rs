//! Defines all data structures (structs and enums) for the Blackjack game.

use crate::commands::games::card::{Card, Rank};
use crate::commands::games::deck::Deck;
use serenity::model::id::UserId;
use serenity::model::user::User;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum GamePhase {
    WaitingForPlayers,
    Betting,
    Insurance,
    PlayerTurns,
    DealerTurn,
    GameOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandStatus {
    Playing,
    Stood,
    Busted,
    Blackjack,
    Surrendered,
}

pub struct Hand {
    pub cards: Vec<Card>,
    pub bet: i64,
    pub status: HandStatus,
}

// (✓) FIXED: The implementation for Hand now lives with the struct definition.
impl Hand {
    pub fn new(bet: i64) -> Self {
        Self {
            cards: Vec::new(),
            bet,
            status: HandStatus::Playing,
        }
    }
    pub fn add_card(&mut self, card: Card) {
        self.cards.push(card);
    }
    pub fn score(&self) -> u8 {
        let (mut score, mut ace_count): (u8, u8) = (0, 0);
        for card in &self.cards {
            let (val, _) = card.rank.value();
            score = score.saturating_add(val);
            if card.rank == Rank::Ace {
                ace_count += 1;
            }
        }
        while ace_count > 0 && score.saturating_add(10) <= 21 {
            score += 10;
            ace_count -= 1;
        }
        score
    }
    pub fn can_split(&self) -> bool {
        self.cards.len() == 2 && self.cards[0].rank.value().0 == self.cards[1].rank.value().0
    }
    pub fn can_double_down(&self) -> bool {
        self.cards.len() == 2
    }
    pub fn can_surrender(&self) -> bool {
        self.cards.len() == 2
    }
    pub fn display(&self, min_bet: i64) -> String {
        let cards_str = format!(
            "[ {} ]",
            self.cards
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        );
        let bet_str = if min_bet > 0 {
            format!("(Bet: 💰{})", self.bet)
        } else {
            "".to_string()
        };
        format!("{}  `Score: {}` {}", cards_str, self.score(), bet_str)
    }
}

pub struct Player {
    pub user: Arc<User>,
    pub hands: Vec<Hand>,
    pub insurance: i64,
    pub current_bet: i64,
    pub insurance_decision_made: bool,
}

pub struct BlackjackGame {
    pub host_id: u64,
    pub players: Vec<Player>,
    pub dealer_hand: Hand,
    pub deck: Deck,
    pub phase: GamePhase,
    pub min_bet: i64,
    pub pot: i64,
    pub ready_players: HashSet<UserId>,
    pub current_player_index: usize,
    pub current_hand_index: usize,
    pub last_action_time: Instant,
}
