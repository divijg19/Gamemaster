use serenity::model::id::UserId;
use sqlx::PgPool;
use tracing::instrument;

use crate::AppState;
use crate::database::models::{DraftedHumanContract, HumanContractOffer};
use crate::database::models::{HumanEncounter, Unit, UnitKind, UnitRarity};
use std::time::Duration;

#[inline]
pub fn defeats_required_for(r: UnitRarity) -> i32 {
    match r {
        UnitRarity::Common => 2,
        UnitRarity::Rare => 3,
        UnitRarity::Epic => 5,
        UnitRarity::Legendary => 7,
        UnitRarity::Unique => 9,
        UnitRarity::Mythical => 12,
        UnitRarity::Fabled => 15,
    }
}

#[instrument(level = "debug", skip(pool))]
pub async fn record_human_defeat(
    pool: &PgPool,
    user_id: UserId,
    unit: &Unit,
) -> Result<(), sqlx::Error> {
    if !matches!(unit.kind, UnitKind::Human) {
        return Ok(());
    }
    let uid = user_id.get() as i64;
    sqlx::query!(
        r#"INSERT INTO human_encounters (user_id, unit_id, defeats) VALUES ($1,$2,1)
        ON CONFLICT (user_id, unit_id)
        DO UPDATE SET defeats = human_encounters.defeats + 1, last_defeated_at = NOW()"#,
        uid,
        unit.unit_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[instrument(level = "debug", skip(pool))]
pub async fn get_encounter(
    pool: &PgPool,
    user_id: UserId,
    unit_id: i32,
) -> Result<Option<HumanEncounter>, sqlx::Error> {
    let uid = user_id.get() as i64;
    sqlx::query_as!(HumanEncounter, "SELECT user_id, unit_id, defeats, last_defeated_at FROM human_encounters WHERE user_id=$1 AND unit_id=$2", uid, unit_id).fetch_optional(pool).await
}

#[instrument(level = "debug", skip(pool))]
pub async fn list_human_progress(
    pool: &PgPool,
    user_id: UserId,
) -> Result<Vec<(Unit, i32, i32)>, sqlx::Error> {
    // Returns (Unit, defeats, required)
    let uid = user_id.get() as i64;
    let rows = sqlx::query!(r#"SELECT u.unit_id, u.name, u.description, u.base_attack, u.base_defense, u.base_health, u.is_recruitable, u.kind as "kind: UnitKind", u.rarity as "rarity: UnitRarity", COALESCE(he.defeats,0) as defeats
        FROM units u LEFT JOIN human_encounters he ON he.user_id = $1 AND he.unit_id = u.unit_id
        WHERE u.kind = 'Human'"#, uid).fetch_all(pool).await?;
    let mut out = Vec::new();
    for r in rows {
        out.push((
            Unit {
                unit_id: r.unit_id,
                name: r.name,
                description: r.description,
                base_attack: r.base_attack,
                base_defense: r.base_defense,
                base_health: r.base_health,
                is_recruitable: r.is_recruitable,
                kind: r.kind,
                rarity: r.rarity,
            },
            r.defeats.unwrap_or(0),
            defeats_required_for(r.rarity),
        ));
    }
    Ok(out)
}

#[instrument(level = "debug", skip(pool))]
pub async fn draft_contract(pool: &PgPool, user_id: UserId, unit_id: i32) -> Result<(), String> {
    let uid = user_id.get() as i64;
    let meta = sqlx::query!("SELECT u.unit_id, u.name, u.description, u.base_attack, u.base_defense, u.base_health, u.is_recruitable, u.kind as \"kind: UnitKind\", u.rarity as \"rarity: UnitRarity\" FROM units u WHERE u.unit_id = $1", unit_id)
        .fetch_one(pool).await.map_err(|_| "Unit not found".to_string())?;
    if !matches!(meta.kind, UnitKind::Human) {
        return Err("That unit is not a human.".into());
    }
    let encounter = sqlx::query!(
        "SELECT defeats FROM human_encounters WHERE user_id=$1 AND unit_id=$2",
        uid,
        unit_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| "Encounter lookup failed".to_string())?;
    let defeats = encounter.map(|r| r.defeats).unwrap_or(0);
    let required = defeats_required_for(meta.rarity);
    if defeats < required {
        return Err(format!("Need {} defeats, you have {}.", required, defeats));
    }
    let drafted_exists = sqlx::query_scalar!(
        "SELECT 1 FROM drafted_human_contracts WHERE user_id=$1 AND unit_id=$2 AND consumed=FALSE",
        uid,
        unit_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| "Draft check failed".to_string())?
    .is_some();
    if drafted_exists {
        return Err("Contract already drafted.".into());
    }
    // Parchment gating (feature-flagged). When enabled:
    //  - Rare humans need Forest parchment (14)
    //  - Epic+ need Frontier parchment (15)
    //  - Common needs none
    use crate::{commands::economy::core::item::Item, constants::ENABLE_PARCHMENT_GATING};
    let parchment_needed = if ENABLE_PARCHMENT_GATING {
        match meta.rarity {
            UnitRarity::Rare => Some(Item::ForestContractParchment),
            UnitRarity::Epic
            | UnitRarity::Legendary
            | UnitRarity::Unique
            | UnitRarity::Mythical
            | UnitRarity::Fabled => Some(Item::FrontierContractParchment),
            _ => None,
        }
    } else {
        None
    };
    if let Some(item) = parchment_needed {
        // Attempt atomic consumption
        let mut tx = pool
            .begin()
            .await
            .map_err(|_| "Tx start fail".to_string())?;
        let inv_row = sqlx::query!(
            "SELECT quantity FROM inventories WHERE user_id=$1 AND item_id=$2 FOR UPDATE",
            uid,
            item as i32
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| "Inventory check failed".to_string())?;
        if inv_row.as_ref().map(|r| r.quantity < 1).unwrap_or(true) {
            tx.rollback().await.ok();
            return Err(format!(
                "You need a {} to draft this contract.",
                item.display_name()
            ));
        }
        sqlx::query!(
            "UPDATE inventories SET quantity = quantity - 1 WHERE user_id=$1 AND item_id=$2",
            uid,
            item as i32
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| "Failed to consume parchment".to_string())?;
        sqlx::query!(
            "INSERT INTO drafted_human_contracts (user_id, unit_id) VALUES ($1,$2)",
            uid,
            unit_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| "Failed to draft".to_string())?;
        tx.commit().await.map_err(|_| "Commit fail".to_string())?;
    } else {
        sqlx::query!(
            "INSERT INTO drafted_human_contracts (user_id, unit_id) VALUES ($1,$2)",
            uid,
            unit_id
        )
        .execute(pool)
        .await
        .map_err(|_| "Failed to draft".to_string())?;
    }
    Ok(())
}

#[instrument(level = "debug", skip(pool))]
pub async fn accept_drafted_contract(
    pool: &PgPool,
    user_id: UserId,
    unit_id: i32,
) -> Result<String, String> {
    let uid = user_id.get() as i64;
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| "Tx start fail".to_string())?;
    let drafted = sqlx::query_scalar!(
        "SELECT 1 FROM drafted_human_contracts WHERE user_id=$1 AND unit_id=$2 AND consumed=FALSE",
        uid,
        unit_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| "Lookup failed".to_string())?
    .is_some();
    if !drafted {
        tx.rollback().await.ok();
        return Err("No drafted contract.".into());
    }
    // Reuse hire logic minimal: fetch unit
    let unit_master = sqlx::query!("SELECT unit_id, name, base_attack, base_defense, base_health, rarity as \"rarity: UnitRarity\", kind as \"kind: UnitKind\", is_recruitable, description FROM units WHERE unit_id=$1", unit_id)
        .fetch_one(&mut *tx).await.map_err(|_| "Unit not found".to_string())?;
    if !matches!(unit_master.kind, UnitKind::Human) {
        tx.rollback().await.ok();
        return Err("Not a human.".into());
    }
    let party_size: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM player_units WHERE user_id=$1 AND is_in_party=TRUE",
        uid
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap_or(Some(0))
    .unwrap_or(0);
    let mut is_in_party = false;
    if party_size < crate::constants::MAX_PARTY_SIZE {
        is_in_party = true;
    }
    sqlx::query!("INSERT INTO player_units (user_id, unit_id, nickname, current_attack, current_defense, current_health, rarity, is_in_party) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)", uid, unit_id, &unit_master.name, unit_master.base_attack, unit_master.base_defense, unit_master.base_health, unit_master.rarity as _, is_in_party)
        .execute(&mut *tx).await.map_err(|_| "Insert failed".to_string())?;
    sqlx::query!(
        "UPDATE drafted_human_contracts SET consumed=TRUE WHERE user_id=$1 AND unit_id=$2",
        uid,
        unit_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| "Consume failed".to_string())?;
    tx.commit().await.map_err(|_| "Commit failed".to_string())?;
    Ok(unit_master.name)
}

pub type ContractStatusRow = (
    Unit,
    i32,
    i32,
    bool,
    bool,
    Option<chrono::DateTime<chrono::Utc>>,
); // (Unit, defeats, required, drafted, recruited, last_defeat)
#[instrument(level = "debug", skip(pool))]

pub async fn list_contract_status(
    pool: &PgPool,
    user_id: UserId,
) -> Result<Vec<ContractStatusRow>, sqlx::Error> {
    // (Unit, defeats, required, drafted, recruited)
    let uid = user_id.get() as i64;
    let rows = sqlx::query!(r#"SELECT u.unit_id, u.name, u.description, u.base_attack, u.base_defense, u.base_health, u.is_recruitable, u.kind as "kind: UnitKind", u.rarity as "rarity: UnitRarity",
        COALESCE(he.defeats,0) as defeats, he.last_defeated_at,
        (SELECT 1 FROM drafted_human_contracts d2 WHERE d2.user_id=$1 AND d2.unit_id=u.unit_id AND d2.consumed=FALSE) as drafted_active,
        (SELECT 1 FROM player_units pu WHERE pu.user_id=$1 AND pu.unit_id=u.unit_id LIMIT 1) as recruited
        FROM units u LEFT JOIN human_encounters he ON he.user_id=$1 AND he.unit_id=u.unit_id WHERE u.kind='Human'"#, uid)
        .fetch_all(pool).await?;
    let mut out = Vec::new();
    for r in rows {
        out.push((
            Unit {
                unit_id: r.unit_id,
                name: r.name,
                description: r.description,
                base_attack: r.base_attack,
                base_defense: r.base_defense,
                base_health: r.base_health,
                is_recruitable: r.is_recruitable,
                kind: r.kind,
                rarity: r.rarity,
            },
            r.defeats.unwrap_or(0),
            defeats_required_for(r.rarity),
            r.drafted_active.is_some(),
            r.recruited.is_some(),
            r.last_defeated_at,
        ));
    }
    Ok(out)
}

/// Fetch currently drafted (but not yet consumed) human contracts for a user. Activates DraftedHumanContract struct.
#[instrument(level = "debug", skip(pool))]
pub async fn list_drafted_contracts(
    pool: &PgPool,
    user_id: UserId,
) -> Result<Vec<DraftedHumanContract>, sqlx::Error> {
    let uid = user_id.get() as i64;
    sqlx::query_as!(DraftedHumanContract, "SELECT user_id, unit_id, drafted_at, consumed FROM drafted_human_contracts WHERE user_id = $1 AND consumed = FALSE ORDER BY drafted_at DESC", uid).fetch_all(pool).await
}

/// Fetch legacy open contract offers (pre-drafting system). Activates HumanContractOffer struct for backward compatibility.
#[instrument(level = "debug", skip(pool))]
pub async fn list_legacy_open_offers(
    pool: &PgPool,
    user_id: UserId,
) -> Result<Vec<HumanContractOffer>, sqlx::Error> {
    let uid = user_id.get() as i64;
    sqlx::query_as!(HumanContractOffer, "SELECT user_id, unit_id, cost, offered_at, expires_at, accepted_at, rarity_snapshot as \"rarity_snapshot: UnitRarity\" FROM human_contract_offers WHERE user_id=$1 AND accepted_at IS NULL ORDER BY offered_at DESC", uid).fetch_all(pool).await
}

/// Cached wrapper around `list_contract_status` with a short TTL to reduce repeated queries from rapid component refreshes.
pub async fn list_contract_status_cached(
    app_state: &AppState,
    user_id: UserId,
) -> Result<Vec<ContractStatusRow>, sqlx::Error> {
    const TTL: Duration = Duration::from_secs(20);
    if let Some(v) =
        crate::services::cache::get_with_ttl(&app_state.contract_cache, &user_id.get(), TTL).await
    {
        return Ok(v);
    }
    let fresh = list_contract_status(&app_state.db, user_id).await?;
    crate::services::cache::insert(&app_state.contract_cache, user_id.get(), fresh.clone()).await;
    Ok(fresh)
}

pub async fn get_encounter_by_id(
    db: &sqlx::PgPool,
    unit_id: i32,
) -> Result<HumanEncounter, sqlx::Error> {
    // We don't have a standalone encounter_id column; composite key (user_id, unit_id). For debug we just pick latest by defeat time.
    let row = sqlx::query_as!(
        HumanEncounter,
        r#"SELECT user_id, unit_id, defeats, last_defeated_at
           FROM human_encounters WHERE unit_id = $1 ORDER BY last_defeated_at DESC NULLS LAST LIMIT 1"#,
        unit_id
    )
    .fetch_one(db)
    .await?;
    Ok(row)
}
