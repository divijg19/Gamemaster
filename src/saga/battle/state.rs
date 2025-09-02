//! Defines the data structures for a battle session.

use crate::database::models::{Pet as Unit, PlayerPet as PlayerUnit};

#[derive(Debug, Clone)]
pub struct BattleUnit {
    pub name: String,
    pub current_hp: i32,
    pub max_hp: i32,
    pub attack: i32,
    pub defense: i32,
    pub unit_id: i32,
    pub is_recruitable: bool,
}

// (✓) NEW: Add explicit constructors to resolve compiler errors.
impl BattleUnit {
    pub fn from_player_pet(unit: &PlayerUnit) -> Self {
        Self {
        name: unit.nickname.as_deref().unwrap_or(&unit.name).to_string(),
        current_hp: unit.current_health,
        max_hp: unit.current_health,
        attack: unit.current_attack,
        defense: unit.current_defense,
        unit_id: unit.pet_id,
        is_recruitable: false, // Players' units can't be recruited.
        }
    }

    pub fn from_pet(unit: &Unit) -> Self {
        Self {
        name: unit.name.clone(),
        current_hp: unit.base_health,
        max_hp: unit.base_health,
        attack: unit.base_attack,
        defense: unit.base_defense,
        unit_id: unit.pet_id,
        is_recruitable: unit.is_tameable,
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
