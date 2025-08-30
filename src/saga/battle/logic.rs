//! Contains the core, stateful logic for processing battle turns.

use super::state::{BattleOutcome, BattleParty, BattleSession};
use rand::prelude::IteratorRandom;

/// Processes the player's entire party's turn.
pub fn process_player_turn(session: &mut BattleSession) -> BattleOutcome {
    session.log.push("--- **Your Turn** ---".to_string());
    let mut rng = rand::rng();

    for i in 0..session.player_party.len() {
        if session.player_party[i].current_hp <= 0 {
            continue;
        }

        // (✓) FIXED: Choose directly from the iterator for better performance.
        if let Some((target_idx, _)) = session
            .enemy_party
            .iter()
            .enumerate()
            .filter(|(_, e)| e.current_hp > 0)
            .choose(&mut rng)
        {
            let attacker = &session.player_party[i];
            let defender = &mut session.enemy_party[target_idx];

            let damage = (attacker.attack - defender.defense).max(1);
            defender.current_hp = (defender.current_hp - damage).max(0);

            session.log.push(format!(
                "⚔️ **{}** attacks **{}** for `{}` damage!",
                attacker.name, defender.name, damage
            ));
            if defender.current_hp == 0 {
                session
                    .log
                    .push(format!("☠️ **{}** has been defeated!", defender.name));
            }
        }
    }

    if session.enemy_party.iter().all(|e| e.current_hp <= 0) {
        return BattleOutcome::PlayerVictory;
    }

    session.current_turn = BattleParty::Enemy;
    BattleOutcome::Ongoing
}

/// Processes the enemy's entire party's turn.
pub fn process_enemy_turn(session: &mut BattleSession) -> BattleOutcome {
    session.log.push("--- **Enemy's Turn** ---".to_string());
    let mut rng = rand::rng();

    for i in 0..session.enemy_party.len() {
        if session.enemy_party[i].current_hp <= 0 {
            continue;
        }

        if let Some((target_idx, _)) = session
            .player_party
            .iter()
            .enumerate()
            .filter(|(_, p)| p.current_hp > 0)
            .choose(&mut rng)
        {
            let attacker = &session.enemy_party[i];
            let defender = &mut session.player_party[target_idx];

            let damage = (attacker.attack - defender.defense).max(1);
            defender.current_hp = (defender.current_hp - damage).max(0);

            session.log.push(format!(
                "💥 **{}** attacks **{}** for `{}` damage!",
                attacker.name, defender.name, damage
            ));
            if defender.current_hp == 0 {
                session
                    .log
                    .push(format!("☠️ **{}** has been defeated!", defender.name));
            }
        }
    }

    if session.player_party.iter().all(|p| p.current_hp <= 0) {
        return BattleOutcome::PlayerDefeat;
    }

    session.current_turn = BattleParty::Player;
    BattleOutcome::Ongoing
}
