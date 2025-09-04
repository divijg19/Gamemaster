//! Handles rendering the battle state into a Discord embed.

use super::state::{BattlePhase, BattleSession, BattleUnit};
use serenity::builder::{CreateActionRow, CreateButton, CreateEmbed};
use serenity::model::application::ButtonStyle;

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
                CreateButton::new("battle_attack")
                    .label("Attack")
                    .style(ButtonStyle::Primary),
                CreateButton::new("battle_item")
                    .label("Item")
                    .style(ButtonStyle::Secondary),
            ];
            if show_tame {
                buttons.push(
                    CreateButton::new("battle_recruit")
                        .label("Tame")
                        .style(ButtonStyle::Success)
                        .disabled(!tame_enabled),
                );
            }
            if show_contract {
                buttons.push(
                    CreateButton::new("battle_contract")
                        .label("Contract")
                        .style(ButtonStyle::Success)
                        .disabled(!contract_enabled),
                );
            }
            buttons.push(
                CreateButton::new("battle_flee")
                    .label("Flee")
                    .style(ButtonStyle::Danger),
            );
            vec![CreateActionRow::Buttons(buttons)]
        }
        // (✓) MODIFIED: In these phases, show the buttons but disable them so the user knows what's available.
        BattlePhase::EnemyTurn | BattlePhase::PlayerSelectingItem => {
            vec![CreateActionRow::Buttons(vec![
                CreateButton::new("disabled_attack")
                    .label("Attack")
                    .style(ButtonStyle::Primary)
                    .disabled(true),
                CreateButton::new("disabled_item")
                    .label("Item")
                    .style(ButtonStyle::Secondary)
                    .disabled(true),
                CreateButton::new("disabled_placeholder")
                    .label("...")
                    .style(ButtonStyle::Success)
                    .disabled(true),
                CreateButton::new("disabled_flee")
                    .label("Flee")
                    .style(ButtonStyle::Danger)
                    .disabled(true),
            ])]
        }
        // (✓) MODIFIED: When the battle is won, show a "Claim Rewards" button.
        BattlePhase::Victory => {
            vec![CreateActionRow::Buttons(vec![
                CreateButton::new("battle_claim_rewards")
                    .label("Claim Rewards")
                    .style(ButtonStyle::Success),
            ])]
        }
        // (✓) MODIFIED: When the battle is lost, show a simple "Close" button.
        BattlePhase::Defeat => {
            vec![CreateActionRow::Buttons(vec![
                CreateButton::new("battle_close")
                    .label("Close")
                    .style(ButtonStyle::Secondary),
            ])]
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
