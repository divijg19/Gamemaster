use crate::{AppState, database};
use serenity::builder::{CreateCommand, CreateCommandOption, CreateEmbed, EditInteractionResponse};
use serenity::model::application::{CommandInteraction, CommandOptionType};
use serenity::prelude::Context;

// Lightweight admin utility to exercise maintenance helpers so they stay active.
pub fn register() -> CreateCommand {
    // Previous definition mixed primitive integer options alongside subcommands, which
    // triggers Discord validation errors (APPLICATION_COMMAND_OPTIONS_TYPE_INVALID).
    // We restructure as a pure subcommand interface for clarity & compliance:
    // /adminutil markhuman <unit_id>
    // /adminutil diaguser <user_id>
    // /adminutil bondtest <host_id> <equip_id>
    // /adminutil researchunit <unit_id>
    // /adminutil cachestats
    // /adminutil sagainit
    CreateCommand::new("adminutil")
        .description("Maintenance utilities (owner-only)")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "markhuman",
                "Mark a unit id as Human",
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::Integer, "unit_id", "Unit id to mark")
                    .required(true),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "diaguser",
                "Diagnose saga state for a user id (default: invoking user)",
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::Integer, "user_id", "Target user id")
                    .required(false),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "bondtest",
                "Bond host + equipped player_unit_ids",
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::Integer, "host", "Host player_unit_id")
                    .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "equip",
                    "Equipped player_unit_id",
                )
                .required(true),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "researchunit",
                "Fetch raw research progress for a unit id",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "unit_id",
                    "Unit id to inspect",
                )
                .required(true),
            ),
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
    // Defer ephemerally so we have more than 3s for DB diagnostics.
    if let Err(e) = interaction.defer_ephemeral(&ctx.http).await {
        tracing::error!(target="adminutil", error=?e, "Failed to defer adminutil interaction");
        return;
    }
    let Some(state) = AppState::from_ctx(ctx).await else {
        tracing::error!(target = "adminutil", "AppState missing in context");
        return;
    };
    let db = &state.db;
    let mut embed = CreateEmbed::new().title("Admin Util");
    let mut notes = Vec::new();
    use serenity::model::application::CommandDataOptionValue as Val;

    // Expect exactly one top-level subcommand.
    if let Some(sub) = interaction.data.options.first() {
        match (&sub.name[..], &sub.value) {
            ("markhuman", Val::SubCommand(nested)) => {
                if let Some(arg) = nested.iter().find(|o| o.name == "unit_id") {
                    if let Val::Integer(unit_id_ref) = &arg.value {
                        let unit_id = *unit_id_ref;
                        if unit_id > 0 {
                            match database::units::mark_units_as_human(db, &[unit_id as i32]).await
                            {
                                Ok(n) => notes.push(format!("Marked {} unit(s) as Human", n)),
                                Err(e) => notes.push(format!("Mark error: {}", e)),
                            }
                        }
                    }
                }
            }
            ("diaguser", Val::SubCommand(nested)) => {
                let mut target: i64 = interaction.user.id.get() as i64; // base user id
                if let Some(arg) = nested.iter().find(|o| o.name == "user_id") {
                    if let Val::Integer(user_id_val) = &arg.value {
                        target = *user_id_val;
                    }
                }
                notes.push(format!("-- Saga Diagnostics for user {} --", target));
                use sqlx::Error;
                use sqlx::Row;
                // Cast the literal to BIGINT so it matches the expected i64 Rust type (avoids int4 vs int8 mismatch)
                match sqlx::query_scalar::<_, i64>(
                    "SELECT 1::BIGINT FROM profiles WHERE user_id = $1",
                )
                .bind(target)
                .fetch_optional(db)
                .await
                {
                    Ok(Some(_)) => notes.push("Base profile: PRESENT".into()),
                    Ok(None) => notes.push("Base profile: MISSING".into()),
                    Err(e) => notes.push(format!("Base profile query error: {e}")),
                }
                match sqlx::query("SELECT current_ap, max_ap, current_tp, max_tp, story_progress FROM player_saga_profile WHERE user_id = $1").bind(target).fetch_optional(db).await {
                    Ok(Some(row)) => notes.push(format!("Saga profile: PRESENT (AP {}/{} | TP {}/{} | Story {})", row.get::<i32,_>(0), row.get::<i32,_>(1), row.get::<i32,_>(2), row.get::<i32,_>(3), row.get::<i32,_>(4))),
                    Ok(None) => notes.push("Saga profile: MISSING".into()),
                    Err(Error::Database(db_err)) if db_err.code().map(|c| c=="42P01").unwrap_or(false) => notes.push("Saga profile: TABLE MISSING".into()),
                    Err(e) => notes.push(format!("Saga profile query error: {e}")),
                }
            }
            ("bondtest", Val::SubCommand(nested)) => {
                let mut host: i32 = 0;
                let mut equip: i32 = 0;
                for o in nested {
                    if o.name == "host" {
                        if let Val::Integer(v) = &o.value {
                            host = *v as i32;
                        }
                    } else if o.name == "equip" {
                        if let Val::Integer(v) = &o.value {
                            equip = *v as i32;
                        }
                    }
                }
                if host > 0 && equip > 0 {
                    match database::units::bond_units(db, interaction.user.id, host, equip).await {
                        Ok(_) => notes.push("Bond attempted (see logs if constraints)".into()),
                        Err(e) => notes.push(format!("Bond error: {e}")),
                    }
                }
            }
            ("researchunit", Val::SubCommand(nested)) => {
                if let Some(arg) = nested.iter().find(|o| o.name == "unit_id") {
                    if let Val::Integer(uid_ref) = &arg.value {
                        let uid = *uid_ref;
                        if let Ok(count) = database::units::get_research_progress(
                            db,
                            interaction.user.id,
                            uid as i32,
                        )
                        .await
                        {
                            notes.push(format!("Research progress unit {} = {}", uid, count));
                        }
                    }
                }
            }
            ("cachestats", _) => {
                let (hits, misses) = crate::services::cache::cache_stats().await;
                let total = hits + misses;
                let pct = if total > 0 {
                    (hits as f64 / total as f64) * 100.0
                } else {
                    0.0
                };
                notes.push(format!(
                    "Cache Stats => hits: {}, misses: {}, hit_rate: {:.1}%",
                    hits, misses, pct
                ));
            }
            ("sagainit", _) => {
                tracing::info!(target="adminutil.sagainit", user_id=%interaction.user.id, "Starting saga initialization");
                let user_id = interaction.user.id;
                match database::economy::get_or_create_profile(db, user_id).await {
                    Ok(_) => notes.push("Base profile ensured.".into()),
                    Err(e) => notes.push(format!("Base profile error: {e}")),
                }
                match crate::database::saga::update_and_get_saga_profile(db, user_id).await {
                    Ok(_) => notes.push("Saga profile ensured.".into()),
                    Err(e) => notes.push(format!("Saga profile error: {e}")),
                }
                tracing::info!(target="adminutil.sagainit", user_id=%interaction.user.id, "Completed saga initialization");
            }
            _ => notes.push("Unknown subcommand.".into()),
        }
    }
    if notes.is_empty() {
        notes.push("No subcommand provided. Available: markhuman, diaguser, bondtest, researchunit, cachestats, sagainit".into());
    }
    embed = embed.description(notes.join("\n"));
    let builder = EditInteractionResponse::new().embed(embed);
    if let Err(e) = interaction.edit_response(&ctx.http, builder).await {
        tracing::error!(target="adminutil", error=?e, "Failed editing adminutil response");
    }
}
