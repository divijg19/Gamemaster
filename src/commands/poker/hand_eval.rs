//! Contains the logic for evaluating a 5-card poker hand.

use super::state::HandRank;
use crate::commands::games::card::{Card, Rank};
use std::collections::HashMap;

/// Takes a slice of 5 cards and returns the best possible HandRank.
pub fn evaluate_hand(hand: &[Card]) -> HandRank {
    let mut sorted_hand = hand.to_vec();
    sorted_hand.sort_by(|a, b| b.rank.cmp(&a.rank));

    let is_flush = sorted_hand.windows(2).all(|w| w[0].suit == w[1].suit);
    let (is_straight, high_card) = is_straight(&sorted_hand);

    if is_straight && is_flush {
        if high_card == Rank::Ace as u8 {
            return HandRank::RoyalFlush;
        }
        return HandRank::StraightFlush(high_card);
    }

    let rank_counts = count_ranks(&sorted_hand);
    let mut pairs = Vec::new();
    let mut threes = Vec::new();
    let mut fours = Vec::new();

    for (rank, count) in rank_counts {
        match count {
            2 => pairs.push(rank as u8),
            3 => threes.push(rank as u8),
            4 => fours.push(rank as u8),
            _ => {}
        }
    }

    pairs.sort_by(|a, b| b.cmp(a));
    threes.sort_by(|a, b| b.cmp(a));

    if let Some(&four) = fours.first() {
        return HandRank::FourOfAKind(four);
    }
    if let (Some(&three), Some(&pair)) = (threes.first(), pairs.first()) {
        return HandRank::FullHouse(three, pair);
    }
    if is_flush {
        return HandRank::Flush(sorted_hand[0].rank as u8);
    }
    if is_straight {
        return HandRank::Straight(high_card);
    }
    if let Some(&three) = threes.first() {
        return HandRank::ThreeOfAKind(three);
    }
    if pairs.len() >= 2 {
        return HandRank::TwoPair(pairs[0], pairs[1]);
    }
    if let Some(&pair) = pairs.first() {
        return HandRank::OnePair(pair);
    }

    HandRank::HighCard(sorted_hand[0].rank as u8)
}

/// Helper to count occurrences of each rank.
fn count_ranks(hand: &[Card]) -> HashMap<Rank, usize> {
    let mut counts = HashMap::new();
    for card in hand {
        *counts.entry(card.rank).or_insert(0) += 1;
    }
    counts
}

/// Helper to check for a straight. Returns (is_straight, high_card_value).
fn is_straight(sorted_hand: &[Card]) -> (bool, u8) {
    // Ace-low straight check (A, 5, 4, 3, 2)
    let is_ace_low = sorted_hand[0].rank == Rank::Ace
        && sorted_hand[1].rank == Rank::Five
        && sorted_hand[2].rank == Rank::Four
        && sorted_hand[3].rank == Rank::Three
        && sorted_hand[4].rank == Rank::Two;
    if is_ace_low {
        return (true, Rank::Five as u8);
    }

    // Standard straight check
    let is_standard = sorted_hand
        .windows(2)
        .all(|w| w[0].rank as u8 == w[1].rank as u8 + 1);
    (is_standard, sorted_hand[0].rank as u8)
}
