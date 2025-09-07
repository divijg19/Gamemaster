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
pub const HIRE_COST: i64 = 250;
pub const TAVERN_PAGE_SIZE: usize = 5;
pub const TAVERN_MAX_DAILY: usize = 25; // cap the number of daily shuffled recruits surfaced
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
pub fn recruit_success_embed(unit_name: &str, player_balance_after: i64) -> CreateEmbed {
    // Reuse generic success styling then append contextual field.
    let mut embed = crate::ui::style::success_embed(
        "Recruit Hired",
        format!("**{}** joins your forces!", unit_name),
    );
    embed = embed.field(
        "Cost",
        format!(
            "{} {} (Remaining: {} {})",
            EMOJI_COIN, HIRE_COST, EMOJI_COIN, player_balance_after
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
    pub filter: TavernFilter,
}

/// Builds the Tavern embed & components given an ordered recruit list and contextual meta.
pub fn create_tavern_menu(
    recruits: &[Unit],
    meta: &TavernUiMeta,
    page: usize,
) -> (CreateEmbed, Vec<CreateActionRow>) {
    let mut embed = CreateEmbed::new()
        .title("The Weary Dragon Tavern")
        .description("The air is thick with the smell of stale ale and adventure. A rotating cast of mercenaries seeks coin. New rotation every day (UTC). Click a Hire button for details & confirmation.")
        .field("Your Balance", format!("{} {}", EMOJI_COIN, meta.balance), true)
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
    let total_pages = recruits.len().div_ceil(TAVERN_PAGE_SIZE);
    let page = page.min(total_pages.saturating_sub(1));
    let start = page * TAVERN_PAGE_SIZE;
    let end = (start + TAVERN_PAGE_SIZE).min(recruits.len());
    let page_slice = &recruits[start..end];

    let mut hire_buttons = Vec::new();
    for unit in page_slice {
        embed = embed.field(
            format!("{} (#{})", unit.name, unit.unit_id),
            format!(
                "{} Atk:`{}` Def:`{}` HP:`{}` | Cost: {} {} | Rarity: {:?}",
                unit.description.as_deref().unwrap_or(""),
                unit.base_attack,
                unit.base_defense,
                unit.base_health,
                EMOJI_COIN,
                HIRE_COST,
                unit.rarity
            ),
            false,
        );
        let label = if meta.balance < HIRE_COST {
            "Cannot Afford"
        } else {
            "Hire"
        };
        hire_buttons.push(
            Btn::success(
                &format!("saga_hire_{}", unit.unit_id),
                &format!("➕ {} {}", label, unit.name),
            )
            .disabled(meta.balance < HIRE_COST),
        );
    }

    // Paging controls (if multiple pages)
    let mut rows: Vec<CreateActionRow> = Vec::new();
    rows.push(crate::commands::saga::ui::global_nav_row("saga"));
    if total_pages > 1 {
        rows.push(CreateActionRow::Buttons(vec![
            Btn::secondary(
                &format!("saga_tavern_page_{}", page.saturating_sub(1)),
                "◀ Prev",
            )
            .disabled(page == 0),
            Btn::secondary(
                "noop_tavern_page",
                &format!("Page {}/{}", page + 1, total_pages),
            )
            .disabled(true),
            Btn::secondary(
                &format!(
                    "saga_tavern_page_{}",
                    (page + 1).min(total_pages.saturating_sub(1))
                ),
                "Next ▶",
            )
            .disabled(page + 1 >= total_pages),
        ]));
    }
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
    // Filter buttons row
    rows.push(CreateActionRow::Buttons(filter_buttons(meta.filter)));
    (embed, rows)
}

/// Confirmation embed for a hire attempt.
pub fn create_hire_confirmation(unit: &Unit, player_balance: i64) -> CreateEmbed {
    
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
                EMOJI_COIN, HIRE_COST, EMOJI_COIN, player_balance
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
    let meta = TavernUiMeta {
        balance,
        favor,
        favor_tier: tier_idx,
        favor_progress: prog,
        daily_rerolls_used: used_today,
        max_daily_rerolls: TAVERN_MAX_DAILY_REROLLS,
        reroll_cost: TAVERN_REROLL_COST,
        can_reroll,
        filter: TavernFilter::All,
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

// ---------------- Filters & Session State (foundation) ----------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TavernFilter {
    All,
    RarePlus,
    EpicPlus,
    LegendaryPlus,
}

impl TavernFilter {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::RarePlus => "Rare+",
            Self::EpicPlus => "Epic+",
            Self::LegendaryPlus => "Legendary+",
        }
    }
}

/// Build the row of filter buttons marking the active one disabled.
pub fn filter_buttons(active: TavernFilter) -> Vec<serenity::builder::CreateButton> {
    use TavernFilter::*;
    let variants = [All, RarePlus, EpicPlus, LegendaryPlus];
    variants
        .iter()
        .map(|f| {
            let id = format!("saga_tavern_filter_{:?}", f).to_lowercase();
            let mut btn = Btn::secondary(&id, f.label());
            if *f == active {
                btn = btn.disabled(true);
            }
            btn
        })
        .collect()
}

/// Filter units into an owned Vec based on filter rarity threshold.
pub fn filter_units(units: &[Unit], filter: TavernFilter) -> Vec<Unit> {
    use TavernFilter::*;
    match filter {
        All => units.to_vec(),
        RarePlus => units
            .iter()
            .filter(|u| u.rarity >= UnitRarity::Rare)
            .cloned()
            .collect(),
        EpicPlus => units
            .iter()
            .filter(|u| u.rarity >= UnitRarity::Epic)
            .cloned()
            .collect(),
        LegendaryPlus => units
            .iter()
            .filter(|u| u.rarity >= UnitRarity::Legendary)
            .cloned()
            .collect(),
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TavernSessionState {
    pub page: usize,
    pub filter: TavernFilter,
}

impl Default for TavernSessionState {
    fn default() -> Self {
        Self {
            page: 0,
            filter: TavernFilter::All,
        }
    }
}
