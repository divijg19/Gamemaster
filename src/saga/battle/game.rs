//! Implements the `Game` trait for a battle session.

use crate::commands::economy::core::item::Item;
use crate::commands::games::engine::{Game, GameUpdate};
use crate::database;
use crate::saga::battle::{logic, state::*, ui};
// (✓) FIXED: Corrected the deprecated `rand` import. `thread_rng` is now brought into scope directly.
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
}

#[async_trait]
impl Game for BattleGame {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    // (✓) FIXED: The render function signature now correctly matches the Game trait and takes no arguments.
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
                if let Some(living_enemy) =
                    self.session.enemy_party.iter().find(|e| e.current_hp > 0)
                {
                    let pet_id_to_tame = living_enemy.pet_id;
                    match database::attempt_tame_pet(db, interaction.user.id, pet_id_to_tame).await
                    {
                        // (✓) FIXED: Added the required `payouts` field to `GameOver`.
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
                // (✓) FIXED: Added the required `payouts` field to `GameOver`.
                payouts: vec![],
            },
            "battle_claim_rewards" => {
                let node_data = match database::get_map_nodes_by_ids(db, &[self.node_id]).await {
                    Ok(mut nodes) if !nodes.is_empty() => nodes.remove(0),
                    // (✓) FIXED: Replaced `GameUpdate::Message` with the correct `GameOver` variant.
                    _ => {
                        return GameUpdate::GameOver {
                            message: "Error: Could not retrieve node reward data.".to_string(),
                            payouts: vec![],
                        };
                    }
                };

                let potential_rewards = match database::get_rewards_for_node(db, self.node_id).await
                {
                    Ok(rewards) => rewards,
                    // (✓) FIXED: Replaced `GameUpdate::Message` with the correct `GameOver` variant.
                    Err(_) => {
                        return GameUpdate::GameOver {
                            message: "Error: Could not retrieve loot data.".to_string(),
                            payouts: vec![],
                        };
                    }
                };

                let actual_loot = {
                    let mut loot = Vec::new();
                    // (✓) FIXED: Corrected call to `thread_rng`.
                    let mut rng = rand::rng();
                    for reward in potential_rewards {
                        // (✓) FIXED: Cast `f32` drop_chance to `f64` to match the function's requirement.
                        if rng.random_bool(reward.drop_chance as f64)
                            && let Some(item_enum) = Item::from_i32(reward.item_id)
                        {
                            loot.push((item_enum, reward.quantity as i64));
                        }
                    }
                    loot
                };

                let results = match database::apply_battle_rewards(
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
                    // (✓) FIXED: Replaced `GameUpdate::Message` with the correct `GameOver` variant.
                    Err(_) => {
                        return GameUpdate::GameOver {
                            message: "Error: Failed to apply rewards to your profile.".to_string(),
                            payouts: vec![],
                        };
                    }
                };

                database::advance_story_progress(db, interaction.user.id, self.node_id)
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
                // (✓) FIXED: Added the required `payouts` field to `GameOver`.
                GameUpdate::GameOver {
                    message: victory_log.join("\n"),
                    payouts: vec![],
                }
            }
            "battle_close" => GameUpdate::GameOver {
                message: "Battle ended.".to_string(),
                // (✓) FIXED: Added the required `payouts` field to `GameOver`.
                payouts: vec![],
            },
            _ => GameUpdate::NoOp,
        }
    }
}
