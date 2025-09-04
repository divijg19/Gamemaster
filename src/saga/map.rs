//! Contains the business logic for map and story progression.

/// Takes the player's current story progress and returns a list of
/// map node IDs that should be visible to them.
pub fn get_available_nodes(story_progress: i32) -> Vec<i32> {
    match story_progress {
        0 => vec![1],
        1 => vec![2],
        n if n >= 2 => vec![2], // Keep last unlocked node repeatable as a fallback.
        _ => vec![1],
    }
}
