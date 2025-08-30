//! Contains the business logic for map and story progression.

/// Takes the player's current story progress and returns a list of
/// map node IDs that should be visible to them.
pub fn get_available_nodes(story_progress: i32) -> Vec<i32> {
    match story_progress {
        0 => vec![1], // At the start, only the "Forest Entrance" is available.
        1 => vec![2], // After beating the first node, the "Shaded Grove" becomes available.
        // As you add more story, you'll add more cases here.
        _ => vec![], // No more nodes available for now.
    }
}
