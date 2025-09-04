//! LEGACY FILE (will be removed): previously contained all database functions related to pets.
//! Phase B refactor: Converted to units terminology & schema. Keep filename temporarily for incremental refactor; callers should migrate to `database::units`.
#![allow(dead_code)]
#![deprecated(note = "Use database::units instead; this legacy pets module will be deleted.")]

use super::economy::{add_balance, add_to_inventory, get_inventory_item};
use super::models::{PlayerUnit, Profile, Unit, UnitKind, UnitRarity};
use crate::commands::economy::core::item::Item;
use crate::saga;
use crate::saga::leveling::LevelUpResult;
use serenity::model::id::UserId;
use sqlx::types::chrono::Utc;
use sqlx::{PgPool, Postgres, Transaction};
use std::str::FromStr;

// -------------------- Equippable / Bonding System ----------------------------------------------
// Higher rarity host units can bond certain special units (e.g., Alpha Wolf) turning them into
// equippable stat augments. Bonding removes the equipped unit from active party consideration.

/// Attempt to bond (equip) one owned unit onto another host unit, applying rarity gating rules.
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
    // Fetch host + equipped with locking and rarity
    let host = sqlx::query!(
        "SELECT player_unit_id, rarity::text as rarity_text, is_in_party FROM player_units WHERE player_unit_id = $1 AND user_id = $2 FOR UPDATE",
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

    // Rarity ordering enforcement (equipped rarity must be <= host rarity)
    // Postgres enum ordering preserved; cast to text and map if needed; here simple numeric map.
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

    // Prevent host already having a bond (one equippable per host policy) and equipped reused.
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

    // Insert bond & mark equipped unit as not in party.
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

/// Mark the current bond for host as unequipped (set is_equipped = FALSE) preserving history.
pub async fn unequip_equippable(
    pool: &PgPool,
    user_id: UserId,
    host_player_unit_id: i32,
) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    // Validate host belongs to user
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
    let updated = sqlx::query!("UPDATE equippable_unit_bonds SET is_equipped = FALSE WHERE host_player_unit_id = $1 AND is_equipped = TRUE", host_player_unit_id).execute(&mut *tx).await?.rows_affected();
    tx.commit().await.ok();
    Ok(updated > 0)
}

// Unbond intentionally unsupported (design choice). Provide future migration path if needed.

/// List currently bonded equippable units for a given host.
pub async fn list_equippables_for_host(
    pool: &PgPool,
    host_player_unit_id: i32,
) -> Result<Vec<(i32, String)>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"SELECT b.equipped_player_unit_id, pu.nickname as nickname
            FROM equippable_unit_bonds b
            JOIN player_units pu ON pu.player_unit_id = b.equipped_player_unit_id
            WHERE b.host_player_unit_id = $1"#,
        host_player_unit_id
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            (
                r.equipped_player_unit_id,
                r.nickname.unwrap_or_else(|| "(Unnamed)".into()),
            )
        })
        .collect())
}

/// Fetches all units owned by a player, joining with the master unit table to get species names.
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

/// Fetches only the units that are currently in the player's active party.
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

/// Fetches the master data for a list of units by their IDs.
pub async fn get_units_by_ids(pool: &PgPool, unit_ids: &[i32]) -> Result<Vec<Unit>, sqlx::Error> {
    sqlx::query_as!(Unit, "SELECT unit_id, name, description, base_attack, base_defense, base_health, is_recruitable, kind as \"kind: UnitKind\", rarity as \"rarity: UnitRarity\" FROM units WHERE unit_id = ANY($1)", unit_ids)
        .fetch_all(pool)
        .await
}

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

pub async fn hire_unit(
    pool: &PgPool,
    user_id: UserId,
    unit_id: i32,
    cost: i64,
) -> Result<String, String> {
    let user_id_i64 = user_id.get() as i64;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
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
    if army_size >= 10 {
        tx.rollback().await.ok();
        return Err("Your army is full (10/10).".to_string());
    }
    let pet_to_hire = sqlx::query_as!(Unit, "SELECT unit_id, name, description, base_attack, base_defense, base_health, is_recruitable, kind as \"kind: UnitKind\", rarity as \"rarity: UnitRarity\" FROM units WHERE unit_id = $1", unit_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| "This mercenary is no longer available.".to_string())?;
    // (✓) FIXED: Pass `&mut tx` to your helper function.
    add_balance(&mut tx, user_id, -cost)
        .await
        .map_err(|_| "Failed to process payment.".to_string())?;
    sqlx::query!("INSERT INTO player_units (user_id, unit_id, nickname, current_attack, current_defense, current_health, rarity) VALUES ($1, $2, $3, $4, $5, $6, $7)", user_id_i64, unit_id, &pet_to_hire.name, pet_to_hire.base_attack, pet_to_hire.base_defense, pet_to_hire.base_health, pet_to_hire.rarity as _).execute(&mut *tx).await.map_err(|_| "Failed to add unit to your army.".to_string())?;
    tx.commit()
        .await
        .map_err(|_| "Failed to finalize the transaction.".to_string())?;
    Ok(pet_to_hire.name)
}

pub async fn attempt_recruit_unit(
    pool: &PgPool,
    user_id: UserId,
    unit_id_to_recruit: i32,
) -> Result<String, String> {
    let user_id_i64 = user_id.get() as i64;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let pet_master = sqlx::query_as!(Unit, "SELECT unit_id, name, description, base_attack, base_defense, base_health, is_recruitable, kind as \"kind: UnitKind\", rarity as \"rarity: UnitRarity\" FROM units WHERE unit_id = $1", unit_id_to_recruit)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| "Creature data not found.".to_string())?;
    if !pet_master.is_recruitable {
        tx.rollback().await.ok();
        return Err("This unit cannot be recruited.".to_string());
    }
    let army_size: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM player_units WHERE user_id = $1",
        user_id_i64
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap_or(Some(0))
    .unwrap_or(0);
    if army_size >= 10 {
        tx.rollback().await.ok();
        return Err(
            "Your army is full! You must dismiss a unit before recruiting a new one.".to_string(),
        );
    }
    let research_data_item_name = format!("{} Research Data", pet_master.name);
    let research_data_item = Item::from_str(&research_data_item_name)
        .map_err(|_| "Could not identify the required research data.".to_string())?;
    let required_items = [(Item::TamingLure, 1), (research_data_item, 10)];
    for (item, required_quantity) in required_items {
        // (✓) FIXED: Pass `&mut tx` to your helper function.
        let has_item = get_inventory_item(&mut tx, user_id, item)
            .await
            .map_err(|_| "Could not check your inventory.".to_string())?;
        if has_item.is_none() || has_item.unwrap().quantity < required_quantity {
            tx.rollback().await.ok();
            return Err(format!(
                "You don't have enough materials! You need {} {}.",
                required_quantity,
                item.display_name()
            ));
        }
    }
    for (item, required_quantity) in required_items {
        // (✓) FIXED: Pass `&mut tx` to your helper function.
        add_to_inventory(&mut tx, user_id, item, -required_quantity)
            .await
            .map_err(|_| "Failed to consume recruitment items.".to_string())?;
    }
    sqlx::query!("INSERT INTO player_units (user_id, unit_id, nickname, current_attack, current_defense, current_health, rarity) VALUES ($1, $2, $3, $4, $5, $6, $7)", user_id_i64, unit_id_to_recruit, &pet_master.name, pet_master.base_attack, pet_master.base_defense, pet_master.base_health, pet_master.rarity as _).execute(&mut *tx).await.map_err(|_| "Failed to add the recruited unit to your army.".to_string())?;
    tx.commit()
        .await
        .map_err(|_| "Failed to finalize the transaction.".to_string())?;
    Ok(pet_master.name)
}

async fn spend_training_points(
    tx: &mut Transaction<'_, Postgres>,
    user_id: UserId,
    amount: i32,
) -> Result<bool, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    let rows_affected = sqlx::query!("UPDATE player_saga_profile SET current_tp = current_tp - $1 WHERE user_id = $2 AND current_tp >= $1", amount, user_id_i64).execute(&mut **tx).await?.rows_affected();
    Ok(rows_affected > 0)
}

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

pub async fn set_unit_party_status(
    pool: &PgPool,
    user_id: UserId,
    player_unit_id: i32,
    is_in_party: bool,
) -> Result<bool, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    let mut tx = pool.begin().await?;
    if is_in_party {
        let party_size: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM player_units WHERE user_id = $1 AND is_in_party = TRUE",
            user_id_i64
        )
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or(0);
        if party_size >= 5 {
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

/// One-time utility to sync existing player_units rarity from master units (in case of pre-existing rows).

/// Compute augmented stats by summing a flat portion of equipped units' base stats (prototype).

/// Returns a mapping of host_player_unit_id -> (bonus_attack, bonus_defense, bonus_health)
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
