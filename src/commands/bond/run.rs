//! Run logic for `/bond` command.

use crate::{AppState, database};
use crate::constants::rarity_icon;
use serenity::builder::{CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, EditInteractionResponse};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::*;

pub fn register() -> CreateCommand { 
    CreateCommand::new("bond")
        .description("Manage or view unit bonds")
        .add_option(
            serenity::builder::CreateCommandOption::new(serenity::model::application::CommandOptionType::SubCommand, "create", "Create a new bond (host + equipped)")
        )
        .add_option(
            serenity::builder::CreateCommandOption::new(serenity::model::application::CommandOptionType::SubCommand, "status", "View active bond contributions")
        )
}

pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    interaction.create_response(&ctx.http, CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new().ephemeral(true))).await.ok();
    let data = ctx.data.read().await;
    let Some(app_state) = data.get::<AppState>().cloned() else { return; };
    let pool = app_state.db.clone();
    let sub = interaction.data.options.first().map(|o| o.name.as_str()).unwrap_or("create");
    match sub {
        "status" => {
            match crate::database::units::list_bond_contributions(&pool, interaction.user.id).await {
                Ok(list) if !list.is_empty() => {
                    // Group by host
                    use std::collections::HashMap;
                    use crate::database::units::BondContribution;
                    let mut by_host: HashMap<i32, Vec<BondContribution>> = HashMap::new();
                    for b in list { by_host.entry(b.host_player_unit_id).or_default().push(b); }
                    // Rarity icon helper (duplicated small; could centralize later)
                    let mut desc = String::new();
                    let mut grand_totals = (0,0,0);
                    for (host, bonds) in by_host.iter() {
                        let mut host_totals = (0,0,0);
                        desc.push_str(&format!("**Host {}**\n", host));
                        for b in bonds { 
                            host_totals.0 += b.bonus_attack; host_totals.1 += b.bonus_defense; host_totals.2 += b.bonus_health;
                            grand_totals.0 += b.bonus_attack; grand_totals.1 += b.bonus_defense; grand_totals.2 += b.bonus_health;
                            desc.push_str(&format!(" • [#{}] {} {} (+{} Atk / +{} Def / +{} HP) [eq_id:{}]\n", b.bond_id, rarity_icon(b.rarity), b.equipped_name, b.bonus_attack, b.bonus_defense, b.bonus_health, b.equipped_player_unit_id));
                        }
                        desc.push_str(&format!("   └─ Host Total: +{} / +{} / +{}\n\n", host_totals.0, host_totals.1, host_totals.2));
                    }
                    desc.push_str(&format!("**Grand Total Bonuses:** +{} Atk / +{} Def / +{} HP", grand_totals.0, grand_totals.1, grand_totals.2));
                    let embed = serenity::builder::CreateEmbed::new()
                        .title("Active Bond Contributions")
                        .description(desc)
                        .color(0x9B59B6);
                    interaction.edit_response(&ctx.http, EditInteractionResponse::new().embed(embed).content(" ")).await.ok();
                }
                Ok(_) => { interaction.edit_response(&ctx.http, EditInteractionResponse::new().content("No active bonds." )).await.ok(); }
                Err(_) => { interaction.edit_response(&ctx.http, EditInteractionResponse::new().content("Failed to load bond contributions." )).await.ok(); }
            }
        }
        _ => {
            // default create flow
            let _ = database::saga::update_and_get_saga_profile(&pool, interaction.user.id).await;
            let units = match database::units::get_player_units(&pool, interaction.user.id).await { Ok(v) => v, Err(_) => { interaction.edit_response(&ctx.http, EditInteractionResponse::new().content("Could not load units.")).await.ok(); return; } };
            use crate::database::models::UnitRarity;
            let hosts: Vec<_> = units.iter().filter(|&u| matches!(u.rarity, UnitRarity::Rare | UnitRarity::Epic | UnitRarity::Legendary | UnitRarity::Unique | UnitRarity::Mythical | UnitRarity::Fabled)).cloned().collect();
            let candidates: Vec<_> = units.iter().filter(|&u| !u.is_in_party).cloned().collect();
            let (embed, components) = crate::commands::bond::ui::create_bond_select(&hosts, &candidates);
            interaction.edit_response(&ctx.http, EditInteractionResponse::new().embed(embed).components(components)).await.ok();
        }
    }
}

pub async fn run_prefix(ctx: &Context, msg: &Message, _args: Vec<&str>) {
    let Some(app_state) = AppState::from_ctx(ctx).await else { return; };
    let pool = app_state.db.clone();
    let _ = database::saga::update_and_get_saga_profile(&pool, msg.author.id).await;
    let units = match database::units::get_player_units(&pool, msg.author.id).await { Ok(v) => v, Err(_) => { msg.reply(&ctx.http, "Could not load units.").await.ok(); return; } };
    use crate::database::models::UnitRarity;
    let hosts: Vec<_> = units.iter().filter(|&u| matches!(u.rarity, UnitRarity::Rare | UnitRarity::Epic | UnitRarity::Legendary | UnitRarity::Unique | UnitRarity::Mythical | UnitRarity::Fabled)).cloned().collect();
    let candidates: Vec<_> = units.iter().filter(|&u| !u.is_in_party).cloned().collect();
    let (embed, components) = crate::commands::bond::ui::create_bond_select(&hosts, &candidates);
    msg.channel_id.send_message(&ctx.http, serenity::builder::CreateMessage::new().embed(embed).components(components)).await.ok();
}
