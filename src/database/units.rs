//! Unit database API (post Phase B). Legacy `pets.rs` logic has been fully inlined here.
//! Remaining file `pets.rs` is now deprecated and will be removed after verification.

use serenity::model::id::UserId;
use sqlx::types::chrono::Utc;
use sqlx::{PgPool, Postgres, Transaction};
use tracing::{instrument, warn};

use super::economy::{add_balance, add_to_inventory, get_inventory_item};
use crate::commands::economy::core::item::Item;
pub use crate::database::models::{PlayerUnit, Profile, Unit, UnitKind, UnitRarity};
use crate::saga;
use crate::saga::leveling::LevelUpResult;
// TEMP: Re-export legacy bonding until pets.rs fully removed
// Legacy pets module is being retired; bonding logic fully inlined below.

// -------------------------------------------------------------------------------------------------
// Core retrieval
// -------------------------------------------------------------------------------------------------
#[instrument(level = "debug", skip(pool))]
pub async fn get_player_units(
    pool: &PgPool,
    user_id: UserId,
) -> Result<Vec<PlayerUnit>, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    sqlx::query_as!(
        PlayerUnit,
        r#"SELECT
        pu.player_unit_id, pu.user_id, pu.unit_id, pu.nickname, pu.current_level, pu.current_xp,
        pu.current_attack, pu.current_defense, pu.current_health, pu.is_in_party, pu.is_training,
        pu.training_stat, pu.training_ends_at, u.name, pu.rarity as "rarity: UnitRarity"
        FROM player_units pu JOIN units u ON pu.unit_id = u.unit_id
        WHERE pu.user_id = $1
        ORDER BY pu.is_in_party DESC, pu.current_level DESC"#,
        user_id_i64
    )
    .fetch_all(pool)
    .await
}

#[instrument(level = "debug", skip(pool))]
pub async fn get_user_party(
    pool: &PgPool,
    user_id: UserId,
) -> Result<Vec<PlayerUnit>, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    sqlx::query_as!(
        PlayerUnit,
        r#"SELECT
        pu.player_unit_id, pu.user_id, pu.unit_id, pu.nickname, pu.current_level, pu.current_xp,
        pu.current_attack, pu.current_defense, pu.current_health, pu.is_in_party, pu.is_training,
        pu.training_stat, pu.training_ends_at, u.name, pu.rarity as "rarity: UnitRarity"
        FROM player_units pu JOIN units u ON pu.unit_id = u.unit_id
        WHERE pu.user_id = $1 AND pu.is_in_party = TRUE
        ORDER BY pu.player_unit_id"#,
        user_id_i64
    )
    .fetch_all(pool)
    .await
}

#[instrument(level = "debug", skip(pool, unit_ids), fields(count = unit_ids.len()))]
pub async fn get_units_by_ids(pool: &PgPool, unit_ids: &[i32]) -> Result<Vec<Unit>, sqlx::Error> {
    sqlx::query_as!(Unit, "SELECT unit_id, name, description, base_attack, base_defense, base_health, is_recruitable, kind as \"kind: UnitKind\", rarity as \"rarity: UnitRarity\" FROM units WHERE unit_id = ANY($1)", unit_ids)
		.fetch_all(pool)
		.await
}

#[instrument(level = "debug", skip(pool))]
pub async fn get_all_units(pool: &PgPool) -> Result<Vec<Unit>, sqlx::Error> {
    sqlx::query_as!(Unit, "SELECT unit_id, name, description, base_attack, base_defense, base_health, is_recruitable, kind as \"kind: UnitKind\", rarity as \"rarity: UnitRarity\" FROM units ORDER BY unit_id")
        .fetch_all(pool)
        .await
}

// -------------------------------------------------------------------------------------------------
// Recruiting / Hiring
// -------------------------------------------------------------------------------------------------
#[instrument(level = "trace", skip(pool))]
pub async fn can_afford_recruit(pool: &PgPool, user_id: UserId) -> Result<bool, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    let lure_item_id = Item::TamingLure as i32;
    let count = sqlx::query_scalar!(
        "SELECT quantity FROM inventories WHERE user_id = $1 AND item_id = $2",
        user_id_i64,
        lure_item_id
    )
    .fetch_optional(pool)
    .await?
    .unwrap_or(0);
    Ok(count >= 1)
}

#[instrument(level = "debug", skip(pool), fields(unit_id, cost))]
pub async fn hire_unit(
    pool: &PgPool,
    user_id: UserId,
    unit_id: i32,
    cost: i64,
) -> Result<String, String> {
    let user_id_i64 = user_id.get() as i64;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    // Serialize concurrent hire/recruit for this user to avoid race on army size via advisory lock
    sqlx::query!("SELECT pg_advisory_xact_lock($1)", user_id_i64)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    let profile = sqlx::query_as!(Profile, "SELECT balance, last_work, work_streak, fishing_xp, fishing_level, mining_xp, mining_level, coding_xp, coding_level FROM profiles WHERE user_id = $1 FOR UPDATE", user_id_i64).fetch_one(&mut *tx).await.map_err(|_| "Could not find your profile.".to_string())?;
    if profile.balance < cost {
        tx.rollback().await.ok();
        return Err("You don't have enough coins.".to_string());
    }
    let army_size: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM player_units WHERE user_id = $1",
        user_id_i64
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap_or(Some(0))
    .unwrap_or(0);
    if army_size >= crate::constants::MAX_ARMY_SIZE {
        tx.rollback().await.ok();
        return Err(format!(
            "Your army is full ({}/{})",
            army_size,
            crate::constants::MAX_ARMY_SIZE
        ));
    }
    let unit_master = sqlx::query_as!(Unit, "SELECT unit_id, name, description, base_attack, base_defense, base_health, is_recruitable, kind as \"kind: UnitKind\", rarity as \"rarity: UnitRarity\" FROM units WHERE unit_id = $1", unit_id)
		.fetch_one(&mut *tx)
		.await
		.map_err(|_| "This mercenary is no longer available.".to_string())?;
    add_balance(&mut tx, user_id, -cost)
        .await
        .map_err(|_| "Failed to process payment.".to_string())?;
    // Decide initial party inclusion:
    // Humans: auto-join party if space; Pets: must satisfy Legendary+ (handled elsewhere on explicit set) so default FALSE here.
    let mut is_in_party = false;
    if matches!(unit_master.kind, UnitKind::Human) {
        let party_size: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM player_units WHERE user_id = $1 AND is_in_party = TRUE",
            user_id_i64
        )
        .fetch_one(&mut *tx)
        .await
        .unwrap_or(Some(0))
        .unwrap_or(0);
        if party_size < crate::constants::MAX_PARTY_SIZE {
            is_in_party = true;
        }
    }
    sqlx::query!("INSERT INTO player_units (user_id, unit_id, nickname, current_attack, current_defense, current_health, rarity, is_in_party) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)", user_id_i64, unit_id, &unit_master.name, unit_master.base_attack, unit_master.base_defense, unit_master.base_health, unit_master.rarity as _, is_in_party).execute(&mut *tx).await.map_err(|_| "Failed to add unit to your army.".to_string())?;
    tx.commit()
        .await
        .map_err(|_| "Failed to finalize the transaction.".to_string())?;
    Ok(unit_master.name)
}

#[instrument(level = "debug", skip(pool), fields(unit_id = unit_id_to_recruit))]
pub async fn attempt_recruit_unit(
    pool: &PgPool,
    user_id: UserId,
    unit_id_to_recruit: i32,
) -> Result<String, String> {
    let user_id_i64 = user_id.get() as i64;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    // Advisory lock to prevent parallel recruit races for same user
    sqlx::query!("SELECT pg_advisory_xact_lock($1)", user_id_i64)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    let unit_master = sqlx::query_as!(Unit, "SELECT unit_id, name, description, base_attack, base_defense, base_health, is_recruitable, kind as \"kind: UnitKind\", rarity as \"rarity: UnitRarity\" FROM units WHERE unit_id = $1", unit_id_to_recruit)
		.fetch_one(&mut *tx)
		.await
		.map_err(|_| "Creature data not found.".to_string())?;
    if !unit_master.is_recruitable {
        tx.rollback().await.ok();
        return Err("This unit cannot be recruited.".to_string());
    }
    // Humans cannot be tamed via the pet research/taming path. They require contracts / gold / special events.
    if matches!(unit_master.kind, UnitKind::Human) {
        tx.rollback().await.ok();
        return Err(
            "Humans can't be tamed. Defeat them to unlock a contract or hire them in town."
                .to_string(),
        );
    }
    let is_party_eligible = match unit_master.kind {
        UnitKind::Human => true, // Humans always eligible (subject to party size)
        UnitKind::Pet => matches!(
            unit_master.rarity,
            UnitRarity::Legendary | UnitRarity::Unique | UnitRarity::Mythical | UnitRarity::Fabled
        ),
    };
    if is_party_eligible {
        let army_size: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM player_units WHERE user_id = $1",
            user_id_i64
        )
        .fetch_one(&mut *tx)
        .await
        .unwrap_or(Some(0))
        .unwrap_or(0);
        if army_size >= crate::constants::MAX_ARMY_SIZE {
            warn!(target: "units", army_full = true, army_size, limit = crate::constants::MAX_ARMY_SIZE, "Recruit blocked: army full");
            tx.rollback().await.ok();
            return Err(format!(
                "Your army is full! ({} / {}). Dismiss a unit first.",
                army_size,
                crate::constants::MAX_ARMY_SIZE
            ));
        }
    }
    // PET TAMING MATERIALS (only pets reach this point). Research Data represents accumulated study from wild battles.
    if let Some(research_item) = Item::research_item_for_unit(&unit_master.name) {
        let requirements = [(Item::TamingLure, 1_i64), (research_item, 10_i64)];
        if let Err(e) = verify_and_consume_items(&mut tx, user_id, &requirements).await {
            tx.rollback().await.ok();
            return Err(e);
        }
    } else {
        tx.rollback().await.ok();
        return Err("This creature cannot currently be researched/tamed.".into());
    }
    // Sub-Legendary path: increment research & NO direct army add (treated as tamed specimen for bonuses).
    if !is_party_eligible {
        crate::database::units::increment_research_progress(pool, user_id, unit_id_to_recruit)
            .await
            .map_err(|_| "Failed to record research progress.".to_string())?;
        // Success message still returns name but indicates research instead of recruit.
        tx.commit().await.ok();
        return Ok(format!(
            "{} tamed (+1 research progress).",
            unit_master.name
        ));
    }
    sqlx::query!("INSERT INTO player_units (user_id, unit_id, nickname, current_attack, current_defense, current_health, rarity, is_in_party) VALUES ($1, $2, $3, $4, $5, $6, $7, TRUE)", user_id_i64, unit_id_to_recruit, &unit_master.name, unit_master.base_attack, unit_master.base_defense, unit_master.base_health, unit_master.rarity as _).execute(&mut *tx).await.map_err(|_| "Failed to add the recruited unit to your army.".to_string())?;
    tx.commit()
        .await
        .map_err(|_| "Failed to finalize the transaction.".to_string())?;
    Ok(unit_master.name)
}

// -------------------------------------------------------------------------------------------------
// Bonding / Equippable System (inlined from legacy pets.rs)
// -------------------------------------------------------------------------------------------------
#[instrument(level = "debug", skip(pool, user_id), fields(host = host_player_unit_id, equipped = equipped_player_unit_id))]
pub async fn bond_unit_as_equippable(
    pool: &PgPool,
    user_id: UserId,
    host_player_unit_id: i32,
    equipped_player_unit_id: i32,
) -> Result<(), String> {
    if host_player_unit_id == equipped_player_unit_id {
        return Err("Cannot bond a unit to itself.".into());
    }
    let user_id_i64 = user_id.get() as i64;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    // Lock rows for host + equipped
    let host = sqlx::query!(
        "SELECT player_unit_id, rarity::text as rarity_text FROM player_units WHERE player_unit_id = $1 AND user_id = $2 FOR UPDATE",
        host_player_unit_id,
        user_id_i64
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| "Host unit not found.".to_string())?;
    let equipped = sqlx::query!(
        "SELECT player_unit_id, rarity::text as rarity_text, is_in_party FROM player_units WHERE player_unit_id = $1 AND user_id = $2 FOR UPDATE",
        equipped_player_unit_id,
        user_id_i64
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| "Equippable unit not found.".to_string())?;
    let rarity_rank = |r: &str| match r {
        "Common" => 1,
        "Rare" => 2,
        "Epic" => 3,
        "Legendary" => 4,
        "Unique" => 5,
        "Mythical" => 6,
        "Fabled" => 7,
        _ => 0,
    };
    if rarity_rank(equipped.rarity_text.as_deref().unwrap_or(""))
        > rarity_rank(host.rarity_text.as_deref().unwrap_or(""))
    {
        tx.rollback().await.ok();
        return Err("Equipped unit's rarity exceeds host unit's rarity.".into());
    }
    let host_has = sqlx::query_scalar!(
        "SELECT 1 FROM equippable_unit_bonds WHERE host_player_unit_id = $1",
        host_player_unit_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    if host_has.is_some() {
        tx.rollback().await.ok();
        return Err("Host already has an equipped unit.".into());
    }
    let equipped_used = sqlx::query_scalar!(
        "SELECT 1 FROM equippable_unit_bonds WHERE equipped_player_unit_id = $1",
        equipped_player_unit_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    if equipped_used.is_some() {
        tx.rollback().await.ok();
        return Err("That unit is already bonded elsewhere.".into());
    }
    sqlx::query!("INSERT INTO equippable_unit_bonds (host_player_unit_id, equipped_player_unit_id, is_equipped) VALUES ($1,$2, TRUE)", host_player_unit_id, equipped_player_unit_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| "Failed to create bond.".to_string())?;
    sqlx::query!(
        "UPDATE player_units SET is_in_party = FALSE WHERE player_unit_id = $1",
        equipped_player_unit_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| "Failed to update equipped unit state.".to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[instrument(level = "debug", skip(pool, user_id), fields(host = host_player_unit_id))]
pub async fn unequip_equippable(
    pool: &PgPool,
    user_id: UserId,
    host_player_unit_id: i32,
) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let owned = sqlx::query_scalar!(
        "SELECT 1 FROM player_units WHERE player_unit_id = $1 AND user_id = $2",
        host_player_unit_id,
        user_id.get() as i64
    )
    .fetch_optional(&mut *tx)
    .await?;
    if owned.is_none() {
        tx.rollback().await.ok();
        return Ok(false);
    }
    let updated = sqlx::query!("UPDATE equippable_unit_bonds SET is_equipped = FALSE WHERE host_player_unit_id = $1 AND is_equipped = TRUE", host_player_unit_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    tx.commit().await.ok();
    Ok(updated > 0)
}

#[instrument(level = "trace", skip(pool, user_id))]
pub async fn get_equipment_bonuses(
    pool: &PgPool,
    user_id: UserId,
) -> Result<std::collections::HashMap<i32, (i32, i32, i32)>, sqlx::Error> {
    use std::collections::HashMap;
    let mut bonuses = HashMap::new();
    let rows = sqlx::query!(r#"SELECT b.host_player_unit_id,
            eu.current_attack as equipped_attack, eu.current_defense as equipped_defense, eu.current_health as equipped_health,
            eu.current_level as equipped_level, eu.rarity as "equipped_rarity: UnitRarity"
        FROM equippable_unit_bonds b
        JOIN player_units eu ON eu.player_unit_id = b.equipped_player_unit_id
        WHERE eu.user_id = $1 AND b.is_equipped = TRUE"#, user_id.get() as i64).fetch_all(pool).await?;
    for row in rows {
        let rarity_mult = match row.equipped_rarity as i32 {
            0 => 0.05,
            1 => 0.08,
            2 => 0.12,
            3 => 0.18,
            4 => 0.24,
            5 => 0.30,
            6 => 0.40,
            _ => 0.05,
        };
        let level_factor = (row.equipped_level as f32).sqrt() / 10.0;
        let base_factor = rarity_mult + level_factor;
        let bonus_attack = ((row.equipped_attack as f32) * base_factor).ceil() as i32;
        let bonus_defense = ((row.equipped_defense as f32) * base_factor * 0.8).ceil() as i32;
        let bonus_health = ((row.equipped_health as f32) * base_factor * 1.2).ceil() as i32;
        bonuses.insert(
            row.host_player_unit_id,
            (bonus_attack, bonus_defense, bonus_health),
        );
    }
    Ok(bonuses)
}

// -------------------------------------------------------------------------------------------------
// Training & Party Management
// -------------------------------------------------------------------------------------------------
async fn spend_training_points(
    tx: &mut Transaction<'_, Postgres>,
    user_id: UserId,
    amount: i32,
) -> Result<bool, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    let rows_affected = sqlx::query!("UPDATE player_saga_profile SET current_tp = current_tp - $1 WHERE user_id = $2 AND current_tp >= $1", amount, user_id_i64).execute(&mut **tx).await?.rows_affected();
    Ok(rows_affected > 0)
}

#[instrument(level = "info", skip(pool))]
pub async fn start_training(
    pool: &PgPool,
    user_id: UserId,
    player_unit_id: i32,
    stat_to_train: &str,
    duration_hours: i64,
    tp_cost: i32,
) -> Result<bool, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    let mut tx = pool.begin().await?;
    if !spend_training_points(&mut tx, user_id, tp_cost).await? {
        tx.rollback().await?;
        return Ok(false);
    }
    let training_ends = Utc::now() + chrono::Duration::hours(duration_hours);
    let rows_affected = sqlx::query!("UPDATE player_units SET is_training = TRUE, training_stat = $1, training_ends_at = $2 WHERE player_unit_id = $3 AND user_id = $4", stat_to_train, training_ends, player_unit_id, user_id_i64).execute(&mut *tx).await?.rows_affected();
    if rows_affected > 0 {
        tx.commit().await?;
        Ok(true)
    } else {
        tx.rollback().await?;
        Ok(false)
    }
}

#[instrument(level = "info", skip(pool))]
pub async fn set_unit_party_status(
    pool: &PgPool,
    user_id: UserId,
    player_unit_id: i32,
    is_in_party: bool,
) -> Result<bool, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    let mut tx = pool.begin().await?;
    if is_in_party {
        // Enforce: Pets must be Legendary+; Humans unrestricted by rarity.
        if let Some(row) = sqlx::query!("SELECT u.kind::text as kind_text, pu.rarity as \"rarity: UnitRarity\" FROM player_units pu JOIN units u ON u.unit_id = pu.unit_id WHERE pu.player_unit_id = $1 AND pu.user_id = $2", player_unit_id, user_id_i64).fetch_optional(&mut *tx).await? {
            if let Some(kind_str) = row.kind_text.as_ref() {
                if kind_str == "Pet" {
                    match row.rarity {
                        UnitRarity::Legendary | UnitRarity::Unique | UnitRarity::Mythical | UnitRarity::Fabled => {}
                        _ => { tx.rollback().await?; return Ok(false); }
                    }
                }
            } else { tx.rollback().await?; return Ok(false); }
        } else { tx.rollback().await?; return Ok(false); }
        let party_size: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM player_units WHERE user_id = $1 AND is_in_party = TRUE",
            user_id_i64
        )
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or(0);
        if party_size >= crate::constants::MAX_PARTY_SIZE {
            tx.rollback().await?;
            return Ok(false);
        }
    }
    let rows_affected = sqlx::query!(
        "UPDATE player_units SET is_in_party = $1 WHERE player_unit_id = $2 AND user_id = $3",
        is_in_party,
        player_unit_id,
        user_id_i64
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if rows_affected > 0 {
        tx.commit().await?;
        Ok(true)
    } else {
        tx.rollback().await?;
        Ok(false)
    }
}

// -------------------------------------------------------------------------------------------------
// Research / Taming Progress (sub-Legendary tames accumulate research milestones)
// -------------------------------------------------------------------------------------------------
#[instrument(level = "debug", skip(pool))]
pub async fn increment_research_progress(
    pool: &PgPool,
    user_id: UserId,
    unit_id: i32,
) -> Result<i32, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    let rec = sqlx::query!("INSERT INTO unit_research_progress (user_id, unit_id, tamed_count) VALUES ($1,$2,1) ON CONFLICT (user_id, unit_id) DO UPDATE SET tamed_count = unit_research_progress.tamed_count + 1, last_updated = NOW() RETURNING tamed_count", user_id_i64, unit_id).fetch_one(pool).await?;
    Ok(rec.tamed_count)
}

#[instrument(level = "debug", skip(pool))]
pub async fn get_research_progress(
    pool: &PgPool,
    user_id: UserId,
    unit_id: i32,
) -> Result<i32, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    Ok(sqlx::query_scalar!(
        "SELECT tamed_count FROM unit_research_progress WHERE user_id = $1 AND unit_id = $2",
        user_id_i64,
        unit_id
    )
    .fetch_optional(pool)
    .await?
    .unwrap_or(0))
}

// Bulk list of research progress for UI (unit_id -> tamed_count)
#[instrument(level = "debug", skip(pool))]
pub async fn list_research_progress(
    pool: &PgPool,
    user_id: UserId,
) -> Result<Vec<(i32, i32)>, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    let rows = sqlx::query!(
        "SELECT unit_id, tamed_count FROM unit_research_progress WHERE user_id = $1",
        user_id_i64
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| (r.unit_id, r.tamed_count))
        .collect())
}

/// Determine research target for a given rarity (configurable through bot_config keys like research_target_common).
pub async fn research_target_for_rarity(pool: &PgPool, rarity: UnitRarity) -> i32 {
    let (key, default) = match rarity {
        UnitRarity::Common => ("research_target_common", 5),
        UnitRarity::Rare => ("research_target_rare", 10),
        UnitRarity::Epic => ("research_target_epic", 18),
        UnitRarity::Legendary | UnitRarity::Unique | UnitRarity::Mythical | UnitRarity::Fabled => {
            ("research_target_high", 0)
        }
    };
    if let Ok(Some(val)) = crate::database::settings::get_config_value(pool, key).await
        && let Ok(parsed) = val.parse::<i32>()
    {
        return parsed;
    }
    default
}

/// Cached variant (20s TTL) via AppState.research_cache
pub async fn list_research_progress_cached(
    app_state: &crate::AppState,
    user_id: UserId,
) -> Result<Vec<(i32, i32)>, sqlx::Error> {
    use std::time::Duration;
    const TTL: Duration = Duration::from_secs(20);
    if let Some(v) =
        crate::services::cache::get_with_ttl(&app_state.research_cache, &user_id.get(), TTL).await
    {
        return Ok(v);
    }
    let fresh = list_research_progress(&app_state.db, user_id).await?;
    crate::services::cache::insert(&app_state.research_cache, user_id.get(), fresh.clone()).await;
    Ok(fresh)
}

// One-off maintenance helper: mark provided unit_ids as Human kind (idempotent)
#[instrument(level = "debug", skip(pool, unit_ids))]
pub async fn mark_units_as_human(pool: &PgPool, unit_ids: &[i32]) -> Result<u64, sqlx::Error> {
    if unit_ids.is_empty() {
        return Ok(0);
    }
    let updated = sqlx::query!(
        "UPDATE units SET kind = 'Human' WHERE unit_id = ANY($1)",
        unit_ids
    )
    .execute(pool)
    .await?
    .rows_affected();
    Ok(updated)
}

// -------------------------------------------------------------------------------------------------
// Battle rewards / leveling
// -------------------------------------------------------------------------------------------------
#[instrument(level = "debug", skip(pool, loot, units_in_battle), fields(loot_items = loot.len(), units = units_in_battle.len(), coins))]
pub async fn apply_battle_rewards(
    pool: &PgPool,
    user_id: UserId,
    coins: i64,
    loot: &[(Item, i64)],
    units_in_battle: &[PlayerUnit],
    xp_per_unit: i32,
) -> Result<Vec<LevelUpResult>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    if coins > 0 {
        add_balance(&mut tx, user_id, coins).await?;
    }
    for (item, quantity) in loot {
        add_to_inventory(&mut tx, user_id, *item, *quantity).await?;
    }
    let mut level_up_results = Vec::new();
    for unit in units_in_battle {
        let level_result = saga::leveling::handle_unit_leveling(unit, xp_per_unit);
        if level_result.did_level_up {
            sqlx::query!("UPDATE player_units SET current_level = $1, current_xp = $2, current_attack = current_attack + $3, current_defense = current_defense + $4, current_health = current_health + $5 WHERE player_unit_id = $6", level_result.new_level, level_result.new_xp, level_result.stat_gains.0, level_result.stat_gains.1, level_result.stat_gains.2, unit.player_unit_id).execute(&mut *tx).await?;
        } else {
            sqlx::query!(
                "UPDATE player_units SET current_xp = $1 WHERE player_unit_id = $2",
                level_result.new_xp,
                unit.player_unit_id
            )
            .execute(&mut *tx)
            .await?;
        }
        level_up_results.push(level_result);
    }
    tx.commit().await?;
    Ok(level_up_results)
}

#[instrument(level = "info", skip(pool))]
pub async fn dismiss_unit(
    pool: &PgPool,
    user_id: UserId,
    player_unit_id: i32,
) -> Result<bool, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    let rows_affected = sqlx::query!(
        "DELETE FROM player_units WHERE player_unit_id = $1 AND user_id = $2",
        player_unit_id,
        user_id_i64
    )
    .execute(pool)
    .await?
    .rows_affected();
    Ok(rows_affected > 0)
}

// -------------------------------------------------------------------------------------------------
// Bonding / Equippables
// -------------------------------------------------------------------------------------------------
#[instrument(
    level = "debug",
    skip(pool),
    fields(host_player_unit_id, equipped_player_unit_id)
)]
pub async fn bond_units(
    pool: &PgPool,
    user_id: UserId,
    host_player_unit_id: i32,
    equipped_player_unit_id: i32,
) -> Result<(), String> {
    if host_player_unit_id == equipped_player_unit_id {
        return Err("Cannot bond a unit to itself.".into());
    }
    let user_id_i64 = user_id.get() as i64;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let host = sqlx::query!("SELECT pu.player_unit_id, pu.rarity::text as rarity_text, pu.is_in_party, u.kind::text as host_kind FROM player_units pu JOIN units u ON u.unit_id = pu.unit_id WHERE pu.player_unit_id = $1 AND pu.user_id = $2 FOR UPDATE", host_player_unit_id, user_id_i64)
	.fetch_one(&mut *tx)
	.await
	.map_err(|_| "Host unit not found.".to_string())?;
    let equipped = sqlx::query!("SELECT pu.player_unit_id, pu.rarity::text as rarity_text, pu.is_in_party, u.kind::text as eq_kind FROM player_units pu JOIN units u ON u.unit_id = pu.unit_id WHERE pu.player_unit_id = $1 AND pu.user_id = $2 FOR UPDATE", equipped_player_unit_id, user_id_i64)
	.fetch_one(&mut *tx)
	.await
	.map_err(|_| "Equippable unit not found.".to_string())?;

    let rarity_rank = |r: &str| -> i32 {
        match r {
            "Common" => 1,
            "Rare" => 2,
            "Epic" => 3,
            "Legendary" => 4,
            "Unique" => 5,
            "Mythical" => 6,
            "Fabled" => 7,
            _ => 0,
        }
    };
    if rarity_rank(equipped.rarity_text.as_deref().unwrap_or(""))
        > rarity_rank(host.rarity_text.as_deref().unwrap_or(""))
    {
        tx.rollback().await.ok();
        return Err("Equipped unit's rarity exceeds host unit's rarity.".into());
    }
    if equipped.eq_kind.as_deref() != Some("Pet") {
        tx.rollback().await.ok();
        return Err("Only pets can be equipped.".into());
    }

    let host_has = sqlx::query_scalar!(
        "SELECT 1 FROM equippable_unit_bonds WHERE host_player_unit_id = $1",
        host_player_unit_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    if host_has.is_some() {
        tx.rollback().await.ok();
        return Err("Host already has an equipped unit.".into());
    }
    let equipped_used = sqlx::query_scalar!(
        "SELECT 1 FROM equippable_unit_bonds WHERE equipped_player_unit_id = $1",
        equipped_player_unit_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    if equipped_used.is_some() {
        tx.rollback().await.ok();
        return Err("That unit is already bonded elsewhere.".into());
    }

    sqlx::query!("INSERT INTO equippable_unit_bonds (host_player_unit_id, equipped_player_unit_id, is_equipped) VALUES ($1,$2, TRUE)", host_player_unit_id, equipped_player_unit_id)
	.execute(&mut *tx)
	.await
	.map_err(|_| "Failed to create bond.".to_string())?;
    sqlx::query!(
        "UPDATE player_units SET is_in_party = FALSE WHERE player_unit_id = $1",
        equipped_player_unit_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| "Failed to update equipped unit state.".to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

// (Removed duplicate unequip_equippable & get_equipment_bonuses definitions after inlining earlier.)

// Typed fetch for active bonds (returns one row per active bond) primarily for diagnostics / future UI.
// Includes bonds even if not equipped yet plus timestamps for richer diagnostics.
#[instrument(level = "debug", skip(pool))]
pub async fn list_active_bonds_detailed(
    pool: &PgPool,
    user_id: UserId,
) -> Result<Vec<crate::database::models::EquippableUnitBond>, sqlx::Error> {
    sqlx::query_as!(crate::database::models::EquippableUnitBond,
		"SELECT bond_id, host_player_unit_id, equipped_player_unit_id, created_at, is_equipped FROM equippable_unit_bonds WHERE host_player_unit_id IN (SELECT player_unit_id FROM player_units WHERE user_id = $1)",
		user_id.get() as i64
	)
	.fetch_all(pool)
	.await
}

// -------------------------------------------------------------------------------------------------
// Bond contribution breakdown (per equipped bond raw stats -> computed bonuses)
// -------------------------------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct BondContribution {
    pub bond_id: i32,
    pub host_player_unit_id: i32,
    pub equipped_player_unit_id: i32,
    pub equipped_name: String,
    pub rarity: UnitRarity,
    pub bonus_attack: i32,
    pub bonus_defense: i32,
    pub bonus_health: i32,
}

#[instrument(level = "debug", skip(pool))]
pub async fn list_bond_contributions(
    pool: &PgPool,
    user_id: UserId,
) -> Result<Vec<BondContribution>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"SELECT b.bond_id, b.host_player_unit_id, b.equipped_player_unit_id,
			COALESCE(pu.nickname, u.name) as equipped_name,
			pu.current_attack, pu.current_defense, pu.current_health, pu.current_level,
			pu.rarity as "rarity: UnitRarity"
		FROM equippable_unit_bonds b
		JOIN player_units pu ON pu.player_unit_id = b.equipped_player_unit_id
		JOIN units u ON u.unit_id = pu.unit_id
		WHERE pu.user_id = $1 AND b.is_equipped = TRUE"#,
        user_id.get() as i64
    )
    .fetch_all(pool)
    .await?;
    let mut out = Vec::new();
    for r in rows {
        let rarity_mult = match r.rarity as i32 {
            0 => 0.05,
            1 => 0.08,
            2 => 0.12,
            3 => 0.18,
            4 => 0.24,
            5 => 0.30,
            6 => 0.40,
            _ => 0.05,
        };
        let level_factor = (r.current_level as f32).sqrt() / 10.0;
        let base_factor = rarity_mult + level_factor;
        let bonus_attack = ((r.current_attack as f32) * base_factor).ceil() as i32;
        let bonus_defense = ((r.current_defense as f32) * base_factor * 0.8).ceil() as i32;
        let bonus_health = ((r.current_health as f32) * base_factor * 1.2).ceil() as i32;
        out.push(BondContribution {
            bond_id: r.bond_id,
            host_player_unit_id: r.host_player_unit_id,
            equipped_player_unit_id: r.equipped_player_unit_id,
            equipped_name: r.equipped_name.unwrap_or_else(|| "(Unnamed)".into()),
            rarity: r.rarity,
            bonus_attack,
            bonus_defense,
            bonus_health,
        });
    }
    Ok(out)
}

// Internal helper: verify and consume a list of (Item, qty) atomically within an open transaction.
#[inline]
async fn verify_and_consume_items(
    tx: &mut Transaction<'_, Postgres>,
    user_id: UserId,
    requirements: &[(Item, i64)],
) -> Result<(), String> {
    for (item, qty) in requirements.iter() {
        let inv = get_inventory_item(tx, user_id, *item)
            .await
            .map_err(|_| "Inventory check failed".to_string())?;
        if inv.as_ref().map(|i| i.quantity < *qty).unwrap_or(true) {
            return Err(format!("You need {} {}.", qty, item.display_name()));
        }
    }
    for (item, qty) in requirements.iter() {
        add_to_inventory(tx, user_id, *item, -*qty)
            .await
            .map_err(|_| "Failed to consume materials".to_string())?;
    }
    Ok(())
}
