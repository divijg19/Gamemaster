//! Contains all the data structures that map to database tables or query results.

use crate::commands::economy::core::item::Item;
use sqlx::Type;
use sqlx::types::chrono::{DateTime, Utc};

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Profile {
    pub balance: i64,
    pub last_work: Option<DateTime<Utc>>,
    pub work_streak: i32,
    pub fishing_xp: i64,
    pub fishing_level: i32,
    pub mining_xp: i64,
    pub mining_level: i32,
    pub coding_xp: i64,
    pub coding_level: i32,
}
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct InventoryItem {
    pub name: String,
    pub quantity: i64,
}
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct SagaProfile {
    pub current_ap: i32,
    pub max_ap: i32,
    pub current_tp: i32,
    pub max_tp: i32,
    pub last_tp_update: DateTime<Utc>,
    pub story_progress: i32,
}
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Pet {
    pub pet_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub base_attack: i32,
    pub base_defense: i32,
    pub base_health: i32,
    pub is_tameable: bool,
}
#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct PlayerPet {
    pub player_pet_id: i32,
    pub user_id: i64,
    pub pet_id: i32,
    pub nickname: Option<String>,
    pub current_level: i32,
    pub current_xp: i32,
    pub current_attack: i32,
    pub current_defense: i32,
    pub current_health: i32,
    pub is_in_party: bool,
    pub is_training: bool,
    pub training_stat: Option<String>,
    pub training_ends_at: Option<DateTime<Utc>>,
    pub name: String,
}
#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct MapNode {
    pub node_id: i32,
    pub area_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub story_progress_required: i32,
    pub reward_coins: i64,
    pub reward_pet_xp: i32,
}
// Transitional aliases while renaming "pets" to "units" in higher layers.
pub type Unit = Pet;
pub type PlayerUnit = PlayerPet;
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct NodeReward {
    pub item_id: i32,
    pub quantity: i32,
    pub drop_chance: f32,
}
#[derive(Debug, Default)]
pub struct WorkRewards {
    pub coins: i64,
    pub xp: i64,
    pub items: Vec<(Item, i64)>,
}
pub struct ProgressionUpdate {
    pub job_name: String,
    pub new_level: i32,
    pub new_xp: i64,
}
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Recipe {
    pub recipe_id: i32,
    pub output_item_id: i32,
    pub output_quantity: i32,
}
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct RecipeIngredient {
    pub item_id: i32,
    pub quantity: i32,
}

// --- Task System ---
#[derive(Debug, Clone, Copy, Type, PartialEq)]
#[sqlx(type_name = "task_type", rename_all = "PascalCase")]
pub enum TaskType {
    Daily,
    Weekly,
}

// (✓) FINAL: Acknowledging that the fields of this struct are used for DB mapping,
// but not all are read in the app logic *yet*. This is the correct final state.
#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Task {
    pub task_id: i32,
    pub task_type: TaskType,
    pub title: String,
    pub description: String,
    pub objective_key: String,
    pub objective_goal: i32,
    pub reward_coins: Option<i64>,
    pub reward_item_id: Option<i32>,
    pub reward_item_quantity: Option<i32>,
}
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct PlayerTaskDetails {
    pub player_task_id: i32,
    pub task_type: TaskType,
    pub progress: i32,
    pub is_completed: bool,
    pub title: String,
    pub description: String,
    pub objective_goal: i32,
    pub reward_coins: Option<i64>,
    pub reward_item_id: Option<i32>,
    pub reward_item_quantity: Option<i32>,
}

// --- Quest System ---
#[derive(Debug, Clone, Copy, Type, PartialEq)]
#[sqlx(type_name = "quest_type_enum", rename_all = "PascalCase")]
pub enum QuestType {
    Battle,
    Riddle,
}
#[derive(Debug, Clone, Copy, Type, PartialEq)]
#[sqlx(type_name = "player_quest_status_enum", rename_all = "PascalCase")]
pub enum PlayerQuestStatus {
    Offered,
    Accepted,
    Completed,
    Failed,
}

// (✓) FINAL: Acknowledging that these fields are for DB mapping and future use.
#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Quest {
    pub quest_id: i32,
    pub title: String,
    pub description: String,
    pub giver_name: String,
    pub difficulty: String,
    pub quest_type: QuestType,
    pub objective_key: String,
}

#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct QuestReward {
    pub quest_reward_id: i32,
    pub quest_id: i32,
    pub reward_coins: Option<i64>,
    pub reward_item_id: Option<i32>,
    pub reward_item_quantity: Option<i32>,
}

// (✓) FINAL: Acknowledging this struct is for the `/questlog` feature we are about to build.
#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct PlayerQuest {
    pub player_quest_id: i32,
    pub user_id: i64,
    pub quest_id: i32,
    pub status: PlayerQuestStatus,
    pub offered_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct QuestDetails {
    pub player_quest_id: i32,
    pub status: PlayerQuestStatus,
    pub title: String,
    pub description: String,
    pub giver_name: String,
    pub difficulty: String,
}
