//! Contains the business logic for unit (formerly pet) progression and leveling.

use crate::database::models::PlayerUnit;

const BASE_XP_PER_LEVEL: i32 = 100;

/// Calculates the XP required to reach the next level for a unit.
pub fn xp_for_unit_level(level: i32) -> i32 {
    BASE_XP_PER_LEVEL + (level * 25)
}

/// A struct to hold the results of a unit gaining XP.
pub struct LevelUpResult {
    pub new_xp: i32,
    pub new_level: i32,
    pub stat_gains: (i32, i32, i32), // (Attack, Defense, Health)
    pub did_level_up: bool,
}

/// Processes XP gain for a unit and calculates level-ups and stat gains.
pub fn handle_unit_leveling(unit: &PlayerUnit, xp_gained: i32) -> LevelUpResult {
    let mut new_xp = unit.current_xp + xp_gained;
    let mut new_level = unit.current_level;
    let mut did_level_up = false;
    let mut stat_gains = (0, 0, 0);

    let mut xp_needed = xp_for_unit_level(new_level);
    while new_xp >= xp_needed {
        new_xp -= xp_needed;
        new_level += 1;
        did_level_up = true;

        // Define stat gains per level-up
        stat_gains.0 += 2; // +2 Attack
        stat_gains.1 += 1; // +1 Defense
        stat_gains.2 += 10; // +10 Health

    xp_needed = xp_for_unit_level(new_level);
    }

    LevelUpResult {
        new_xp,
        new_level,
        stat_gains,
        did_level_up,
    }
}

// Wrappers removed post-migration.
