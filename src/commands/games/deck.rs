//! This module contains a standard 52-card playing deck.

use super::card::{Card, Rank, Suit};
use rand::seq::SliceRandom; // (✓) MODIFIED: Use the correct import for the thread-local RNG.

pub struct Deck {
    // (✓) MODIFIED: The internal card vector is now private again to ensure
    // other modules must use the public methods, respecting encapsulation.
    cards: Vec<Card>,
}

impl Default for Deck {
    fn default() -> Self {
        Self::new()
    }
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
        self.cards.shuffle(&mut rand::rng());
    }

    /// Deals one card from the top of the deck.
    /// Returns `None` if the deck is empty.
    pub fn deal_one(&mut self) -> Option<Card> {
        self.cards.pop()
    }

    /// Deals a specified number of cards from the top of the deck.
    pub fn deal(&mut self, count: usize) -> Option<Vec<Card>> {
        if self.cards_remaining() < count {
            return None;
        }
        let mut hand = Vec::with_capacity(count);
        for _ in 0..count {
            // This unwrap is safe because we checked the length above.
            if let Some(card) = self.deal_one() {
                hand.push(card);
            } else {
                break; // deck exhausted early
            }
        }
        Some(hand)
    }

    /// A public method to safely check the number of cards remaining.
    pub fn cards_remaining(&self) -> usize {
        self.cards.len()
    }
}
