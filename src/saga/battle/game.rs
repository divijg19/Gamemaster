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
use std::time::Duration;

pub struct BattleGame {
    pub session: BattleSession,
    pub party_members: Vec<database::models::PlayerPet>,
    pub node_id: i32,
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
        let (embed, components) = ui::render_battle(&self.session);
        ("".to_string(), embed, components)
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
                    let coins = 50;
                    let xp_per_pet = 25;
                    let actual_loot = {
                        let potential_rewards = database::get_rewards_for_node(db, self.node_id)
                            .await
                            .unwrap_or_default();
                        let mut loot = Vec::new();
                        let mut rng = rand::rng();
                        for reward in potential_rewards {
                            // (✓) FIXED: The `if` statement has been collapsed for better readability.
                            if rng.random_bool(reward.drop_chance as f64)
                                && let Some(item_enum) = Item::from_i32(reward.item_id)
                            {
                                loot.push((item_enum, reward.quantity as i64));
                            }
                        }
                        loot
                    };
                    let results = database::apply_battle_rewards(
                        db,
                        interaction.user.id,
                        coins,
                        &actual_loot,
                        &self.party_members,
                        xp_per_pet,
                    )
                    .await
                    .unwrap_or_default();
                    database::advance_story_progress(db, interaction.user.id, self.node_id)
                        .await
                        .ok();
                    let mut victory_log = vec![
                        format!("🎉 **Victory!**"),
                        format!("💰 You earned **{}** coins.", coins),
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
                            victory_log
                                .push(format!("- **{}** gained `{}` XP.", pet_name, xp_per_pet));
                        }
                    }
                    return GameUpdate::GameOver {
                        message: victory_log.join("\n"),
                        payouts: vec![],
                    };
                }

                tokio::time::sleep(Duration::from_secs(1)).await;
                if logic::process_enemy_turn(&mut self.session) == BattleOutcome::PlayerDefeat {
                    return GameUpdate::GameOver {
                        message: "Your party was defeated.".to_string(),
                        payouts: vec![],
                    };
                }

                GameUpdate::ReRender
            }
            // (✓) ADDED: The logic for the "Tame" button is now live.
            "battle_tame" => {
                // Find the single living enemy. This is safe because the button is only enabled when there is one.
                let living_enemy = self
                    .session
                    .enemy_party
                    .iter()
                    .find(|e| e.current_hp > 0)
                    .unwrap();
                let pet_id_to_tame = living_enemy.pet_id; // This makes the `pet_id` field "live".

                let result =
                    database::attempt_tame_pet(db, interaction.user.id, pet_id_to_tame).await;

                match result {
                    Ok(pet_name) => {
                        // If taming is successful, the battle ends immediately.
                        return GameUpdate::GameOver {
                            message: format!(
                                "🐾 **Success!** You spent your materials and successfully tamed the **{}**!",
                                pet_name
                            ),
                            payouts: vec![],
                        };
                    }
                    Err(e) => {
                        // If taming fails, add a message to the log and continue the battle.
                        self.session.log.push(format!("⚠️ Tame failed: {}", e));
                        GameUpdate::ReRender
                    }
                }
            }
            "battle_flee" => GameUpdate::GameOver {
                message: "You fled from the battle.".to_string(),
                payouts: vec![],
            },
            _ => GameUpdate::NoOp,
        }
    }
}
