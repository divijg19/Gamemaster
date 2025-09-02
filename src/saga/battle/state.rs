//! Defines the data structures for a battle session.

use crate::database::models::{Pet, PlayerPet};

#[derive(Debug, Clone)]
pub struct BattleUnit {
    pub name: String,
    pub current_hp: i32,
    pub max_hp: i32,
    pub attack: i32,
    pub defense: i32,
    pub pet_id: i32,
    pub is_tameable: bool,
}

// (✓) NEW: Add explicit constructors to resolve compiler errors.
impl BattleUnit {
    pub fn from_player_pet(pet: &PlayerPet) -> Self {
        Self {
            name: pet.nickname.as_deref().unwrap_or(&pet.name).to_string(),
            current_hp: pet.current_health,
            max_hp: pet.current_health,
            attack: pet.current_attack,
            defense: pet.current_defense,
            pet_id: pet.pet_id,
            is_tameable: false, // Players' pets can't be tamed.
        }
    }

    pub fn from_pet(pet: &Pet) -> Self {
        Self {
            name: pet.name.clone(),
            current_hp: pet.base_health,
            max_hp: pet.base_health,
            attack: pet.base_attack,
            defense: pet.base_defense,
            pet_id: pet.pet_id,
            is_tameable: pet.is_tameable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlePhase {
    PlayerTurn,
    PlayerSelectingItem,
    EnemyTurn,
    Victory,
    Defeat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleOutcome {
    PlayerVictory,
    PlayerDefeat,
    Ongoing,
}

#[derive(Debug, Clone)]
pub struct BattleSession {
    pub player_party: Vec<BattleUnit>,
    pub enemy_party: Vec<BattleUnit>,
    pub phase: BattlePhase,
    pub log: Vec<String>,
}

// (✓) NEW: Add a constructor to resolve compiler errors.
impl BattleSession {
    pub fn new(player_party: Vec<BattleUnit>, enemy_party: Vec<BattleUnit>) -> Self {
        Self {
            log: vec![format!(
                "A battle begins between your party and {} enemies!",
                enemy_party.len()
            )],
            player_party,
            enemy_party,
            phase: BattlePhase::PlayerTurn,
        }
    }
}
