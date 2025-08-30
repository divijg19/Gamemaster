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
    pub party_members: Vec<database::profile::PlayerPet>,
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

                    // (✓) FIXED: Create a new scope for loot calculation.
                    // This ensures the non-thread-safe `rng` variable is dropped before any `.await` calls.
                    let actual_loot = {
                        let potential_rewards =
                            database::profile::get_rewards_for_node(db, self.node_id)
                                .await
                                .unwrap_or_default();
                        let mut loot = Vec::new();
                        // `rng` is created and used only within this block.
                        let mut rng = rand::rng();
                        for reward in potential_rewards {
                            if rng.random_bool(reward.drop_chance as f64) {
                                if let Some(item_enum) = Item::from_i32(reward.item_id) {
                                    loot.push((item_enum, reward.quantity as i64));
                                }
                            }
                        }
                        loot // The calculated loot is returned from the block.
                    }; // `rng` goes out of scope here, BEFORE the next .await.

                    // Now it is safe to await the database call.
                    let results = database::profile::apply_battle_rewards(
                        db,
                        interaction.user.id,
                        coins,
                        &actual_loot,
                        &self.party_members,
                        xp_per_pet,
                    )
                    .await
                    .unwrap_or_default();

                    database::profile::advance_story_progress(
                        db,
                        interaction.user.id,
                        self.node_id,
                    )
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
            "battle_flee" => GameUpdate::GameOver {
                message: "You fled from the battle.".to_string(),
                payouts: vec![],
            },
            _ => GameUpdate::NoOp,
        }
    }
}
