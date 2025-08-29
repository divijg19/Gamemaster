//! Defines the state for an active shop session.

use crate::commands::economy::core::item::ItemCategory;

pub struct ShopSession {
    pub user_id: u64,
    pub current_category: ItemCategory,
    pub current_page: usize,
}
