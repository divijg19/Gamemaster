//! Contains the core "business logic" for the Gamemaster Saga.

use crate::database::models::SagaProfile;
use chrono::{Duration, Utc};

// Constants for game balance.
const TP_REPLENISH_HOURS: i64 = 1; // Replenish 1 TP every hour.

/// Calculates the current number of Training Points a player should have based on
/// how much time has passed since the last update.
///
/// Returns a tuple of `(new_current_tp, needs_database_update)`.
pub fn calculate_tp_recharge(saga_profile: &SagaProfile) -> (i32, bool) {
    let now = Utc::now();
    let time_since_last_update = now - saga_profile.last_tp_update;

    // If not enough time has passed to gain a single point, do nothing.
    if time_since_last_update < Duration::hours(TP_REPLENISH_HOURS) {
        return (saga_profile.current_tp, false);
    }

    // Calculate how many points should have been generated.
    let points_to_add = (time_since_last_update.num_hours() / TP_REPLENISH_HOURS) as i32;
    if points_to_add <= 0 {
        return (saga_profile.current_tp, false);
    }

    // Add the points, ensuring it doesn't exceed the player's maximum.
    let new_tp = (saga_profile.current_tp + points_to_add).min(saga_profile.max_tp);

    // Only flag for an update if the value has actually changed.
    let needs_update = new_tp != saga_profile.current_tp;

    (new_tp, needs_update)
}
