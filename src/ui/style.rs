//! Central UI style constants and helpers.
pub const COLOR_SAGA_MAIN: u32 = 0x9B59B6; // Purple
pub const COLOR_SAGA_MAP: u32 = 0x2ECC71; // Green
pub const COLOR_SAGA_TAVERN: u32 = 0xCD7F32; // Bronze
pub const COLOR_SAGA_TUTORIAL: u32 = 0x3498DB; // Blue
pub const COLOR_ALERT: u32 = 0xE74C3C; // Red

pub const EMOJI_AP: &str = "⚔️";
pub const EMOJI_TP: &str = "⚡";
pub const EMOJI_REFRESH: &str = "🔄";
pub const EMOJI_BACK: &str = "⬅";
pub const EMOJI_COIN: &str = "💰";

// Standard target widths for padded button labels (approx char counts before Discord trimming)
pub const BTN_W_NARROW: usize = 12; // short actions (Rock, Fold, Claim)
pub const BTN_W_STD: usize = 16; // common secondary buttons (Refresh, Research)
pub const BTN_W_PRIMARY: usize = 22; // primary saga/world/nav actions

pub fn stat_pair(current: i32, max: i32) -> String {
    format!("`{}/{}`", current, max)
}

/// Pads a label to a target visible width using spaces so multi-row action bars align better.
/// Discord strips excessive trailing spaces at end of entire component row but preserves some internal padding.
/// We keep this conservative (max pad 2) to avoid discord collapsing them entirely.
pub fn pad_label(label: &str, target_min: usize) -> String {
    let len = label.chars().count();
    if len >= target_min {
        return label.to_string();
    }
    // Provide trailing spaces but clamp to 2 to avoid Discord trimming collapse.
    format!("{label}{pad}", pad = " ".repeat((target_min - len).min(2)))
}

/// Convenience wrapper picking a standard category width.
pub fn pad_primary(label: &str) -> String {
    pad_label(label, BTN_W_PRIMARY)
}
pub fn pad_std(label: &str) -> String {
    pad_label(label, BTN_W_STD)
}
pub fn pad_narrow(label: &str) -> String {
    pad_label(label, BTN_W_NARROW)
}

use serenity::builder::CreateEmbed;

/// Convenience builder for an alert/error-styled embed.
pub fn error_embed<T: Into<String>, U: Into<String>>(title: T, description: U) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .description(description)
        .color(COLOR_ALERT)
}
