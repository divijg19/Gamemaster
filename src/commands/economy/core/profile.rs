//! Manages the business logic for user profiles, such as leveling and work streaks.
//! This module acts as the "rules engine" for the economy.

use crate::database;
use chrono::{Duration, Utc};

/// Calculates the total experience points required to reach the next level from the current one.
///
/// # Arguments
/// * `level` - The current level.
///
/// # Returns
/// The total XP needed to advance to `level + 1`.
pub fn xp_for_level(level: i32) -> i64 {
    // Uses a power curve to make higher levels require significantly more XP.
    (100.0 * (level as f64).powf(1.5)).round() as i64
}

/// Checks the user's work history to determine and update their daily work streak.
///
/// This function mutates the `work_streak` field of the provided profile in memory.
///
/// # Arguments
/// * `profile` - A mutable reference to the user's database profile.
///
/// # Returns
/// The new, updated streak count.
pub fn check_and_update_streak(profile: &mut database::models::Profile) -> i32 {
    let today = Utc::now().date_naive();

    // Determine the new streak based on the last work date.
    let new_streak = if let Some(last_work_day) = profile.last_work.map(|lw| lw.date_naive()) {
        if today == last_work_day + Duration::days(1) {
            profile.work_streak + 1 // Consecutive day, increment streak.
        } else if today > last_work_day {
            1 // A day was missed, reset streak.
        } else {
            profile.work_streak // Worked again on the same day, no change.
        }
    } else {
        1 // This is the user's first time working.
    };

    profile.work_streak = new_streak;
    new_streak
}

/// Processes XP gain to determine if a user has leveled up.
///
/// This function can handle multiple level-ups from a single XP gain.
///
/// # Arguments
/// * `current_level` - The user's current level for a specific job.
/// * `current_xp` - The user's current XP towards the next level.
/// * `xp_gain` - The amount of new XP being added.
///
/// # Returns
/// A tuple containing:
/// 1. `(i32)`: The user's new level after the update.
/// 2. `(i64)`: The user's new XP total (as progress towards the next level).
/// 3. `Option<(i32, i64)>`: If a level-up occurred, this contains the `(new_level, xp_for_next_level)`.
pub fn handle_leveling(
    current_level: i32,
    current_xp: i64,
    xp_gain: i64,
) -> (i32, i64, Option<(i32, i64)>) {
    let mut new_level = current_level;
    let mut new_total_xp = current_xp + xp_gain;
    let mut xp_for_next = xp_for_level(new_level + 1);
    let mut level_up_info = None;

    // Loop to handle multiple level-ups from a single, large XP gain.
    while new_total_xp >= xp_for_next {
        new_level += 1;
        new_total_xp -= xp_for_next;
        xp_for_next = xp_for_level(new_level + 1);
        level_up_info = Some((new_level, xp_for_next));
    }

    (new_level, new_total_xp, level_up_info)
}
