//! This module contains a standard 52-card playing deck.

use super::card::{Card, Rank, Suit};
use rand::rng;
use rand::seq::SliceRandom; // (✓) MODIFIED: Use the correct import for the thread-local RNG.

pub struct Deck {
    // (✓) MODIFIED: The internal card vector is now private again to ensure
    // other modules must use the public methods, respecting encapsulation.
    cards: Vec<Card>,
}

impl Deck {
    /// Creates a new, standard 52-card deck.
    pub fn new() -> Self {
        let mut cards = Vec::with_capacity(52);
        let suits = [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades];
        let ranks = [
            Rank::Two,
            Rank::Three,
            Rank::Four,
            Rank::Five,
            Rank::Six,
            Rank::Seven,
            Rank::Eight,
            Rank::Nine,
            Rank::Ten,
            Rank::Jack,
            Rank::Queen,
            Rank::King,
            Rank::Ace,
        ];

        for &suit in &suits {
            for &rank in &ranks {
                cards.push(Card { suit, rank });
            }
        }
        Deck { cards }
    }

    /// Shuffles the deck randomly.
    pub fn shuffle(&mut self) {
        // (✓) MODIFIED: Use the modern and correct `thread_rng()` function.
        self.cards.shuffle(&mut rng());
    }

    /// Deals one card from the top of the deck.
    /// Returns `None` if the deck is empty.
    pub fn deal_one(&mut self) -> Option<Card> {
        self.cards.pop()
    }

    /// A public method to safely check the number of cards remaining.
    pub fn cards_remaining(&self) -> usize {
        self.cards.len()
    }
}
