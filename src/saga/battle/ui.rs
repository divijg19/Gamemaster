//! Handles rendering the battle state into a Discord embed.

use super::state::{BattlePhase, BattleSession, BattleUnit};
use crate::commands::economy::core::item::Item;
use crate::ui::buttons::Btn;
use serenity::builder::{CreateActionRow, CreateEmbed};

pub fn render_battle(
    session: &BattleSession,
    can_afford_recruit: bool,
) -> (CreateEmbed, Vec<CreateActionRow>) {
    // (✓) MODIFIED: Title and color are now dynamic based on the battle's final outcome.
    let (title, color) = match session.phase {
        BattlePhase::PlayerTurn => ("Battle - Your Turn", 0xE74C3C), // Red
        BattlePhase::PlayerSelectingItem => ("Battle - Select an Item", 0x3498DB), // Blue
        BattlePhase::EnemyTurn => ("Battle - Enemy's Turn", 0xE74C3C), // Red
        BattlePhase::Victory => ("Victory!", 0x57F287),              // Green
        BattlePhase::Defeat => ("Defeat", 0x99AAB5),                 // Grey
    };

    // Build a concise header line (phase + living counts)
    let living_players = session
        .player_party
        .iter()
        .filter(|u| u.current_hp > 0)
        .count();
    let living_enemies = session
        .enemy_party
        .iter()
        .filter(|u| u.current_hp > 0)
        .count();
    let mut desc_lines = Vec::new();
    desc_lines.push(format!(
        "{} vs {} alive | Log:",
        living_players, living_enemies
    ));
    // Limit log to last 12 lines for readability
    let max_log = 12usize;
    let log_slice = if session.log.len() > max_log {
        &session.log[session.log.len() - max_log..]
    } else {
        &session.log[..]
    };
    desc_lines.extend(log_slice.iter().cloned());
    let embed = CreateEmbed::new()
        .title(title)
        .description(desc_lines.join("\n"))
        .field("Your Party", format_party_hp(&session.player_party), true)
        .field("Enemy Party", format_party_hp(&session.enemy_party), true)
        .footer(serenity::builder::CreateEmbedFooter::new(
            "Actions cost 1 turn • Tame only when one recruitable enemy remains",
        ))
        .color(color);

    // Component layout is determined by the battle phase.
    let components = match session.phase {
        BattlePhase::PlayerTurn => {
            let living_enemies: Vec<_> = session
                .enemy_party
                .iter()
                .filter(|e| e.current_hp > 0)
                .collect();
            let (show_tame, show_contract, tame_enabled, contract_enabled) =
                if living_enemies.len() == 1 {
                    let target = living_enemies[0];
                    let tame = target.is_recruitable && !target.is_human;
                    let contract = target.is_human; // contract drafting path
                    let tame_enabled = tame && can_afford_recruit;
                    let contract_enabled = contract; // contract doesn't require lure cost
                    (tame, contract, tame_enabled, contract_enabled)
                } else {
                    (false, false, false, false)
                };

            let mut buttons = vec![
                Btn::primary("battle_attack", "⚔️ Attack"),
                Btn::secondary("battle_item", "🎒 Item"),
            ];
            if show_tame {
                buttons.push(Btn::success("battle_recruit", "🪄 Tame").disabled(!tame_enabled));
            }
            if show_contract {
                buttons.push(
                    Btn::success("battle_contract", "📜 Contract").disabled(!contract_enabled),
                );
            }
            buttons.push(Btn::danger("battle_flee", "🏃 Flee"));
            vec![CreateActionRow::Buttons(buttons)]
        }
        // During enemy turn, show the buttons but disabled so the user knows what's available.
        BattlePhase::EnemyTurn => {
            vec![CreateActionRow::Buttons(vec![
                Btn::primary("disabled_attack", "⚔️ Attack").disabled(true),
                Btn::secondary("disabled_item", "🎒 Item").disabled(true),
                Btn::success("disabled_placeholder", "...").disabled(true),
                Btn::danger("disabled_flee", "🏃 Flee").disabled(true),
            ])]
        }
        // Item selection menu phase
        BattlePhase::PlayerSelectingItem => {
            let hp_id = Item::HealthPotion as i32;
            let ghp_id = Item::GreaterHealthPotion as i32;
            vec![
                CreateActionRow::Buttons(vec![
                    Btn::success(
                        &format!("battle_item_use_{}", hp_id),
                        "✨ Use Health Potion",
                    ),
                    Btn::success(
                        &format!("battle_item_use_{}", ghp_id),
                        "✨ Use Greater Health Potion",
                    ),
                ]),
                CreateActionRow::Buttons(vec![Btn::secondary("battle_item_cancel", "↩ Back")]),
            ]
        }
        // When the battle is won, show a "Claim Rewards" button.
        BattlePhase::Victory => {
            vec![
                CreateActionRow::Buttons(vec![Btn::success(
                    "battle_claim_rewards",
                    "🎁 Claim Rewards",
                )]),
                CreateActionRow::Buttons(vec![Btn::secondary("battle_close", "❌ Close")]),
                CreateActionRow::Buttons(vec![
                    Btn::secondary(crate::interactions::ids::SAGA_MAP, "↩ Map"),
                    Btn::secondary(crate::interactions::ids::SAGA_TAVERN, "🍺 Tavern"),
                ]),
            ]
        }
        BattlePhase::Defeat => {
            vec![CreateActionRow::Buttons(vec![Btn::secondary(
                "battle_close",
                "❌ Close",
            )])]
        }
    };

    (embed, components)
}

fn format_party_hp(party: &[BattleUnit]) -> String {
    party
        .iter()
        .map(|unit| {
            let (icon, hp_part) = if unit.current_hp <= 0 {
                ("💀", format!("0/{}", unit.max_hp))
            } else {
                ("❤️", format!("{}/{}", unit.current_hp, unit.max_hp))
            };
            format!("{} {} [{}]", icon, unit.name, hp_part)
        })
        .collect::<Vec<_>>()
        .join("\n")
}
