use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rank {
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

// (✓) ADDED: A method to get the Blackjack value(s) of a rank.
// Ace is special, returning two possible values.
impl Rank {
    pub fn value(&self) -> (u8, Option<u8>) {
        match self {
            Rank::Two => (2, None),
            Rank::Three => (3, None),
            Rank::Four => (4, None),
            Rank::Five => (5, None),
            Rank::Six => (6, None),
            Rank::Seven => (7, None),
            Rank::Eight => (8, None),
            Rank::Nine => (9, None),
            Rank::Ten | Rank::Jack | Rank::Queen | Rank::King => (10, None),
            Rank::Ace => (1, Some(11)), // Ace can be 1 or 11
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rank_str = match self.rank {
            Rank::Two => "2",
            Rank::Three => "3",
            Rank::Four => "4",
            Rank::Five => "5",
            Rank::Six => "6",
            Rank::Seven => "7",
            Rank::Eight => "8",
            Rank::Nine => "9",
            Rank::Ten => "10",
            Rank::Jack => "J",
            Rank::Queen => "Q",
            Rank::King => "K",
            Rank::Ace => "A",
        };
        let suit_char = match self.suit {
            Suit::Hearts => '♥',
            Suit::Diamonds => '♦',
            Suit::Clubs => '♣',
            Suit::Spades => '♠',
        };
        write!(f, "{}{}", rank_str, suit_char)
    }
}
