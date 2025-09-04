//! Implements the `Game` trait for a battle session.

use crate::commands::games::engine::{Game, GameUpdate};
use crate::database;
use crate::database::battle;
use crate::database::models::UnitKind;
use crate::saga::battle::{logic, state::*, ui};
use serenity::async_trait;
use serenity::builder::{CreateActionRow, CreateEmbed};
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;
use sqlx::PgPool;
use std::any::Any;
use tokio::time::Duration;

pub struct BattleGame {
    pub session: BattleSession,
    pub party_members: Vec<database::models::PlayerUnit>,
    pub node_id: i32,
    pub node_name: String,
    pub can_afford_recruit: bool,
    // (✓) NEW: Add a field to track if this battle is for a quest.
    pub player_quest_id: Option<i32>,
    // Cached equipment bonuses applied (host_player_unit_id -> (atk,def,hp)) to prevent recompute each interaction
    pub applied_equipment: bool,
    pub claimed: bool,
}

#[async_trait]
impl Game for BattleGame {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn render(&self) -> (String, CreateEmbed, Vec<CreateActionRow>) {
        let content = match self.session.phase {
            BattlePhase::Victory => "🎉 **VICTORY** 🎉".to_string(),
            BattlePhase::Defeat => "☠️ **DEFEAT** ☠️".to_string(),
            _ => "".to_string(),
        };
        let (embed, components) = ui::render_battle(&self.session, self.can_afford_recruit);
        (content, embed, components)
    }

    async fn handle_interaction(
        &mut self,
        ctx: &Context,
        interaction: &mut ComponentInteraction,
        db: &PgPool,
    ) -> GameUpdate {
        // One-time application of equipment bonuses (snapshot at first interaction) if not already applied.
        if !self.applied_equipment {
            if let Some(_app_state) = crate::AppState::from_ctx(ctx).await {
                // context fetched (reserved for future use)
                let bonuses = database::units::get_equipment_bonuses(db, interaction.user.id)
                    .await
                    .unwrap_or_default();
                for unit in &mut self.session.player_party {
                    if let Some(b) = bonuses.get(&unit.unit_id) {
                        unit.attack += b.0;
                        unit.defense += b.1;
                        unit.max_hp += b.2;
                        unit.current_hp += b.2; // heal with bonus
                        unit.bonus_attack = b.0;
                        unit.bonus_defense = b.1;
                        unit.bonus_health = b.2;
                        self.session.log.push(format!(
                            "Equipment power surges around {} (+{} Atk / +{} Def / +{} HP).",
                            unit.name, b.0, b.1, b.2
                        ));
                    }
                }
            }
            self.applied_equipment = true;
        }
        match interaction.data.custom_id.as_str() {
            "battle_attack" => {
                if logic::process_player_turn(&mut self.session) == BattleOutcome::PlayerVictory {
                    self.session.phase = BattlePhase::Victory;
                    self.session.log.push("---".to_string());
                    self.session
                        .log
                        .push("You have defeated all enemies!".to_string());
                    return GameUpdate::ReRender;
                }

                tokio::time::sleep(Duration::from_millis(750)).await;

                if logic::process_enemy_turn(&mut self.session) == BattleOutcome::PlayerDefeat {
                    self.session.phase = BattlePhase::Defeat;
                    self.session.log.push("---".to_string());
                    self.session
                        .log
                        .push("Your party has been defeated.".to_string());
                    return GameUpdate::ReRender;
                }

                GameUpdate::ReRender
            }
            "battle_contract" => {
                if self.player_quest_id.is_some() {
                    self.session
                        .log
                        .push("⚠️ Contracts disabled in quest battles.".to_string());
                    return GameUpdate::ReRender;
                }
                if let Some(target) = self.session.enemy_party.iter().find(|e| e.current_hp > 0) {
                    match database::units::get_units_by_ids(db, &[target.unit_id]).await {
                        Ok(units)
                            if !units.is_empty() && matches!(units[0].kind, UnitKind::Human) =>
                        {
                            match database::human::draft_contract(
                                db,
                                interaction.user.id,
                                units[0].unit_id,
                            )
                            .await
                            {
                                Ok(_) => {
                                    self.session.log.push(format!(
                                        "📝 Drafted a contract for {}! Use /contracts to review.",
                                        units[0].name
                                    ));
                                }
                                Err(e) => {
                                    self.session
                                        .log
                                        .push(format!("⚠️ Could not draft contract: {}", e));
                                }
                            }
                        }
                        _ => {
                            self.session
                                .log
                                .push("⚠️ No human target available.".to_string());
                        }
                    }
                } else {
                    self.session.log.push("⚠️ No target.".to_string());
                }
                GameUpdate::ReRender
            }
            "battle_recruit" => {
                if self.player_quest_id.is_some() {
                    self.session
                        .log
                        .push("⚠️ You cannot recruit or tame during a quest battle.".to_string());
                    return GameUpdate::ReRender;
                }
                if let Some(target) = self.session.enemy_party.iter().find(|e| e.current_hp > 0) {
                    // Distinguish human vs pet by master data lookup (kind)
                    match database::units::get_units_by_ids(db, &[target.unit_id]).await {
                        Ok(units) if !units.is_empty() => {
                            let meta = &units[0];
                            if matches!(meta.kind, UnitKind::Human) {
                                // For humans: attempt contract draft if ready (auto path) else inform progress
                                match database::human::draft_contract(
                                    db,
                                    interaction.user.id,
                                    meta.unit_id,
                                )
                                .await
                                {
                                    Ok(_) => {
                                        self.session.log.push(format!("📝 Drafted a contract for {} (use /contracts accept {}).", meta.name, meta.unit_id));
                                        GameUpdate::ReRender
                                    }
                                    Err(e) => {
                                        self.session
                                            .log
                                            .push(format!("⚠️ Contract draft failed: {}", e));
                                        GameUpdate::ReRender
                                    }
                                }
                            } else {
                                // Pet / creature taming path
                                match database::units::attempt_recruit_unit(
                                    db,
                                    interaction.user.id,
                                    meta.unit_id,
                                )
                                .await
                                {
                                    Ok(name) => GameUpdate::GameOver {
                                        message: format!("🟢 **Tamed {}!**", name),
                                        payouts: vec![],
                                    },
                                    Err(e) => {
                                        self.session.log.push(format!("⚠️ Tame failed: {}", e));
                                        GameUpdate::ReRender
                                    }
                                }
                            }
                        }
                        _ => {
                            self.session
                                .log
                                .push("⚠️ Could not resolve target metadata.".to_string());
                            GameUpdate::ReRender
                        }
                    }
                } else {
                    self.session.log.push("⚠️ No valid target.".to_string());
                    GameUpdate::ReRender
                }
            }
            "battle_item" => {
                self.session.phase = BattlePhase::PlayerSelectingItem;
                self.session.log.push("You open your bag...".to_string());
                GameUpdate::ReRender
            }
            "battle_flee" => GameUpdate::GameOver {
                message: "You fled from the battle.".to_string(),
                payouts: vec![],
            },
            "battle_claim_rewards" => {
                if self.claimed {
                    return GameUpdate::ReRender;
                }
                self.claimed = true;
                // (✓) MODIFIED: Branch logic for Quest vs. Node battles.
                if let Some(player_quest_id) = self.player_quest_id {
                    // --- This is a QUEST BATTLE victory ---
                    match database::quests::complete_quest(db, interaction.user.id, player_quest_id)
                        .await
                    {
                        Ok(_) => {
                            let quest_title =
                                database::quests::get_quest_title(db, player_quest_id)
                                    .await
                                    .unwrap_or_else(|_| "a quest".to_string());
                            GameUpdate::GameOver {
                                message: format!(
                                    "🎉 **Quest Complete!** 🎉\n\nYou have successfully completed: **{}**.\nYour rewards have been added to your balance and inventory!",
                                    quest_title
                                ),
                                payouts: vec![],
                            }
                        }
                        Err(e) => GameUpdate::GameOver {
                            message: format!("There was an error completing your quest: {}", e),
                            payouts: vec![],
                        },
                    }
                } else {
                    // --- This is a NORMAL NODE BATTLE victory ---
                    match battle::resolve_node_victory(
                        db,
                        interaction.user.id,
                        self.node_id,
                        &self.node_name,
                        &self.party_members,
                        self.session.vitality_mitigated,
                        &self
                            .session
                            .enemy_party
                            .iter()
                            .map(|e| e.unit_id)
                            .collect::<Vec<_>>(),
                    )
                    .await
                    {
                        Ok(r) => {
                            // Invalidate caches that may change due to human defeats or research drops
                            if let Some(state) = crate::AppState::from_ctx(ctx).await {
                                state.invalidate_user_caches(interaction.user.id).await;
                            }
                            GameUpdate::GameOver {
                                message: r.victory_log.join("\n"),
                                payouts: vec![],
                            }
                        }
                        Err(e) => GameUpdate::GameOver {
                            message: format!("Error: {}", e),
                            payouts: vec![],
                        },
                    }
                }
            }
            "battle_close" => GameUpdate::GameOver {
                message: "Battle ended.".to_string(),
                payouts: vec![],
            },
            _ => GameUpdate::NoOp,
        }
    }
}
