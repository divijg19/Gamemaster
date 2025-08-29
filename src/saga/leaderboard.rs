//! Contains the definitions and logic for different leaderboard types.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaderboardType {
    Gamemaster,
    Wealth,
    WorkStreak,
}

impl LeaderboardType {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Gamemaster => "🏆 Gamemaster Score",
            Self::Wealth => "💰 Wealth",
            Self::WorkStreak => "📈 Work Streak",
        }
    }

    pub fn score_name(&self) -> &'static str {
        match self {
            Self::Gamemaster => "Score",
            Self::Wealth => "Coins",
            Self::WorkStreak => "Days",
        }
    }
}
