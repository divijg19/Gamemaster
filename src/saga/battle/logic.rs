//! Contains the core, stateful logic for processing battle turns.

use super::state::{BattleOutcome, BattlePhase, BattleSession, BattleUnit};
use rand::prelude::IteratorRandom;
use rand::rng;

// (✓) NEW: A private helper function to handle the core attack logic for any party.
// This eliminates code duplication between the player and enemy turn functions.
fn process_party_attack(
    attacking_party: &[BattleUnit],
    defending_party: &mut [BattleUnit],
    log: &mut Vec<String>,
    attack_message: &str,
    vitality_accumulator: &mut i32,
) {
    // Use thread-local RNG each call; scope-limited so no Send issues.
    let mut rng = rng();
    for attacker in attacking_party.iter().filter(|a| a.current_hp > 0) {
        // Find a random, living target in the defending party.
        if let Some(target_idx) = defending_party
            .iter()
            .enumerate()
            .filter(|(_, d)| d.current_hp > 0)
            .map(|(i, _)| i)
            .choose(&mut rng)
        {
            let defender = &mut defending_party[target_idx];
            let effective_attack = attacker.attack + attacker.bonus_attack;
            let mut effective_defense = defender.defense + defender.bonus_defense;
            // NEW: Leverage bonus_health as a minor mitigation factor (each 10 bonus HP = +1 defense virtual).
            let mut mitigated = 0;
            if defender.bonus_health > 0 {
                let extra = (defender.bonus_health / 10).max(1);
                effective_defense += extra;
                mitigated = extra;
            }
            let damage = (effective_attack - effective_defense).max(1);
            defender.current_hp = (defender.current_hp - damage).max(0);

            if mitigated > 0 {
                log.push(format!(
                    "{} **{}** attacks **{}** for `{}` damage ({} mitigated by Vitality)!",
                    attack_message, attacker.name, defender.name, damage, mitigated
                ));
                *vitality_accumulator += mitigated;
            } else {
                log.push(format!(
                    "{} **{}** attacks **{}** for `{}` damage!",
                    attack_message, attacker.name, defender.name, damage
                ));
            }

            if defender.current_hp == 0 {
                log.push(format!("☠️ **{}** has been defeated!", defender.name));
            }
        }
    }
}

/// Processes the player's entire party's turn.
pub fn process_player_turn(session: &mut BattleSession) -> BattleOutcome {
    session.log.push("--- **Your Turn** ---".to_string());

    // (✓) MODIFIED: All core logic is now in the shared helper function.
    process_party_attack(
        &session.player_party.clone(),
        &mut session.enemy_party,
        &mut session.log,
        "⚔️",
        &mut session.vitality_mitigated,
    );

    if session.enemy_party.iter().all(|e| e.current_hp <= 0) {
        return BattleOutcome::PlayerVictory;
    }

    session.phase = BattlePhase::EnemyTurn;
    BattleOutcome::Ongoing
}

/// Processes the enemy's entire party's turn.
pub fn process_enemy_turn(session: &mut BattleSession) -> BattleOutcome {
    session.log.push("--- **Enemy's Turn** ---".to_string());

    // (✓) MODIFIED: All core logic is now in the shared helper function.
    process_party_attack(
        &session.enemy_party.clone(),
        &mut session.player_party,
        &mut session.log,
        "💥",
        &mut session.vitality_mitigated,
    );

    if session.player_party.iter().all(|p| p.current_hp <= 0) {
        return BattleOutcome::PlayerDefeat;
    }

    session.phase = BattlePhase::PlayerTurn;
    BattleOutcome::Ongoing
}
