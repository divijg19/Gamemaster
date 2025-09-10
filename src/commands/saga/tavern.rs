//! Contains the UI and logic for the Tavern.

use crate::AppState;
use crate::commands::economy::core::item::Item;
use crate::database;
use crate::database::models::{Unit, UnitRarity};
use crate::ui::buttons::Btn;
use crate::ui::style::{COLOR_SAGA_TAVERN, EMOJI_COIN};
use ahash::AHasher;
use chrono::{Datelike, NaiveTime, Utc};
use serenity::builder::{CreateActionRow, CreateEmbed};
use serenity::model::id::UserId;
use std::hash::Hasher; // for rng.random()

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
pub const FAME_PER_HIRE: i32 = 5;
pub const FAME_TIERS: [i32; 4] = [0, 50, 150, 400];

// Daily Shop rotation sizing
pub const SHOP_DAILY_COUNT: usize = 3; // number of Small Arms items shown per day

/// Computes current fame tier index and progress (0..1) toward next tier.
pub fn fame_tier(fame: i32) -> (usize, f32) {
    let mut idx = 0usize;
    for (i, thr) in FAME_TIERS.iter().enumerate().rev() {
        if fame >= *thr {
            idx = i;
            break;
        }
    }
    if idx + 1 >= FAME_TIERS.len() {
        return (idx, 1.0);
    }
    let cur = FAME_TIERS[idx];
    let next = FAME_TIERS[idx + 1];
    let frac = (fame - cur) as f32 / (next - cur) as f32;
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

/// Returns a stable, per-user daily rotation of Small Arms shop items.
/// Deterministic over (user_id, UTC day). Keep the list small to encourage revisits.
pub fn get_daily_shop_items(user: UserId) -> Vec<Item> {
    // Full candidate pool for the Small Arms shop (expand over time)
    let mut all: Vec<Item> = vec![
        Item::GreaterHealthPotion,
        Item::XpBooster,
        Item::ForestContractParchment,
        Item::FrontierContractParchment,
        Item::HealthPotion,
        Item::StaminaDraft,
        Item::FocusTonic,
        Item::TamingLure,
    ];
    let today = Utc::now().date_naive();
    // Stable deterministic shuffle by hashing (user, date, item id)
    all.sort_by_key(|it| {
        let mut h = AHasher::default();
        h.write_u64(user.get());
        h.write_i32(today.year());
        h.write_u32(today.ordinal());
        h.write_i32(*it as i32);
        h.finish()
    });
    if all.len() > SHOP_DAILY_COUNT {
        all.truncate(SHOP_DAILY_COUNT);
    }
    all
}

/// Human-friendly time remaining until next UTC midnight reset, e.g., "5h 12m".
pub fn time_until_reset_str() -> String {
    let now = Utc::now();
    let today = now.date_naive();
    let reset_naive = today
        .succ_opt()
        .unwrap_or(today)
        .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    let reset = chrono::DateTime::<Utc>::from_naive_utc_and_offset(reset_naive, Utc);
    let dur = reset.signed_duration_since(now);
    let total_secs = dur.num_seconds().max(0);
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

/// Fame tier perks: (shop_discount_rate, reroll_discount_rate, extra_visible_recruits)
pub fn fame_perks(tier_idx: usize) -> (f32, f32, usize) {
    match tier_idx {
        0 => (0.0, 0.0, 0),
        1 => (0.05, 0.0, 0),
        2 => (0.10, 0.25, 1),
        _ => (0.15, 0.50, 2),
    }
}

fn round_up_to_5(v: i64) -> i64 {
    if v <= 0 {
        return 1;
    }
    let rem = v % 5;
    if rem == 0 { v } else { v + (5 - rem) }
}

/// Apply a fame-based shop discount and round up to a sensible coin step.
pub fn apply_shop_discount(base_cost: i64, rate: f32) -> i64 {
    let discounted = ((base_cost as f32) * (1.0 - rate)).round() as i64;
    round_up_to_5(discounted).max(1)
}

/// Creates the embed and components for the Tavern menu.
#[derive(Debug, Clone)]
pub struct TavernUiMeta {
    pub balance: i64,
    pub fame: i32,
    pub fame_tier: usize,
    pub fame_progress: f32,
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
        .description(format!(
            "Daily rotation (UTC). Resets in {}.\n**{} / {} affordable** • Avg Cost: {} {}",
            time_until_reset_str(),
            affordable,
            recruits.len(),
            EMOJI_COIN,
            avg_cost
        ))
        .field("Balance", format!("{} {}", EMOJI_COIN, meta.balance), true)
        .field(
            "Fame",
            format!(
                "{} pts • Tier {}/{} {}\n{}",
                meta.fame,
                meta.fame_tier,
                FAME_TIERS.len() - 1,
                fame_bar(meta.fame_progress),
                if meta.fame_tier + 1 < FAME_TIERS.len() {
                    format!(
                        "{} to next tier",
                        FAME_TIERS[meta.fame_tier + 1] - meta.fame
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
        Btn::primary("saga_tavern_home", "🏰 Home"),
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
/// along with the UI meta block (fame, rerolls, balance, etc.).
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
        // Fast path: read lock; only upgrade to write on miss/stale
        let r = app.tavern_daily_cache.read().await;
        if let Some((cached_date, units)) = r.as_ref() {
            if *cached_date == today {
                units.clone()
            } else {
                drop(r);
                let fresh = get_daily_recruits(&app.db).await;
                let mut w = app.tavern_daily_cache.write().await;
                *w = Some((today, fresh.clone()));
                fresh
            }
        } else {
            drop(r);
            let fresh = get_daily_recruits(&app.db).await;
            let mut w = app.tavern_daily_cache.write().await;
            *w = Some((today, fresh.clone()));
            fresh
        }
    };
    // Gate pets until later story/quest progress: exclude pets if story_progress < 5
    let story_progress = database::saga::get_story_progress(&app.db, user)
        .await
        .unwrap_or(0);
    let gated_units: Vec<Unit> = global_units
        .into_iter()
        .filter(|u| {
            if matches!(u.kind, crate::database::models::UnitKind::Pet) {
                story_progress >= 5
            } else {
                true
            }
        })
        .collect();
    let global_ids: Vec<i32> = gated_units.iter().map(|u| u.unit_id).collect();
    let rotation_ids =
        database::tavern::get_or_generate_rotation(&app.db, user, &global_ids).await?;
    let map: std::collections::HashMap<i32, Unit> =
        gated_units.into_iter().map(|u| (u.unit_id, u)).collect();
    let mut ordered: Vec<Unit> = rotation_ids
        .iter()
        .filter_map(|id| map.get(id).cloned())
        .collect();
    if ordered.is_empty() {
        ordered = map.values().cloned().collect();
    }
    let (fame, daily_rerolls_used, last_reroll) =
        database::tavern::get_or_create_fame(&app.db, user).await?;
    let used_today = if let Some(ts) = last_reroll {
        if ts.date_naive() == today {
            daily_rerolls_used
        } else {
            0
        }
    } else {
        0
    };
    let (tier_idx, prog) = fame_tier(fame);
    let can_reroll = database::tavern::can_reroll(&app.db, user, TAVERN_MAX_DAILY_REROLLS)
        .await
        .unwrap_or(false);
    let (_shop_disc, reroll_disc, extra_visible) = fame_perks(tier_idx);
    // Bias rarity based on story progress: later progress slightly increases chance of higher rarities in rotation order.
    // Make this ordering deterministic per-user-per-day by using a seeded hash as jitter instead of RNG.
    {
        // Weighting by rarity tier index + story factor
        let weight_for = |r: UnitRarity| -> f32 {
            let base = match r {
                UnitRarity::Common => 1.0,
                UnitRarity::Rare => 1.1,
                UnitRarity::Epic => 1.2,
                UnitRarity::Legendary => 1.3,
                UnitRarity::Unique => 1.4,
                UnitRarity::Mythical => 1.5,
                UnitRarity::Fabled => 1.6,
            };
            let prog_bonus = (story_progress as f32 / 20.0).min(0.6); // up to +0.6
            base + prog_bonus
        };
        let uid = user.get();
        let year = today.year();
        let ordinal = today.ordinal();

        // Deterministic jitter in [0,1) per (user, day, unit)
        let jitter_for = |unit_id: i32| -> f32 {
            let mut h = AHasher::default();
            h.write_u64(uid);
            h.write_i32(year);
            h.write_u32(ordinal);
            h.write_i32(unit_id);
            let v = h.finish();
            // Map u64 to (0,1)
            (v as f64 / (u64::MAX as f64)) as f32
        };

        ordered.sort_by(|a, b| {
            let wa = weight_for(a.rarity);
            let wb = weight_for(b.rarity);
            let ja = jitter_for(a.unit_id);
            let jb = jitter_for(b.unit_id);
            let sa = ja * wa;
            let sb = jb * wb;
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    let mut visible_cap = TAVERN_BASE_ROTATION + extra_visible;
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
        fame,
        fame_tier: tier_idx,
        fame_progress: prog,
        daily_rerolls_used: used_today,
        max_daily_rerolls: TAVERN_MAX_DAILY_REROLLS,
        reroll_cost: apply_shop_discount(TAVERN_REROLL_COST, reroll_disc),
        can_reroll,
    };
    Ok((ordered, meta))
}

fn fame_bar(frac: f32) -> String {
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

    #[test]
    fn discount_application_is_sane() {
        // 10% off 100 -> 90 (already multiple of 5)
        assert_eq!(apply_shop_discount(100, 0.10), 90);
        // 15% off 101 -> 85.85 -> 86 -> round up to 90
        assert_eq!(apply_shop_discount(101, 0.15), 90);
        // Never below 1
        assert_eq!(apply_shop_discount(1, 0.90), 1);
    }
}

// Session state and rarity filters removed (no pagination or rarity filtering needed after redesign).
