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
    // UI/UX REFINEMENT: Updated emojis to exactly match the user's specified look.
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

#[derive(Clone, Copy, Debug)]
pub struct Scores {
    pub p1: u32,
    pub p2: u32,
}

#[derive(Clone, Debug)]
pub struct RoundRecord {
    pub p1_move: Move,
    pub p2_move: Move,
    pub outcome: RoundOutcome,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RoundOutcome {
    Tie,
    Winner(UserId),
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

            self.p1_move = None;
            self.p2_move = None;
            self.round += 1;
        }
    }
}
