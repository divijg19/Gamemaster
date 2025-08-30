//! Implements the `Game` trait for a battle session.

use crate::commands::economy::core::item::Item;
use crate::commands::games::engine::{Game, GameUpdate};
use crate::database;
use crate::saga::battle::{logic, state::*, ui};
use serenity::async_trait;
use serenity::builder::{CreateActionRow, CreateEmbed};
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;
use sqlx::PgPool;
use std::any::Any;
use std::time::Duration;

#[allow(dead_code)]
pub struct BattleGame {
    pub session: BattleSession,
    pub party_members: Vec<database::profile::PlayerPet>,
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
                    // Define the rewards for winning this specific battle.
                    let coins = 50;
                    let xp_per_pet = 25;
                    let loot = vec![(Item::SlimeGel, 1), (Item::SlimeResearchData, 1)];

                    // Call the database function to apply all rewards atomically.
                    let results = database::profile::apply_battle_rewards(
                        db,
                        interaction.user.id,
                        coins,
                        &loot,
                        &self.party_members,
                        xp_per_pet,
                    )
                    .await
                    .unwrap_or_default();

                    // Format a detailed victory message.
                    let mut victory_log = vec![
                        format!("🎉 **Victory!**"),
                        format!("💰 You earned **{}** coins.", coins),
                        format!("🎁 You found **1 Slime Gel** and **1 Slime Research Data**!"),
                        "\n--- **Party Members Gained XP** ---".to_string(),
                    ];

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
