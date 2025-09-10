//! Implements the `Game` trait for a battle session.

use crate::commands::games::{Game, GameUpdate};
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
        let mut session_clone = self.session.clone();
        if matches!(session_clone.phase, BattlePhase::Victory)
            && !session_clone
                .log
                .iter()
                .any(|l| l.contains("Vitality mitigated"))
            && session_clone.vitality_mitigated > 0
        {
            session_clone.log.push(format!(
                "✨ Vitality mitigated a total of {} damage this battle.",
                session_clone.vitality_mitigated
            ));
        }
        let (embed, mut components) = ui::render_battle(&session_clone, self.can_afford_recruit);
        // Append global nav row for cross-command navigation (Saga active).
        if matches!(
            self.session.phase,
            BattlePhase::Victory | BattlePhase::Defeat
        ) {
            crate::commands::saga::ui::add_nav(&mut components, "saga");
        }
        (content, embed, components)
    }

    async fn handle_interaction(
        &mut self,
        ctx: &Context,
        interaction: &mut ComponentInteraction,
        db: &PgPool,
    ) -> GameUpdate {
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
            "battle_item_cancel" => {
                self.session.phase = BattlePhase::PlayerTurn;
                self.session.log.push("You close your bag.".to_string());
                GameUpdate::ReRender
            }
            cid if cid.starts_with("battle_item_use_") => {
                // Parse item id suffix
                let item_id_str = cid.trim_start_matches("battle_item_use_");
                let item_id: i32 = match item_id_str.parse() {
                    Ok(v) => v,
                    Err(_) => {
                        self.session
                            .log
                            .push("⚠️ That item cannot be used.".to_string());
                        return GameUpdate::ReRender;
                    }
                };
                use crate::commands::economy::core::item::Item;
                let Some(item) = Item::from_i32(item_id) else {
                    self.session.log.push("⚠️ Unknown item.".to_string());
                    return GameUpdate::ReRender;
                };
                // Only allow health potions in-battle for now
                let heal_amount = match item {
                    Item::HealthPotion => 30,
                    Item::GreaterHealthPotion => 80,
                    _ => 0,
                };
                if heal_amount == 0 {
                    self.session
                        .log
                        .push("⚠️ You cannot use that here.".to_string());
                    return GameUpdate::ReRender;
                }
                // Determine if any ally can benefit before consuming inventory
                let target_exists = self
                    .session
                    .player_party
                    .iter()
                    .any(|u| u.current_hp > 0 && u.current_hp < u.max_hp);
                if !target_exists {
                    self.session
                        .log
                        .push("🧪 You are already at full health.".to_string());
                    return GameUpdate::ReRender;
                }
                // Consume from inventory and apply effect atomically
                let mut tx = match db.begin().await {
                    Ok(t) => t,
                    Err(_) => {
                        self.session
                            .log
                            .push("⚠️ Could not access your bag.".to_string());
                        return GameUpdate::ReRender;
                    }
                };
                match database::economy::get_inventory_item(&mut tx, interaction.user.id, item)
                    .await
                {
                    Ok(Some(it)) if it.quantity > 0 => {
                        if database::economy::add_to_inventory(
                            &mut tx,
                            interaction.user.id,
                            item,
                            -1,
                        )
                        .await
                        .is_err()
                        {
                            let _ = tx.rollback().await;
                            self.session
                                .log
                                .push("⚠️ Something went wrong using that item.".to_string());
                            return GameUpdate::ReRender;
                        }
                    }
                    _ => {
                        let _ = tx.rollback().await;
                        self.session
                            .log
                            .push("⚠️ You don't have that item.".to_string());
                        return GameUpdate::ReRender;
                    }
                }
                if tx.commit().await.is_err() {
                    self.session
                        .log
                        .push("⚠️ Failed to use the item.".to_string());
                    return GameUpdate::ReRender;
                }
                // Apply healing to the first living ally
                if let Some(unit) = self
                    .session
                    .player_party
                    .iter_mut()
                    .find(|u| u.current_hp > 0 && u.current_hp < u.max_hp)
                {
                    let before = unit.current_hp;
                    unit.current_hp = (unit.current_hp + heal_amount).min(unit.max_hp);
                    let healed = unit.current_hp - before;
                    self.session.log.push(format!(
                        "🧪 Used {} on {} (+{} HP).",
                        item.display_name(),
                        unit.name,
                        healed
                    ));
                } else {
                    self.session
                        .log
                        .push("🧪 You are already at full health.".to_string());
                }
                // Advance flow: end player phase and let enemy act
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
                    // Check Focus Tonic buff (TTL cache) for this user
                    let focus_active: bool = if let Some(state) =
                        crate::model::AppState::from_ctx(ctx).await
                    {
                        crate::services::cache::get_with_ttl(
                            &state.focus_buff_cache,
                            &interaction.user.id.get(),
                            std::time::Duration::from_secs(crate::constants::FOCUS_TONIC_TTL_SECS),
                        )
                        .await
                        .unwrap_or_default()
                    } else {
                        false
                    };
                    match battle::resolve_node_victory(
                        db,
                        crate::database::battle::ResolveVictoryInput {
                            user_id: interaction.user.id,
                            node_id: self.node_id,
                            node_name: self.node_name.clone(),
                            party_units: self.party_members.clone(),
                            vitality_mitigated: self.session.vitality_mitigated,
                            enemy_unit_ids: self
                                .session
                                .enemy_party
                                .iter()
                                .map(|e| e.unit_id)
                                .collect::<Vec<_>>(),
                            focus_active,
                        },
                    )
                    .await
                    {
                        Ok(r) => {
                            // Invalidate caches that may change due to human defeats or research drops
                            if let Some(state) = crate::AppState::from_ctx(ctx).await {
                                state.invalidate_user_caches(interaction.user.id).await;
                            }
                            {
                                let mut msg = r.victory_log.join("\n");
                                if self.session.vitality_mitigated > 0 {
                                    msg.push_str(&format!(
                                        "\n🔰 Vitality mitigated {} damage this battle.",
                                        self.session.vitality_mitigated
                                    ));
                                }
                                GameUpdate::GameOver {
                                    message: msg,
                                    payouts: vec![],
                                }
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
