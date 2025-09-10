// Central constants for caching and limits.
pub const EQUIP_BONUS_CACHE_TTL_SECS: u64 = 5; // previously hard-coded in saga_handler
pub const BOND_MAP_CACHE_TTL_SECS: u64 = 10; // cache lifetime for bonded mapping in party view
pub const MAX_PARTY_SIZE: i64 = 5;
pub const MAX_ARMY_SIZE: i64 = 10;
// Feature flags / toggles (runtime constants). Flip to false during balancing sessions
// to allow drafting human contracts without parchment consumption.
pub const ENABLE_PARCHMENT_GATING: bool = true;

// Duration (in seconds) that a Focus Tonic buff remains active for a user.
pub const FOCUS_TONIC_TTL_SECS: u64 = 15 * 60; // 15 minutes
/// Multiplier applied to research drop chances while Focus Tonic is active.
pub const FOCUS_TONIC_BONUS_MULT: f64 = 1.25; // +25%

use crate::database::models::UnitRarity;
/// Return a short emoji/icon for a given rarity.
pub fn rarity_icon(r: UnitRarity) -> &'static str {
    use UnitRarity::*;
    match r {
        Common => "⚪",
        Rare => "🟢",
        Epic => "🔵",
        Legendary => "🟣",
        Unique => "🟡",
        Mythical => "🔴",
        Fabled => "🔷",
    }
}
