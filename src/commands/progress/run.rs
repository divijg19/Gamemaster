use crate::database::models::UnitRarity;
use crate::ui::buttons::Btn;
use crate::{AppState, database};
use serenity::builder::{CreateActionRow, CreateCommand, CreateEmbed};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::Context;

pub fn register() -> CreateCommand {
    CreateCommand::new("progress").description("Unified snapshot: contracts & research progress")
}

pub async fn run_slash(ctx: &Context, interaction: &mut CommandInteraction) {
    interaction.defer_ephemeral(&ctx.http).await.ok();
    let Some(state) = AppState::from_ctx(ctx).await else {
        return;
    };
    let db = &state.db;
    let mut embed = CreateEmbed::new()
        .title("Progress Overview")
        .description("Summary of human recruitment and pet research.");
    if let Ok(contract_rows) =
        database::human::list_contract_status_cached(&state, interaction.user.id).await
    {
        if contract_rows.is_empty() {
            embed = embed.field("Contracts", "No human encounters yet.", false);
        } else {
            // show top 5 nearest to ready (sort by remaining defeats)
            let mut rows = contract_rows.clone();
            rows.sort_by_key(|(_u, d, req, _dr, _rec, _last)| req - d);
            let sample = rows
                .into_iter()
                .take(5)
                .map(|(u, d, req, dr, rec, _last)| {
                    let status = if rec {
                        "Recruited"
                    } else if dr {
                        "Drafted"
                    } else if d >= req {
                        "Ready"
                    } else {
                        "Progress"
                    };
                    format!("{}: {}/{} {}", u.name, d, req, status)
                })
                .collect::<Vec<_>>()
                .join("\n");
            embed = embed.field("Contracts", sample, false);
        }
    }
    // Optional verbose (uses legacy list_human_progress) if a config flag is set
    if let Ok(Some(flag)) =
        database::settings::get_config_value(db, "progress_verbose_contracts").await
        && flag == "1"
        && let Ok(detail_rows) = database::human::list_human_progress(db, interaction.user.id).await
    {
        let snippet = detail_rows
            .into_iter()
            .take(3)
            .map(|(u, d, req)| format!("{} {} / {}", u.name, d, req))
            .collect::<Vec<_>>()
            .join(" | ");
        if !snippet.is_empty() {
            embed = embed.field("Contracts (Verbose)", snippet, false);
        }
    }
    if let Ok(research_rows) =
        database::units::list_research_progress_cached(&state, interaction.user.id).await
    {
        use std::collections::HashMap;
        let map: HashMap<i32, i32> = research_rows.into_iter().collect();
        if let Ok(units) = database::units::get_all_units(db).await {
            let pet_units: Vec<_> = units
                .into_iter()
                .filter(|u| matches!(u.kind, database::models::UnitKind::Pet))
                .collect();
            // Pre-fetch rarity targets once (tiny enum so just call function per variant)
            let common_t =
                database::units::research_target_for_rarity(db, UnitRarity::Common).await;
            let rare_t = database::units::research_target_for_rarity(db, UnitRarity::Rare).await;
            let epic_t = database::units::research_target_for_rarity(db, UnitRarity::Epic).await;
            let high_t =
                database::units::research_target_for_rarity(db, UnitRarity::Legendary).await; // high group same func path
            let mut tuples: Vec<(database::models::Unit, i32, i32)> = Vec::new();
            for u in pet_units.into_iter() {
                let count = map.get(&u.unit_id).cloned().unwrap_or(0);
                let target = match u.rarity {
                    UnitRarity::Common => common_t,
                    UnitRarity::Rare => rare_t,
                    UnitRarity::Epic => epic_t,
                    UnitRarity::Legendary
                    | UnitRarity::Unique
                    | UnitRarity::Mythical
                    | UnitRarity::Fabled => high_t,
                };
                tuples.push((u, count, target));
            }
            tuples.sort_by_key(|(_u, c, t)| if *t == 0 { 0 } else { *t - *c });
            let sample = tuples
                .into_iter()
                .take(5)
                .map(|(u, count, target)| {
                    if target == 0 {
                        format!("{}: Party Eligible", u.name)
                    } else {
                        format!("{}: {}/{}", u.name, count, target)
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            embed = embed.field(
                "Research",
                if sample.is_empty() {
                    "No pets".to_string()
                } else {
                    sample
                },
                false,
            );
        }
    }
    let row = CreateActionRow::Buttons(vec![
        Btn::secondary("contracts_refresh", "Contracts"),
        Btn::secondary("research_refresh", "Research"),
        Btn::secondary("bestiary_refresh", "Bestiary"),
    ]);
    let resp = serenity::builder::CreateInteractionResponseMessage::new()
        .embed(embed)
        .components(vec![row]);
    let _ = interaction
        .create_response(
            &ctx.http,
            serenity::builder::CreateInteractionResponse::Message(resp),
        )
        .await;
}

pub async fn run_prefix(ctx: &Context, msg: &Message, _args: Vec<&str>) {
    let Some(state) = AppState::from_ctx(ctx).await else {
        return;
    };
    let db = &state.db;
    let mut embed = CreateEmbed::new()
        .title("Progress Overview")
        .description("Summary of human recruitment and pet research.");
    if let Ok(contract_rows) =
        database::human::list_contract_status_cached(&state, msg.author.id).await
    {
        if contract_rows.is_empty() {
            embed = embed.field("Contracts", "No human encounters yet.", false);
        } else {
            let mut rows = contract_rows.clone();
            rows.sort_by_key(|(_u, d, req, _dr, _rec, _last)| req - d);
            let sample = rows
                .into_iter()
                .take(5)
                .map(|(u, d, req, dr, rec, _last)| {
                    let status = if rec {
                        "Recruited"
                    } else if dr {
                        "Drafted"
                    } else if d >= req {
                        "Ready"
                    } else {
                        "Progress"
                    };
                    format!("{}: {}/{} {}", u.name, d, req, status)
                })
                .collect::<Vec<_>>()
                .join("\n");
            embed = embed.field("Contracts", sample, false);
        }
    }
    if let Ok(Some(flag)) =
        database::settings::get_config_value(db, "progress_verbose_contracts").await
        && flag == "1"
        && let Ok(detail_rows) = database::human::list_human_progress(db, msg.author.id).await
    {
        let snippet = detail_rows
            .into_iter()
            .take(3)
            .map(|(u, d, req)| format!("{} {} / {}", u.name, d, req))
            .collect::<Vec<_>>()
            .join(" | ");
        if !snippet.is_empty() {
            embed = embed.field("Contracts (Verbose)", snippet, false);
        }
    }
    if let Ok(research_rows) =
        database::units::list_research_progress_cached(&state, msg.author.id).await
    {
        use std::collections::HashMap;
        let map: HashMap<i32, i32> = research_rows.into_iter().collect();
        if let Ok(units) = database::units::get_all_units(db).await {
            let pet_units: Vec<_> = units
                .into_iter()
                .filter(|u| matches!(u.kind, database::models::UnitKind::Pet))
                .collect();
            let common_t =
                database::units::research_target_for_rarity(db, UnitRarity::Common).await;
            let rare_t = database::units::research_target_for_rarity(db, UnitRarity::Rare).await;
            let epic_t = database::units::research_target_for_rarity(db, UnitRarity::Epic).await;
            let high_t =
                database::units::research_target_for_rarity(db, UnitRarity::Legendary).await;
            let mut tuples: Vec<(database::models::Unit, i32, i32)> = Vec::new();
            for u in pet_units.into_iter() {
                let count = map.get(&u.unit_id).cloned().unwrap_or(0);
                let target = match u.rarity {
                    UnitRarity::Common => common_t,
                    UnitRarity::Rare => rare_t,
                    UnitRarity::Epic => epic_t,
                    UnitRarity::Legendary
                    | UnitRarity::Unique
                    | UnitRarity::Mythical
                    | UnitRarity::Fabled => high_t,
                };
                tuples.push((u, count, target));
            }
            tuples.sort_by_key(|(_u, c, t)| if *t == 0 { 0 } else { *t - *c });
            let sample = tuples
                .into_iter()
                .take(5)
                .map(|(u, count, target)| {
                    if target == 0 {
                        format!("{}: Party Eligible", u.name)
                    } else {
                        format!("{}: {}/{}", u.name, count, target)
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            embed = embed.field(
                "Research",
                if sample.is_empty() {
                    "No pets".to_string()
                } else {
                    sample
                },
                false,
            );
        }
    }
    let row = CreateActionRow::Buttons(vec![
        Btn::secondary("contracts_refresh", "Contracts"),
        Btn::secondary("research_refresh", "Research"),
        Btn::secondary("bestiary_refresh", "Bestiary"),
    ]);
    let _ = msg
        .channel_id
        .send_message(
            &ctx.http,
            serenity::builder::CreateMessage::new()
                .embed(embed)
                .components(vec![row]),
        )
        .await;
}
