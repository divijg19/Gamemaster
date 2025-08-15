use serenity::model::user::User;
use std::sync::Arc;

// Represents a player's move.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Move {
    Rock,
    Paper,
    Scissors,
}

impl Move {
    // --- CORRECTED: The `&` is removed from `&self` to adhere to Rust conventions for `Copy` types. ---
    pub fn to_emoji(self) -> &'static str {
        match self {
            Move::Rock => "🪨",
            Move::Paper => "📜",
            Move::Scissors => "✂️",
        }
    }
}

// Defines the win condition for the duel.
#[derive(Clone, Copy, Debug)]
pub enum DuelFormat {
    BestOf(u32),
    RaceTo(u32),
}

// Holds the complete state for an active duel.
#[derive(Clone)]
pub struct GameState {
    pub player1: Arc<User>,
    pub player2: Arc<User>,
    pub p1_move: Option<Move>,
    pub p2_move: Option<Move>,
    pub accepted: bool,
    pub format: DuelFormat,
    pub scores: (u32, u32), // (p1_score, p2_score)
    pub round: u32,
}
