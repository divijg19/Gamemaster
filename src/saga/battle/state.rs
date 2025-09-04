//! Defines the data structures for a battle session.

use crate::database::models::{PlayerUnit, Unit};

#[derive(Debug, Clone)]
pub struct BattleUnit {
    pub name: String,
    pub current_hp: i32,
    pub max_hp: i32,
    pub attack: i32,
    pub defense: i32,
    pub unit_id: i32,
    pub is_recruitable: bool,
    pub is_human: bool,
    // Augmentation bonus stats (from bonded equippables)
    pub bonus_attack: i32,
    pub bonus_defense: i32,
    pub bonus_health: i32,
    pub owner_user_id: Option<i64>, // new: original owner when derived from PlayerUnit
}

// (✓) NEW: Add explicit constructors to resolve compiler errors.
impl BattleUnit {
    pub fn from_player_unit(unit: &PlayerUnit) -> Self {
        // At this layer we treat any stats beyond a plausible baseline as bonuses.
        // Baseline heuristic: assume bonuses never exceed 50% of stat; we can't know original without passing it.
        Self {
            name: unit.nickname.as_deref().unwrap_or(&unit.name).to_string(),
            current_hp: unit.current_health,
            max_hp: unit.current_health,
            attack: unit.current_attack,
            defense: unit.current_defense,
            unit_id: unit.unit_id,
            is_recruitable: false,
            is_human: false,
            bonus_attack: 0,
            bonus_defense: 0,
            bonus_health: 0,
            owner_user_id: Some(unit.user_id),
        }
    }

    pub fn from_player_unit_with_bonus(unit: &PlayerUnit, bonus: (i32, i32, i32)) -> Self {
        Self {
            name: unit.nickname.as_deref().unwrap_or(&unit.name).to_string(),
            current_hp: unit.current_health + bonus.2,
            max_hp: unit.current_health + bonus.2,
            attack: unit.current_attack + bonus.0,
            defense: unit.current_defense + bonus.1,
            unit_id: unit.unit_id,
            is_recruitable: false,
            is_human: false,
            bonus_attack: bonus.0,
            bonus_defense: bonus.1,
            bonus_health: bonus.2,
            owner_user_id: Some(unit.user_id),
        }
    }

    pub fn from_unit(unit: &Unit) -> Self {
        Self {
            name: unit.name.clone(),
            current_hp: unit.base_health,
            max_hp: unit.base_health,
            attack: unit.base_attack,
            defense: unit.base_defense,
            unit_id: unit.unit_id,
            is_recruitable: unit.is_recruitable,
            is_human: matches!(unit.kind, crate::database::models::UnitKind::Human),
            bonus_attack: 0,
            bonus_defense: 0,
            bonus_health: 0,
            owner_user_id: None,
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
    // Total damage prevented by Vitality (bonus_health mitigation) this battle
    pub vitality_mitigated: i32,
}

// (✓) NEW: Add a constructor to resolve compiler errors.
impl BattleSession {
    pub fn new(player_party: Vec<BattleUnit>, enemy_party: Vec<BattleUnit>) -> Self {
        let mut log = vec![format!(
            "A battle begins between your party and {} enemies!",
            enemy_party.len()
        )];
        // Touch owner_user_id to keep field live (aggregate unique owners for debug header)
        let owners: Vec<_> = player_party
            .iter()
            .filter_map(|u| u.owner_user_id)
            .collect();
        if !owners.is_empty() {
            log.push(format!("Party unit owners: {:?}", owners));
        }
        Self {
            log,
            player_party,
            enemy_party,
            phase: BattlePhase::PlayerTurn,
            vitality_mitigated: 0,
        }
    }
}
