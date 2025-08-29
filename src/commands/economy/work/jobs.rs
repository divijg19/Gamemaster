//! Defines all available jobs and their unique properties.

use crate::commands::economy::core::item::Item;
use chrono::Duration;
// (✓) FIXED: Use the correct imports for your version of the `rand` crate.
use rand::{Rng, rng};

pub struct Job {
    pub name: &'static str,
    pub display_name: &'static str,
    pub min_payout: i64,
    pub max_payout: i64,
    pub cooldown: Duration,
    pub xp_gain: i64,
    pub resource: fn(level: i32) -> (Item, i64),
    pub rare_reward: Option<(Item, f64)>,
}

pub const JOBS: &[Job] = &[
    Job {
        name: "fishing",
        display_name: "Fishing",
        min_payout: 25,
        max_payout: 75,
        cooldown: Duration::minutes(30),
        xp_gain: 10,
        // (✓) FINAL FIX: Use the correct `rng()` and `random_range()` for your crate version.
        resource: |level| (Item::Fish, rng().random_range(3..=8) + (level as i64 / 2)),
        rare_reward: Some((Item::GoldenFish, 0.05)),
    },
    Job {
        name: "mining",
        display_name: "Mining",
        min_payout: 100,
        max_payout: 300,
        cooldown: Duration::hours(2),
        xp_gain: 25,
        resource: |level| (Item::Ore, rng().random_range(5..=15) + (level as i64)),
        rare_reward: Some((Item::LargeGeode, 0.02)),
    },
    Job {
        name: "coding",
        display_name: "Coding",
        min_payout: 400,
        max_payout: 800,
        cooldown: Duration::hours(8),
        xp_gain: 100,
        resource: |level| (Item::Gem, rng().random_range(1..=3) + (level as i64 / 5)),
        rare_reward: None,
    },
];
