//! Battle resolution helpers extracted from saga/battle/game.rs for cleaner game loop.
use crate::commands::economy::core::item::Item;
use crate::database;
use crate::database::models::{PlayerUnit, UnitKind, UnitRarity};
use rand::Rng;
use rand::rng;
use serenity::model::id::UserId;
use sqlx::PgPool;

pub struct NodeVictoryResult {
    pub victory_log: Vec<String>,
}

/// Input bundle for resolving a node victory. Grouping avoids passing many arguments.
pub struct ResolveVictoryInput {
    pub user_id: UserId,
    pub node_id: i32,
    pub node_name: String,
    pub party_units: Vec<PlayerUnit>,
    pub vitality_mitigated: i32,
    pub enemy_unit_ids: Vec<i32>,
    pub focus_active: bool,
}

/// Chance table for research drops based on rarity.
fn research_drop_chance(rarity: UnitRarity) -> f64 {
    match rarity {
        UnitRarity::Common => 0.55,
        UnitRarity::Rare => 0.45,
        UnitRarity::Epic => 0.30,
        UnitRarity::Legendary | UnitRarity::Unique | UnitRarity::Mythical | UnitRarity::Fabled => {
            0.0
        }
    }
}

/// Compute rewards, dynamic research additions, human defeat tracking, apply payouts, and return assembled log lines.
pub async fn resolve_node_victory(
    db: &PgPool,
    input: ResolveVictoryInput,
) -> Result<NodeVictoryResult, String> {
    // Fetch node core data
    let node = database::world::get_map_nodes_by_ids(db, &[input.node_id])
        .await
        .map_err(|_| "Node lookup failed")?
        .into_iter()
        .next()
        .ok_or_else(|| "Node not found".to_string())?;
    let rewards = database::world::get_rewards_for_node(db, input.node_id)
        .await
        .map_err(|_| "Reward lookup failed")?;
    let mut dynamic_loot: Vec<(Item, i64)> = Vec::new();
    // Use thread-local RNG; confined to this async function scope (no cross-await hold) so Send issues avoided.
    {
        let mut local_rng = rng();
        for r in rewards.into_iter() {
            let roll: f64 = local_rng.random();
            if roll < r.drop_chance as f64
                && let Some(it) = Item::from_i32(r.item_id)
            {
                dynamic_loot.push((it, r.quantity as i64));
            }
        }
    }
    // Fetch enemy metas in one batch
    // Accumulate enemy rarity weights for reward scaling.
    let mut rarity_scaler: f64 = 0.0;
    let mut rarity_count: usize = 0;
    if let Ok(enemy_units) = database::units::get_units_by_ids(db, &input.enemy_unit_ids).await {
        let mut human_units: Vec<crate::database::models::Unit> = Vec::new();
        for meta in enemy_units.iter() {
            // Add to rarity scaling (all enemies contribute equally weight 1).
            rarity_scaler += match meta.rarity {
                UnitRarity::Common => 1.0,
                UnitRarity::Rare => 1.08,
                UnitRarity::Epic => 1.18,
                UnitRarity::Legendary => 1.35,
                UnitRarity::Unique => 1.55,
                UnitRarity::Mythical => 1.80,
                UnitRarity::Fabled => 2.10,
            };
            rarity_count += 1;
            if matches!(meta.kind, UnitKind::Pet)
                && !matches!(
                    meta.rarity,
                    UnitRarity::Legendary
                        | UnitRarity::Unique
                        | UnitRarity::Mythical
                        | UnitRarity::Fabled
                )
            {
                if let Some(research_item) = Item::research_item_for_unit(&meta.name) {
                    let mut chance = research_drop_chance(meta.rarity);
                    // Apply focus buff multiplier if active
                    if input.focus_active {
                        chance = (chance * crate::constants::FOCUS_TONIC_BONUS_MULT).min(0.95);
                    }
                    if chance > 0.0 {
                        let mut roll_rng = rng();
                        let roll: f64 = roll_rng.random();
                        if roll < chance {
                            dynamic_loot.push((research_item, 1));
                        }
                    }
                }
            } else if matches!(meta.kind, UnitKind::Human) {
                human_units.push(meta.clone());
            }
        }
        // Record defeats for humans after RNG loop (sequential, no RNG held across await)
        for h in human_units {
            let _ = database::human::record_human_defeat(db, input.user_id, &h).await;
        }
    }
    // Derive average rarity multiplier if any enemies processed.
    let avg_multiplier = if rarity_count > 0 {
        (rarity_scaler / rarity_count as f64).clamp(1.0, 2.25)
    } else {
        1.0
    };
    // Scale node base rewards (ensure at least 1 coin/xp when base >0).
    let scaled_coins = if node.reward_coins > 0 {
        ((node.reward_coins as f64) * avg_multiplier).round() as i64
    } else {
        0
    };
    let scaled_xp = if node.reward_unit_xp > 0 {
        ((node.reward_unit_xp as f64) * avg_multiplier).round() as i32
    } else {
        0
    };
    // Apply rewards
    let results = database::units::apply_battle_rewards(
        db,
        input.user_id,
        scaled_coins,
        &dynamic_loot,
        &input.party_units,
        scaled_xp,
    )
    .await
    .map_err(|_| "Apply rewards failed")?;
    database::saga::advance_story_progress(db, input.user_id, input.node_id)
        .await
        .ok();
    database::tasks::update_task_progress(
        db,
        input.user_id,
        &format!("WinBattle:{}", input.node_id),
        1,
    )
    .await
    .ok();
    database::tasks::update_task_progress(db, input.user_id, "WinBattle", 1)
        .await
        .ok();

    let mut log = vec![
        format!("🎉 **Victory at the {}!**", input.node_name),
        if (scaled_coins - node.reward_coins).abs() > 0 {
            format!(
                "💰 You earned **{}** coins ({} base × {:.2} rarity).",
                scaled_coins, node.reward_coins, avg_multiplier
            )
        } else {
            format!("💰 You earned **{}** coins.", scaled_coins)
        },
    ];
    // Show Focus buff note when active to explain boosted research drop odds.
    if input.focus_active {
        let bonus = ((crate::constants::FOCUS_TONIC_BONUS_MULT - 1.0) * 100.0).round() as i32;
        log.push(format!(
            "🧠 Focus active (+{}% research drop chance this battle).",
            bonus
        ));
    }
    if !dynamic_loot.is_empty() {
        let loot_str = dynamic_loot
            .iter()
            .map(|(i, q)| format!("`{}` {}", q, i.display_name()))
            .collect::<Vec<_>>()
            .join(", ");
        log.push(format!("🎁 You found: **{}**!", loot_str));
    }
    if input.vitality_mitigated > 0 {
        log.push(format!(
            "🛡️ Vitality prevented **{}** damage this battle.",
            input.vitality_mitigated
        ));
    }
    log.push("\n--- **Party Members Gained XP** ---".to_string());
    for (idx, res) in results.iter().enumerate() {
        if let Some(pu) = input.party_units.get(idx) {
            let name = pu.nickname.as_deref().unwrap_or(&pu.name);
            if res.did_level_up {
                log.push(format!(
                    "🌟 **{} leveled up to {}!** (+{} ATK, +{} DEF, +{} HP)",
                    name, res.new_level, res.stat_gains.0, res.stat_gains.1, res.stat_gains.2
                ));
            } else if (scaled_xp - node.reward_unit_xp).abs() > 0 {
                log.push(format!(
                    "- **{}** gained `{}` XP ({} base × {:.2} rarity).",
                    name, scaled_xp, node.reward_unit_xp, avg_multiplier
                ));
            } else {
                log.push(format!("- **{}** gained `{}` XP.", name, scaled_xp));
            }
        }
    }
    Ok(NodeVictoryResult { victory_log: log })
}
