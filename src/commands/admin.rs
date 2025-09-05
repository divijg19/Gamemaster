use crate::{AppState, database};
use serenity::builder::{CreateCommand, CreateCommandOption, CreateEmbed};
use serenity::model::application::{CommandInteraction, CommandOptionType};
use serenity::prelude::Context;

// Lightweight admin utility to exercise maintenance helpers so they stay active.
pub fn register() -> CreateCommand {
    CreateCommand::new("adminutil")
        .description("Maintenance utilities (owner-only)")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "markhuman",
                "Mark a unit id as Human",
            )
            .required(false),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "diaguser",
                "Diagnose saga state for a user id (default: invoking user)",
            )
            .required(false),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "bondhost",
                "Host player_unit_id for bonding test",
            )
            .required(false),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "bondequip",
                "Equipped player_unit_id for bonding test",
            )
            .required(false),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "researchunit",
                "Unit id to fetch raw research progress",
            )
            .required(false),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "cachestats",
            "Show global in-memory cache hit/miss counters",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "sagainit",
            "Bootstrap base + saga profile and starter unit if absent",
        ))
}

pub async fn run_slash(ctx: &Context, interaction: &mut CommandInteraction) {
    interaction.defer_ephemeral(&ctx.http).await.ok();
    let Some(state) = AppState::from_ctx(ctx).await else {
        return;
    };
    let db = &state.db;
    let mut embed = CreateEmbed::new().title("Admin Util");
    let mut notes = Vec::new();
    let mut mark_ids: Vec<i32> = Vec::new();
    let mut bond_pair: Option<(i32, i32)> = None;
    let mut research_unit: Option<i32> = None;
    let mut show_cache = false;
    let mut diag_user: Option<i64> = None;
    let mut saga_init = false;
    for opt in &interaction.data.options {
        match opt.name.as_str() {
            "cachestats" => show_cache = true,
            "sagainit" => saga_init = true,
            "markhuman" => {
                if let Some(v) = opt.value.as_i64() {
                    mark_ids.push(v as i32);
                }
            }
            "diaguser" => {
                if let Some(v) = opt.value.as_i64() {
                    diag_user = Some(v);
                }
            }
            "bondhost" => {
                let host = opt.value.as_i64().unwrap_or(0) as i32;
                if let Some(existing) = bond_pair {
                    bond_pair = Some((host, existing.1));
                } else {
                    bond_pair = Some((host, 0));
                }
            }
            "bondequip" => {
                let eq = opt.value.as_i64().unwrap_or(0) as i32;
                if let Some(existing) = bond_pair {
                    bond_pair = Some((existing.0, eq));
                } else {
                    bond_pair = Some((0, eq));
                }
            }
            "researchunit" => research_unit = opt.value.as_i64().map(|v| v as i32),
            _ => {}
        }
    }
    if !mark_ids.is_empty() {
        match database::units::mark_units_as_human(db, &mark_ids).await {
            Ok(n) => notes.push(format!("Marked {} units as Human", n)),
            Err(e) => notes.push(format!("Mark error: {}", e)),
        }
    }
    if let Some((host, equip)) = bond_pair
        && host > 0
        && equip > 0
    {
        if let Err(e) = database::units::bond_units(db, interaction.user.id, host, equip).await {
            notes.push(format!("Bond error: {}", e));
        } else {
            notes.push("Bond attempted (see logs if constraints)".into());
        }
    }
    if let Some(uid) = research_unit {
        // Also touch list_research_progress to keep bulk path active
        let _ = database::units::list_research_progress(db, interaction.user.id)
            .await
            .ok();
        if let Ok(count) =
            database::units::get_research_progress(db, interaction.user.id, uid).await
        {
            notes.push(format!("Research progress unit {} = {}", uid, count));
        }
    }
    // Saga bootstrap before diagnostics so results show updated state
    if saga_init {
        let user_id = interaction.user.id;
        match database::economy::get_or_create_profile(db, user_id).await {
            Ok(_) => notes.push("Base profile ensured.".into()),
            Err(e) => notes.push(format!("Base profile error: {e}")),
        }
        match crate::database::saga::update_and_get_saga_profile(db, user_id).await {
            Ok(_) => notes.push("Saga profile ensured.".into()),
            Err(e) => notes.push(format!("Saga profile error: {e}")),
        }
        let starter_id = *state.starter_unit_id.read().await;
        let user_id_i64 = user_id.get() as i64;
        let owned: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM player_units WHERE user_id = $1")
            .bind(user_id_i64)
            .fetch_one(db)
            .await
            .unwrap_or(0);
        if owned == 0 {
            match sqlx::query!("INSERT INTO player_units (user_id, unit_id, nickname, current_level, current_xp, current_attack, current_defense, current_health, is_in_party, rarity) SELECT $1, u.unit_id, u.name, 1, 0, u.base_attack, u.base_defense, u.base_health, TRUE, u.rarity FROM units u WHERE u.unit_id = $2 ON CONFLICT DO NOTHING", user_id_i64, starter_id).execute(db).await {
                Ok(r) => {
                    if r.rows_affected() > 0 {
                        notes.push(format!("Starter unit {} granted.", starter_id));
                    } else {
                        notes.push("Starter unit already present or invalid starter id.".into());
                    }
                }
                Err(e) => notes.push(format!("Starter grant error: {e}")),
            }
        } else {
            notes.push(format!(
                "Units already owned: {} (no starter grant).",
                owned
            ));
        }
    }
    if notes.is_empty() {
        notes.push("No actions performed.".into());
    }
    // Saga diagnostics (performed after mutation actions so they don't overwrite earlier notes)
    if let Some(target) = diag_user {
        use sqlx::{Error, Row};
        notes.push(format!("-- Saga Diagnostics for user {} --", target));
        // Base profile presence
        match sqlx::query_scalar::<_, i64>("SELECT 1 FROM profiles WHERE user_id = $1")
            .bind(target)
            .fetch_optional(db)
            .await
        {
            Ok(Some(_)) => notes.push("Base profile: PRESENT".into()),
            Ok(None) => {
                notes.push("Base profile: MISSING (run an economy command like /profile)".into())
            }
            Err(e) => notes.push(format!("Base profile query error: {e}")),
        }
        // Saga profile row
        match sqlx::query("SELECT current_ap, max_ap, current_tp, max_tp, story_progress FROM player_saga_profile WHERE user_id = $1")
            .bind(target)
            .fetch_optional(db)
            .await
        {
            Ok(Some(row)) => {
                notes.push(format!(
                    "Saga profile: PRESENT (AP {}/{} | TP {}/{} | Story {})",
                    row.get::<i32,_>(0),
                    row.get::<i32,_>(1),
                    row.get::<i32,_>(2),
                    row.get::<i32,_>(3),
                    row.get::<i32,_>(4)
                ));
            }
            Ok(None) => notes.push("Saga profile: MISSING (first /saga run should auto-create)".into()),
            Err(Error::Database(db_err)) if db_err.code().map(|c| c == "42P01").unwrap_or(false) => {
                notes.push("Saga profile: TABLE MISSING (run migrations: sqlx migrate run)".into());
            }
            Err(e) => notes.push(format!("Saga profile query error: {e}")),
        }
        // Unit count
        match sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM player_units WHERE user_id = $1")
            .bind(target)
            .fetch_one(db)
            .await
        {
            Ok(count) => notes.push(format!("Units owned: {}", count)),
            Err(e) => notes.push(format!("Unit count error: {e}")),
        }
        notes.push("Diagnostic hints: if Saga profile missing but base present, invoke /saga again; if table missing, run migrations; if connection errors persist, verify DATABASE_URL.".into());
    }
    if show_cache {
        let (hits, misses) = crate::services::cache::cache_stats().await;
        let total = hits + misses;
        let hit_rate = if total > 0 {
            (hits as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        notes.push(format!(
            "Cache Stats => hits: {}, misses: {}, hit_rate: {:.1}%",
            hits, misses, hit_rate
        ));
    }
    embed = embed.description(notes.join("\n"));
    let resp = serenity::builder::CreateInteractionResponseMessage::new().embed(embed);
    let _ = interaction
        .create_response(
            &ctx.http,
            serenity::builder::CreateInteractionResponse::Message(resp),
        )
        .await;
}
