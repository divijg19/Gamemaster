use serenity::model::user::User;
use std::sync::Arc;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Move {
    Rock,
    Paper,
    Scissors,
}

impl Move {
    pub fn to_emoji(self) -> &'static str {
        match self {
            Move::Rock => "✊",
            Move::Paper => "✋",
            Move::Scissors => "✌️",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum DuelFormat {
    BestOf(u32),
    RaceTo(u32),
}

#[derive(Clone, Copy, Debug)]
pub struct Scores {
    pub p1: u32,
    pub p2: u32,
}

// CORRECTED: The enum no longer has a lifetime. It now owns the winner's data
// by cloning the Arc, which is a cheap operation.
#[derive(Debug, Clone)]
pub enum RoundOutcome {
    Tie,
    Winner(Arc<User>),
}

#[derive(Clone)]
pub struct GameState {
    pub player1: Arc<User>,
    pub player2: Arc<User>,
    pub p1_move: Option<Move>,
    pub p2_move: Option<Move>,
    pub accepted: bool,
    pub format: DuelFormat,
    pub scores: Scores,
    pub round: u32,
}

impl GameState {
    pub fn new(player1: Arc<User>, player2: Arc<User>, format: DuelFormat) -> Self {
        Self {
            player1,
            player2,
            p1_move: None,
            p2_move: None,
            accepted: false,
            format,
            scores: Scores { p1: 0, p2: 0 },
            round: 1,
        }
    }

    pub fn get_target_score(&self) -> u32 {
        match self.format {
            DuelFormat::BestOf(n) => (n / 2) + 1,
            DuelFormat::RaceTo(n) => n,
        }
    }

    pub fn is_over(&self) -> bool {
        let target = self.get_target_score();
        self.scores.p1 >= target || self.scores.p2 >= target
    }

    pub fn process_round(&mut self) -> Option<RoundOutcome> {
        if let (Some(p1m), Some(p2m)) = (self.p1_move, self.p2_move) {
            let outcome = match (p1m, p2m) {
                (u, b) if u == b => RoundOutcome::Tie,
                (Move::Rock, Move::Scissors)
                | (Move::Paper, Move::Rock)
                | (Move::Scissors, Move::Paper) => {
                    self.scores.p1 += 1;
                    // CORRECTED: Clone the Arc to give ownership to the outcome.
                    RoundOutcome::Winner(self.player1.clone())
                }
                _ => {
                    self.scores.p2 += 1;
                    RoundOutcome::Winner(self.player2.clone())
                }
            };
            Some(outcome)
        } else {
            None
        }
    }

    pub fn prepare_for_next_round(&mut self) {
        self.p1_move = None;
        self.p2_move = None;
        self.round += 1;
    }
}
