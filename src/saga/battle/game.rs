//! Implements the `Game` trait for a battle session.

use crate::commands::economy::core::item::Item;
use crate::commands::games::engine::{Game, GameUpdate};
use crate::database;
use crate::saga::battle::{logic, state::*, ui};
use rand::Rng;
use serenity::async_trait;
use serenity::builder::{CreateActionRow, CreateEmbed};
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;
use sqlx::PgPool;
use std::any::Any;
use tokio::time::Duration;

pub struct BattleGame {
    pub session: BattleSession,
    pub party_members: Vec<database::models::PlayerPet>,
    pub node_id: i32,
    pub node_name: String,
    pub can_afford_tame: bool,
    // (✓) NEW: Add a field to track if this battle is for a quest.
    pub player_quest_id: Option<i32>,
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
        let (embed, components) = ui::render_battle(&self.session, self.can_afford_tame);
        (content, embed, components)
    }

    async fn handle_interaction(
        &mut self,
        _ctx: &Context,
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
            "battle_tame" => {
                // Taming is disabled for quest battles for simplicity.
                if self.player_quest_id.is_some() {
                    self.session
                        .log
                        .push("⚠️ You cannot tame quest enemies.".to_string());
                    return GameUpdate::ReRender;
                }

                if let Some(living_enemy) =
                    self.session.enemy_party.iter().find(|e| e.current_hp > 0)
                {
                    let pet_id_to_tame = living_enemy.pet_id;
                    match database::pets::attempt_tame_pet(db, interaction.user.id, pet_id_to_tame)
                        .await
                    {
                        Ok(pet_name) => GameUpdate::GameOver {
                            message: format!(
                                "🐾 **Success!** You spent your materials and successfully tamed the **{}**!",
                                pet_name
                            ),
                            payouts: vec![],
                        },
                        Err(e) => {
                            self.session.log.push(format!("⚠️ Tame failed: {}", e));
                            GameUpdate::ReRender
                        }
                    }
                } else {
                    self.session
                        .log
                        .push("⚠️ Tame failed: No target found.".to_string());
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
                    let node_data = match database::world::get_map_nodes_by_ids(db, &[self.node_id])
                        .await
                    {
                        Ok(mut nodes) if !nodes.is_empty() => nodes.remove(0),
                        _ => {
                            return GameUpdate::GameOver {
                                message: "Error: Could not retrieve node reward data.".to_string(),
                                payouts: vec![],
                            };
                        }
                    };

                    let potential_rewards =
                        match database::world::get_rewards_for_node(db, self.node_id).await {
                            Ok(rewards) => rewards,
                            Err(_) => {
                                return GameUpdate::GameOver {
                                    message: "Error: Could not retrieve loot data.".to_string(),
                                    payouts: vec![],
                                };
                            }
                        };

                    let actual_loot = {
                        let mut loot = Vec::new();
                        let mut rng = rand::rng();
                        for reward in potential_rewards {
                            if rng.random_bool(reward.drop_chance as f64)
                                && let Some(item_enum) = Item::from_i32(reward.item_id)
                            {
                                loot.push((item_enum, reward.quantity as i64));
                            }
                        }
                        loot
                    };

                    let results = match database::pets::apply_battle_rewards(
                        db,
                        interaction.user.id,
                        node_data.reward_coins,
                        &actual_loot,
                        &self.party_members,
                        node_data.reward_pet_xp,
                    )
                    .await
                    {
                        Ok(res) => res,
                        Err(_) => {
                            return GameUpdate::GameOver {
                                message: "Error: Failed to apply rewards to your profile."
                                    .to_string(),
                                payouts: vec![],
                            };
                        }
                    };

                    database::saga::advance_story_progress(db, interaction.user.id, self.node_id)
                        .await
                        .ok();

                    // Update battle-related tasks after a node victory.
                    database::tasks::update_task_progress(
                        db,
                        interaction.user.id,
                        &format!("WinBattle:{}", self.node_id),
                        1,
                    )
                    .await
                    .ok();
                    database::tasks::update_task_progress(db, interaction.user.id, "WinBattle", 1)
                        .await
                        .ok();

                    let mut victory_log = vec![
                        format!("🎉 **Victory at the {}!**", self.node_name),
                        format!("💰 You earned **{}** coins.", node_data.reward_coins),
                    ];
                    if !actual_loot.is_empty() {
                        let loot_str = actual_loot
                            .iter()
                            .map(|(item, qty)| format!("`{}` {}", qty, item.display_name()))
                            .collect::<Vec<_>>()
                            .join(", ");
                        victory_log.push(format!("🎁 You found: **{}**!", loot_str));
                    }
                    victory_log.push("\n--- **Party Members Gained XP** ---".to_string());
                    for (i, result) in results.iter().enumerate() {
                        let pet_name = self.party_members[i]
                            .nickname
                            .as_deref()
                            .unwrap_or(&self.party_members[i].name);
                        if result.did_level_up {
                            victory_log.push(format!(
                                "🌟 **{} leveled up to {}!** (+{} ATK, +{} DEF, +{} HP)",
                                pet_name,
                                result.new_level,
                                result.stat_gains.0,
                                result.stat_gains.1,
                                result.stat_gains.2
                            ));
                        } else {
                            victory_log.push(format!(
                                "- **{}** gained `{}` XP.",
                                pet_name, node_data.reward_pet_xp
                            ));
                        }
                    }
                    GameUpdate::GameOver {
                        message: victory_log.join("\n"),
                        payouts: vec![],
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
