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
pub const SAGA_PREVIEW_PREFIX: &str = "saga_preview_"; // followed by node id
pub const SAGA_TUTORIAL_HIRE: &str = "saga_tutorial_hire";
pub const SAGA_TUTORIAL_SKIP: &str = "saga_tutorial_skip";

// Saga Tavern actions and prefixes
pub const SAGA_TAVERN_HOME: &str = "saga_tavern_home";
pub const SAGA_TAVERN_REROLL: &str = "saga_tavern_reroll";
pub const SAGA_TAVERN_GOODS: &str = "saga_tavern_goods";
pub const SAGA_TAVERN_GAMES: &str = "saga_tavern_games";
pub const SAGA_TAVERN_GAMES_BLACKJACK: &str = "saga_tavern_games_blackjack";
pub const SAGA_TAVERN_GAMES_POKER: &str = "saga_tavern_games_poker";
pub const SAGA_TAVERN_QUESTS: &str = "saga_tavern_quests";
pub const SAGA_TAVERN_SHOP: &str = "saga_tavern_shop";
pub const SAGA_TAVERN_GAMES_ARM: &str = "saga_tavern_games_arm";
pub const SAGA_TAVERN_GAMES_DARTS: &str = "saga_tavern_games_darts";
pub const SAGA_TAVERN_GAMES_PLAY_PREFIX: &str = "saga_tavern_games_play_"; // followed by game + _ + unit id
pub const SAGA_TAVERN_GAMES_ANTE_PREFIX: &str = "saga_tavern_games_ante_"; // followed by game + _ + amount
pub const SAGA_TAVERN_GAMES_ANTE_CANCEL: &str = "saga_tavern_games_ante_cancel";
pub const SAGA_TAVERN_BUY_PREFIX: &str = "saga_tavern_buy_"; // followed by item id
pub const SAGA_TAVERN_SHOP_BUY_PREFIX: &str = "saga_tavern_shop_buy_"; // followed by item id
pub const SAGA_TAVERN_SHOP_BUY_CONFIRM_PREFIX: &str = "saga_tavern_shop_buy_confirm_"; // followed by item id
pub const SAGA_TAVERN_SHOP_BUY_CANCEL: &str = "saga_tavern_shop_buy_cancel";
pub const SAGA_TAVERN_USE_PREFIX: &str = "saga_tavern_use_"; // followed by item id
pub const SAGA_TAVERN_BUY_CONFIRM_PREFIX: &str = "saga_tavern_buy_confirm_"; // followed by item id
pub const SAGA_TAVERN_BUY_CANCEL: &str = "saga_tavern_buy_cancel";
pub const SAGA_TAVERN_CANCEL: &str = "saga_tavern_cancel";
pub const SAGA_TAVERN_REROLL_CONFIRM: &str = "saga_tavern_reroll_confirm";
pub const SAGA_TAVERN_REROLL_CANCEL: &str = "saga_tavern_reroll_cancel";

// Saga hire prefix (recruitment)
pub const SAGA_HIRE_PREFIX: &str = "saga_hire_"; // followed by unit id
pub const SAGA_HIRE_CONFIRM_PREFIX: &str = "saga_hire_confirm_"; // followed by unit id
pub const SAGA_HIRE_CANCEL: &str = "saga_hire_cancel";

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

pub fn is_saga_preview(id: &str) -> bool {
    id.starts_with(SAGA_PREVIEW_PREFIX)
}

/// Parse an ante selection custom_id into (game_key, amount).
/// Expected form: `saga_tavern_games_ante_<game>_<amount>`.
pub fn parse_tavern_ante_id(id: &str) -> Option<(String, i64)> {
    if !id.starts_with(SAGA_TAVERN_GAMES_ANTE_PREFIX) {
        return None;
    }
    let rest = &id[SAGA_TAVERN_GAMES_ANTE_PREFIX.len()..];
    // Support future game keys that might contain underscores by splitting from the right.
    let (game_key, amount_str) = rest.rsplit_once('_')?;

    let amount = amount_str.parse::<i64>().ok()?;
    if game_key.is_empty() {
        return None;
    }
    Some((game_key.to_string(), amount))
}
