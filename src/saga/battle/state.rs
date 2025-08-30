//! Defines the data structures for a battle session.

use crate::database::profile::{Pet, PlayerPet};

#[derive(Debug, Clone)]
pub struct BattleUnit {
    pub name: String,
    pub current_hp: i32,
    pub max_hp: i32,
    pub attack: i32,
    pub defense: i32,
}

impl From<&PlayerPet> for BattleUnit {
    fn from(pet: &PlayerPet) -> Self {
        Self {
            name: pet.nickname.as_deref().unwrap_or(&pet.name).to_string(),
            current_hp: pet.current_health,
            max_hp: pet.current_health,
            attack: pet.current_attack,
            defense: pet.current_defense,
        }
    }
}

impl From<&Pet> for BattleUnit {
    fn from(pet: &Pet) -> Self {
        Self {
            name: pet.name.clone(),
            current_hp: pet.base_health,
            max_hp: pet.base_health,
            attack: pet.base_attack,
            defense: pet.base_defense,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleParty {
    Player,
    Enemy,
}

// (✓) FIXED: Added `#[derive(PartialEq)]` to allow comparison with `==`.
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
    pub current_turn: BattleParty,
    pub log: Vec<String>,
}
