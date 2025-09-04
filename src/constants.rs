// Central constants for caching and limits.
pub const EQUIP_BONUS_CACHE_TTL_SECS: u64 = 5; // previously hard-coded in saga_handler
pub const BOND_MAP_CACHE_TTL_SECS: u64 = 10; // cache lifetime for bonded mapping in party view
pub const MAX_PARTY_SIZE: i64 = 5;
pub const MAX_ARMY_SIZE: i64 = 10;
// Feature flags / toggles (runtime constants). Flip to false during balancing sessions
// to allow drafting human contracts without parchment consumption.
pub const ENABLE_PARCHMENT_GATING: bool = true;

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
