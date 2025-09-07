//! Handles rendering the battle state into a Discord embed.

use super::state::{BattlePhase, BattleSession, BattleUnit};
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

    let embed = CreateEmbed::new()
        .title(title)
        .description(session.log.join("\n"))
        .field("Your Party", format_party_hp(&session.player_party), true)
        .field("Enemy Party", format_party_hp(&session.enemy_party), true)
        .color(color);

    // (✓) MODIFIED: The entire component layout is now determined by the battle phase.
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
        // (✓) MODIFIED: In these phases, show the buttons but disable them so the user knows what's available.
        BattlePhase::EnemyTurn | BattlePhase::PlayerSelectingItem => {
            vec![CreateActionRow::Buttons(vec![
                Btn::primary("disabled_attack", "⚔️ Attack").disabled(true),
                Btn::secondary("disabled_item", "🎒 Item").disabled(true),
                Btn::success("disabled_placeholder", "...").disabled(true),
                Btn::danger("disabled_flee", "🏃 Flee").disabled(true),
            ])]
        }
        // (✓) MODIFIED: When the battle is won, show a "Claim Rewards" button.
        BattlePhase::Victory => {
            vec![CreateActionRow::Buttons(vec![Btn::success(
                "battle_claim_rewards",
                "🎁 Claim Rewards",
            )])]
        }
        // (✓) MODIFIED: When the battle is lost, show a simple "Close" button.
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
            let status = if unit.current_hp <= 0 {
                "💀"
            } else {
                "❤️"
            };
            format!(
                "{} **{}** ({}/{})",
                status, unit.name, unit.current_hp, unit.max_hp
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
