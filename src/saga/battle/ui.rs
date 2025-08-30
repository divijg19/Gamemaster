//! Handles rendering the battle state into a Discord embed.

use super::state::{BattleParty, BattleSession, BattleUnit};
use serenity::builder::{CreateActionRow, CreateButton, CreateEmbed};
use serenity::model::application::ButtonStyle;

pub fn render_battle(session: &BattleSession) -> (CreateEmbed, Vec<CreateActionRow>) {
    let turn_text = match session.current_turn {
        BattleParty::Player => "Your Turn",
        BattleParty::Enemy => "Enemy's Turn",
    };

    let embed = CreateEmbed::new()
        .title(format!("Battle in Progress - {}", turn_text))
        .description(session.log.join("\n"))
        .field("Your Party", format_party_hp(&session.player_party), true)
        .field("Enemy Party", format_party_hp(&session.enemy_party), true)
        .color(0xE74C3C); // Red

    let components = vec![CreateActionRow::Buttons(vec![
        CreateButton::new("battle_attack")
            .label("Attack")
            .style(ButtonStyle::Primary)
            .disabled(session.current_turn != BattleParty::Player),
        CreateButton::new("battle_flee")
            .label("Flee")
            .style(ButtonStyle::Secondary),
    ])];

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
