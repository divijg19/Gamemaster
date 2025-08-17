use serenity::model::id::UserId;
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
            Move::Rock => "🤜",
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

// A thoughtful replacement for the ambiguous (u32, u32) tuple.
#[derive(Clone, Copy, Debug)]
pub struct Scores {
    pub p1: u32,
    pub p2: u32,
}

// A new struct to immutably store the result of a single round for the history log.
#[derive(Clone, Debug)]
pub struct RoundRecord {
    pub p1_move: Move,
    pub p2_move: Move,
    pub outcome: RoundOutcome,
}

// The outcome now stores the UserId of the winner for easy display.
#[derive(Debug, Clone, PartialEq)]
pub enum RoundOutcome {
    Tie,
    Winner(UserId),
}

// The GameState now includes a history of all completed rounds.
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
    pub history: Vec<RoundRecord>,
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
            history: Vec::new(),
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

    // This is now the single source of truth for game logic. It processes moves,
    // updates scores, and records the round in history all at once.
    pub fn process_round(&mut self) {
        if let (Some(p1m), Some(p2m)) = (self.p1_move, self.p2_move) {
            let outcome = match (p1m, p2m) {
                (u, b) if u == b => RoundOutcome::Tie,
                (Move::Rock, Move::Scissors)
                | (Move::Paper, Move::Rock)
                | (Move::Scissors, Move::Paper) => {
                    self.scores.p1 += 1;
                    RoundOutcome::Winner(self.player1.id)
                }
                _ => {
                    self.scores.p2 += 1;
                    RoundOutcome::Winner(self.player2.id)
                }
            };

            self.history.push(RoundRecord {
                p1_move: p1m,
                p2_move: p2m,
                outcome,
            });

            // Prepare for next round immediately after processing
            self.p1_move = None;
            self.p2_move = None;
            self.round += 1;
        }
    }
}
