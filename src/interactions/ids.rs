//! Centralized custom_id string constants for interaction components.
//! Consolidating here reduces typos and enables future refactors (renaming / prefix changes).

// Saga core actions
pub const SAGA_MAP: &str = "saga_map";
pub const SAGA_MAP_LOCKED: &str = "saga_map_locked"; // disabled placeholder when no party
pub const SAGA_TAVERN: &str = "saga_tavern";
pub const SAGA_RECRUIT: &str = "saga_recruit";
pub const SAGA_BACK: &str = "saga_back";
pub const SAGA_REFRESH: &str = "saga_refresh";
pub const SAGA_NODE_PREFIX: &str = "saga_node_"; // followed by node id
pub const SAGA_AREA_PREFIX: &str = "saga_area_"; // followed by area id
pub const SAGA_TUTORIAL_HIRE: &str = "saga_tutorial_hire";
pub const SAGA_TUTORIAL_SKIP: &str = "saga_tutorial_skip";

// Global nav bar ids
pub const NAV_SAGA: &str = "nav_saga";
pub const NAV_PARTY: &str = "nav_party";
pub const NAV_TRAIN: &str = "nav_train";

// Utility predicates
pub fn is_saga_node(id: &str) -> bool {
    id.starts_with(SAGA_NODE_PREFIX)
}

pub fn is_saga_area(id: &str) -> bool {
    id.starts_with(SAGA_AREA_PREFIX)
}
