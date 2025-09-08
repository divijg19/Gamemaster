//! Contains the UI and logic for the Tavern.
#![allow(unused_mut)]

use crate::AppState;
use crate::database;
use crate::database::models::{Unit, UnitRarity};
use crate::ui::buttons::Btn;
use crate::ui::style::{COLOR_SAGA_TAVERN, EMOJI_COIN};
use ahash::AHasher;
use chrono::{Datelike, Utc};
use serenity::builder::{CreateActionRow, CreateEmbed};
use serenity::model::id::UserId;
use std::hash::Hasher;

// Configurable parameters for the dynamic tavern.
/// Base hire cost used for cost scaling (Common rarity baseline).
pub const HIRE_COST: i64 = 250;
// Base visible recruits per rotation (always shown)
pub const TAVERN_BASE_ROTATION: usize = 5;
// Additional recruits unlocked via story progress milestones (progress >=3, >=6)
pub const TAVERN_UNLOCK_PROGRESS_1: i32 = 3;
pub const TAVERN_UNLOCK_PROGRESS_2: i32 = 6;
// Hard ceiling safeguard (was previously 25)
pub const TAVERN_MAX_DAILY: usize = 25; // still fetch a wider pool for future features / rerolls
pub const TAVERN_REROLL_COST: i64 = 150; // coins per reroll (coins or future token)
pub const TAVERN_MAX_DAILY_REROLLS: i32 = 3;
pub const FAVOR_PER_HIRE: i32 = 5;
pub const FAVOR_TIERS: [i32; 4] = [0, 50, 150, 400];

/// Computes current favor tier index and progress (0..1) toward next tier.
pub fn favor_tier(favor: i32) -> (usize, f32) {
    let mut idx = 0usize;
    for (i, thr) in FAVOR_TIERS.iter().enumerate().rev() {
        if favor >= *thr {
            idx = i;
            break;
        }
    }
    if idx + 1 >= FAVOR_TIERS.len() {
        return (idx, 1.0);
    }
    let cur = FAVOR_TIERS[idx];
    let next = FAVOR_TIERS[idx + 1];
    let frac = (favor - cur) as f32 / (next - cur) as f32;
    (idx, frac.clamp(0.0, 1.0))
}

/// Deterministically produce a shuffled list of today's recruitable units.
/// We derive a stable order based on the current UTC date so all players see the same rotation per day.
pub async fn get_daily_recruits(pool: &sqlx::PgPool) -> Vec<Unit> {
    // Fetch all recruitable units once (could cache behind RwLock if needed later).
    let all = crate::database::units::get_all_units(pool)
        .await
        .unwrap_or_default();
    let mut recruitable: Vec<Unit> = all.into_iter().filter(|u| u.is_recruitable).collect();
    let today = Utc::now().date_naive();
    // Stable deterministic shuffle: hash(date + unit_id) and sort by that.
    recruitable.sort_by_key(|u| {
        let mut h = AHasher::default();
        h.write_i32(today.year());
        h.write_u32(today.ordinal());
        h.write_i32(u.unit_id);
        h.finish()
    });
    // Truncate to daily maximum
    if recruitable.len() > TAVERN_MAX_DAILY {
        recruitable.truncate(TAVERN_MAX_DAILY);
    }
    recruitable
}

/// Helper embed for a successful recruit hire (DRY for interaction handlers)
pub fn recruit_success_embed(
    unit_name: &str,
    unit_cost: i64,
    player_balance_after: i64,
) -> CreateEmbed {
    // Reuse generic success styling then append contextual field.
    let mut embed = crate::ui::style::success_embed(
        "Recruit Hired",
        format!("**{}** joins your forces!", unit_name),
    );
    embed = embed.field(
        "Cost",
        format!(
            "{} {} (Remaining: {} {})",
            EMOJI_COIN, unit_cost, EMOJI_COIN, player_balance_after
        ),
        true,
    );
    embed
}

/// Creates the embed and components for the Tavern menu.
#[derive(Debug, Clone)]
pub struct TavernUiMeta {
    pub balance: i64,
    pub favor: i32,
    pub favor_tier: usize,
    pub favor_progress: f32,
    pub daily_rerolls_used: i32,
    pub max_daily_rerolls: i32,
    pub reroll_cost: i64,
    pub can_reroll: bool,
    // Filter removed from UI; retained implicitly by not exposing buttons.
}

/// Builds the Tavern embed & components given an ordered recruit list and contextual meta.
pub fn create_tavern_menu(
    recruits: &[Unit],
    meta: &TavernUiMeta,
) -> (CreateEmbed, Vec<CreateActionRow>) {
    // Pre-compute some rotation stats
    let avg_cost: i64 = if !recruits.is_empty() {
        (recruits
            .iter()
            .map(|u| hire_cost_for_rarity(u.rarity))
            .sum::<i64>() as f64
            / recruits.len() as f64)
            .round() as i64
    } else {
        0
    };
    let affordable = recruits
        .iter()
        .filter(|u| meta.balance >= hire_cost_for_rarity(u.rarity))
        .count();
    let mut embed = CreateEmbed::new()
        .title("The Weary Dragon Tavern")
        .description(format!("Daily rotation (UTC). Hire mercenaries to grow your forces. **{} / {} affordable** • Avg Cost: {} {}", affordable, recruits.len(), EMOJI_COIN, avg_cost))
        .field("Balance", format!("{} {}", EMOJI_COIN, meta.balance), true)
        .field(
            "Favor",
            format!(
                "{} pts • Tier {}/{} {}\n{}",
                meta.favor,
                meta.favor_tier,
                FAVOR_TIERS.len() - 1,
                favor_bar(meta.favor_progress),
                if meta.favor_tier + 1 < FAVOR_TIERS.len() {
                    format!(
                        "{} to next tier",
                        FAVOR_TIERS[meta.favor_tier + 1] - meta.favor
                    )
                } else {
                    "Max tier reached".to_string()
                }
            ),
            true,
        )
        .field(
            "Rerolls",
            format!(
                "Used {}/{} today (Cost: {} {})",
                meta.daily_rerolls_used, meta.max_daily_rerolls, EMOJI_COIN, meta.reroll_cost
            ),
            true,
        )
    .color(COLOR_SAGA_TAVERN);
    // Pagination removed (rotation <= 7 entries). Show all recruits provided.
    let page_slice = recruits;

    let mut hire_buttons = Vec::new();
    for unit in page_slice {
        let unit_cost = hire_cost_for_rarity(unit.rarity);
        let rarity_icon = rarity_emoji(unit.rarity);
        embed = embed.field(
            format!("{} {} (#{})", rarity_icon, unit.name, unit.unit_id),
            format!(
                "{} Atk:`{}` Def:`{}` HP:`{}` • Cost: {} {} • {} {}{}",
                unit.description.as_deref().unwrap_or(""),
                unit.base_attack,
                unit.base_defense,
                unit.base_health,
                EMOJI_COIN,
                unit_cost,
                rarity_icon,
                rarity_label(unit.rarity),
                if meta.balance >= unit_cost {
                    " ✔"
                } else {
                    ""
                }
            ),
            false,
        );
        let label = if meta.balance < unit_cost {
            "Cannot Afford"
        } else {
            "Hire"
        };
        hire_buttons.push(
            Btn::success(
                &format!("saga_hire_{}", unit.unit_id),
                &format!("➕ {} {}", label, unit.name),
            )
            .disabled(meta.balance < unit_cost),
        );
    }

    // Paging controls (if multiple pages)
    let mut rows: Vec<CreateActionRow> = Vec::new();
    rows.push(crate::commands::saga::ui::global_nav_row("saga"));
    rows.push(CreateActionRow::Buttons(hire_buttons));
    // Reroll button with dynamic label/state
    let left = (meta.max_daily_rerolls - meta.daily_rerolls_used).max(0);
    let reroll_label = if left > 0 {
        format!(
            "🔁 Reroll ({} left • {} {})",
            left, EMOJI_COIN, meta.reroll_cost
        )
    } else {
        "🔁 Reroll (0 left)".to_string()
    };
    rows.push(CreateActionRow::Buttons(vec![
        Btn::secondary("saga_tavern_reroll", &reroll_label)
            .disabled(!meta.can_reroll || meta.balance < meta.reroll_cost || left == 0),
    ]));
    // Rarity filter buttons removed per spec (simplify UX).
    (embed, rows)
}

/// Confirmation embed for a hire attempt.
pub fn create_hire_confirmation(unit: &Unit, player_balance: i64) -> CreateEmbed {
    let unit_cost = hire_cost_for_rarity(unit.rarity);
    CreateEmbed::new()
        .title(format!("Confirm Hire: {}", unit.name))
        .description(unit.description.as_deref().unwrap_or("No description."))
        .field(
            "Stats",
            format!(
                "Atk `{}` | Def `{}` | HP `{}`",
                unit.base_attack, unit.base_defense, unit.base_health
            ),
            false,
        )
        .field(
            "Cost",
            format!(
                "{} {} (You have: {} {})",
                EMOJI_COIN, unit_cost, EMOJI_COIN, player_balance
            ),
            true,
        )
        .color(COLOR_SAGA_TAVERN)
}

/// Build the ordered list of today's recruits for a given user (respecting per-user rerolls)
/// along with the UI meta block (favor, rerolls, balance, etc.).
// (legacy uncached builder removed; use build_tavern_state_cached)
/// Cached variant that uses `AppState.tavern_daily_cache` to avoid re-sorting every call.
pub async fn build_tavern_state_cached(
    app: &AppState,
    user: UserId,
) -> anyhow::Result<(Vec<Unit>, TavernUiMeta)> {
    let balance = database::economy::get_or_create_profile(&app.db, user)
        .await?
        .balance;
    // Fetch cached or compute daily list
    let today = Utc::now().date_naive();
    let global_units: Vec<Unit> = {
        let lock = app.tavern_daily_cache.write().await; // not mut until needed
        if let Some((cached_date, units)) = lock.as_ref() {
            if *cached_date == today {
                units.clone()
            } else {
                drop(lock); // release before recompute
                let fresh = get_daily_recruits(&app.db).await;
                let mut w = app.tavern_daily_cache.write().await;
                *w = Some((today, fresh.clone()));
                fresh
            }
        } else {
            drop(lock);
            let fresh = get_daily_recruits(&app.db).await;
            let mut w = app.tavern_daily_cache.write().await;
            *w = Some((today, fresh.clone()));
            fresh
        }
    };
    let global_ids: Vec<i32> = global_units.iter().map(|u| u.unit_id).collect();
    let rotation_ids =
        database::tavern::get_or_generate_rotation(&app.db, user, &global_ids).await?;
    let map: std::collections::HashMap<i32, Unit> =
        global_units.into_iter().map(|u| (u.unit_id, u)).collect();
    let mut ordered: Vec<Unit> = rotation_ids
        .iter()
        .filter_map(|id| map.get(id).cloned())
        .collect();
    if ordered.is_empty() {
        ordered = map.values().cloned().collect();
    }
    let (favor, daily_rerolls_used, last_reroll) =
        database::tavern::get_or_create_favor(&app.db, user).await?;
    let used_today = if let Some(ts) = last_reroll {
        if ts.date_naive() == today {
            daily_rerolls_used
        } else {
            0
        }
    } else {
        0
    };
    let (tier_idx, prog) = favor_tier(favor);
    let can_reroll = database::tavern::can_reroll(&app.db, user, TAVERN_MAX_DAILY_REROLLS)
        .await
        .unwrap_or(false);
    // Determine story progress to unlock extra recruits (call lightweight profile fetch)
    let story_progress = database::saga::get_story_progress(&app.db, user)
        .await
        .unwrap_or(0);
    let mut visible_cap = TAVERN_BASE_ROTATION;
    if story_progress >= TAVERN_UNLOCK_PROGRESS_1 {
        visible_cap += 1;
    }
    if story_progress >= TAVERN_UNLOCK_PROGRESS_2 {
        visible_cap += 1;
    }
    if ordered.len() > visible_cap {
        ordered.truncate(visible_cap);
    }
    let meta = TavernUiMeta {
        balance,
        favor,
        favor_tier: tier_idx,
        favor_progress: prog,
        daily_rerolls_used: used_today,
        max_daily_rerolls: TAVERN_MAX_DAILY_REROLLS,
        reroll_cost: TAVERN_REROLL_COST,
        can_reroll,
    };
    Ok((ordered, meta))
}

fn favor_bar(frac: f32) -> String {
    let filled = (frac * 10.0).floor() as usize;
    let mut s = String::from("[");
    for i in 0..10 {
        if i < filled {
            s.push('█');
        } else {
            s.push('░');
        }
    }
    s.push(']');
    s
}

/// Provide a short emoji marker for unit rarity for quick scanning.
pub fn rarity_emoji(r: UnitRarity) -> &'static str {
    match r {
        UnitRarity::Common => "⚪",
        UnitRarity::Rare => "🟢",
        UnitRarity::Epic => "🔵",
        UnitRarity::Legendary => "🟣",
        UnitRarity::Unique => "🟡",
        UnitRarity::Mythical => "🔴",
        UnitRarity::Fabled => "🟦",
    }
}

/// Human‑readable rarity label (could later localize or shorten further).
pub fn rarity_label(r: UnitRarity) -> &'static str {
    match r {
        UnitRarity::Common => "Common",
        UnitRarity::Rare => "Rare",
        UnitRarity::Epic => "Epic",
        UnitRarity::Legendary => "Legendary",
        UnitRarity::Unique => "Unique",
        UnitRarity::Mythical => "Mythical",
        UnitRarity::Fabled => "Fabled",
    }
}

/// Multiplier applied to the base hire cost for each rarity tier.
pub fn rarity_cost_multiplier(r: UnitRarity) -> f64 {
    match r {
        UnitRarity::Common => 1.0,
        UnitRarity::Rare => 1.15,
        UnitRarity::Epic => 1.35,
        UnitRarity::Legendary => 1.65,
        UnitRarity::Unique => 1.95,
        UnitRarity::Mythical => 2.25,
        UnitRarity::Fabled => 2.75,
    }
}

/// Compute the hire cost for a given rarity (rounded to nearest 5 for cleaner numbers).
pub fn hire_cost_for_rarity(r: UnitRarity) -> i64 {
    let raw = (HIRE_COST as f64 * rarity_cost_multiplier(r)).round() as i64;
    // Round to nearest 5 to avoid odd values.
    let rem = raw % 5;
    if rem == 0 { raw } else { raw + (5 - rem) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cost_progression_increases_monotonically() {
        let mut last = 0;
        for r in [
            UnitRarity::Common,
            UnitRarity::Rare,
            UnitRarity::Epic,
            UnitRarity::Legendary,
            UnitRarity::Unique,
            UnitRarity::Mythical,
            UnitRarity::Fabled,
        ] {
            let c = hire_cost_for_rarity(r);
            assert!(c >= last, "Cost should not decrease going up rarities");
            last = c;
        }
    }
}

// Session state and rarity filters removed (no pagination or rarity filtering needed after redesign).
