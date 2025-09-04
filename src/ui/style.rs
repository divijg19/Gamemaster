//! Central UI style constants and helpers.
pub const COLOR_SAGA_MAIN: u32 = 0x9B59B6; // Purple
pub const COLOR_SAGA_MAP: u32 = 0x2ECC71; // Green
pub const COLOR_SAGA_TAVERN: u32 = 0xCD7F32; // Bronze
pub const COLOR_SAGA_TUTORIAL: u32 = 0x3498DB; // Blue
pub const COLOR_ALERT: u32 = 0xE74C3C; // Red
pub const COLOR_SUCCESS: u32 = 0x2ECC71; // Green reuse

pub const EMOJI_AP: &str = "⚔️";
pub const EMOJI_TP: &str = "⚡";
pub const EMOJI_REFRESH: &str = "🔄";
pub const EMOJI_BACK: &str = "⬅";
pub const EMOJI_COIN: &str = "💰";

pub fn stat_pair(current: i32, max: i32) -> String {
    format!("`{}/{}`", current, max)
}

use serenity::builder::CreateEmbed;

/// Convenience builder for a success-styled embed.
pub fn success_embed<T: Into<String>, U: Into<String>>(title: T, description: U) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .description(description)
        .color(COLOR_SUCCESS)
}

/// Convenience builder for an alert/error-styled embed.
pub fn error_embed<T: Into<String>, U: Into<String>>(title: T, description: U) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .description(description)
        .color(COLOR_ALERT)
}
