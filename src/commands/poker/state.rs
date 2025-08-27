//! Defines all data structures (structs and enums) for the Poker game.

use crate::commands::games::card::Card;
use crate::commands::games::deck::Deck;
use serenity::model::id::UserId;
use serenity::model::user::User;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum GamePhase {
    WaitingForPlayers,
    Ante,
    PlayerTurns,
    DealerTurn, // (✓) FIXED: Added the missing DealerTurn phase.
    GameOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerStatus {
    Waiting,
    Playing,
    Folded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HandRank {
    HighCard(u8),
    OnePair(u8),
    TwoPair(u8, u8),
    ThreeOfAKind(u8),
    Straight(u8),
    Flush(u8),
    FullHouse(u8, u8),
    FourOfAKind(u8),
    StraightFlush(u8),
    RoyalFlush,
}

pub struct Player {
    pub user: Arc<User>,
    pub hand: Vec<Card>,
    pub hand_rank: Option<HandRank>,
    pub ante_bet: i64,
    pub play_bet: i64,
    pub status: PlayerStatus,
}

pub struct PokerGame {
    pub host_id: u64,
    pub players: Vec<Player>,
    pub dealer_hand: Vec<Card>,
    pub dealer_rank: Option<HandRank>,
    pub deck: Deck,
    pub phase: GamePhase,
    pub min_bet: i64, // The Ante
    pub pot: i64,
    pub round: u32,
    pub ready_players: HashSet<UserId>,
    pub current_player_index: usize,
    pub last_action_time: Instant,
}
