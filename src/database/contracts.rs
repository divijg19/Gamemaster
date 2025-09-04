use serenity::model::id::UserId;
use sqlx::PgPool;
use tracing::instrument;

use crate::database::models::{HumanContractOffer, Unit, UnitKind, UnitRarity};

// NOTE: This legacy contracts module is superseded by encounter-based drafting in `human.rs`.
// It remains temporarily for backward compatibility / potential migration of existing offer rows.

fn rarity_cost_multiplier(r: UnitRarity) -> i64 {
    match r {
        UnitRarity::Common => 1,
        UnitRarity::Rare => 3,
        UnitRarity::Epic => 8,
        UnitRarity::Legendary => 20,
        UnitRarity::Unique => 35,
        UnitRarity::Mythical => 55,
        UnitRarity::Fabled => 85,
    }
}

#[instrument(level="debug", skip(pool))]
pub async fn unlock_human_contract_if_needed(pool: &PgPool, user_id: UserId, unit: &Unit) -> Result<(), sqlx::Error> {
    if !matches!(unit.kind, UnitKind::Human) { return Ok(()); }
    let user_id_i64 = user_id.get() as i64;
    // Idempotent: insert only if absent
    let base_cost: i64 = 250; // base
    let cost = base_cost * rarity_cost_multiplier(unit.rarity);
    sqlx::query!("INSERT INTO human_contract_offers (user_id, unit_id, cost, rarity_snapshot) VALUES ($1,$2,$3,$4) ON CONFLICT (user_id, unit_id) DO NOTHING", user_id_i64, unit.unit_id, cost, unit.rarity as _).execute(pool).await?;
    Ok(())
}

#[instrument(level="debug", skip(pool))]
pub async fn list_open_contracts(pool: &PgPool, user_id: UserId) -> Result<Vec<HumanContractOffer>, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    sqlx::query_as!(HumanContractOffer, "SELECT user_id, unit_id, cost, offered_at, expires_at, accepted_at, rarity_snapshot as \"rarity_snapshot: UnitRarity\" FROM human_contract_offers WHERE user_id = $1 AND accepted_at IS NULL", user_id_i64).fetch_all(pool).await
}

#[instrument(level="debug", skip(pool))]
pub async fn accept_contract(pool: &PgPool, user_id: UserId, unit_id: i32) -> Result<String, String> {
    use crate::database::economy::add_balance;
    use crate::database::models::Profile;
    let user_id_i64 = user_id.get() as i64;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    sqlx::query!("SELECT pg_advisory_xact_lock($1)", user_id_i64).execute(&mut *tx).await.map_err(|e| e.to_string())?;
    let offer = sqlx::query!("SELECT cost, accepted_at FROM human_contract_offers WHERE user_id = $1 AND unit_id = $2 FOR UPDATE", user_id_i64, unit_id).fetch_optional(&mut *tx).await.map_err(|e| e.to_string())?;
    let Some(offer_row) = offer else { tx.rollback().await.ok(); return Err("No active contract offer.".into()); };
    if offer_row.accepted_at.is_some() { tx.rollback().await.ok(); return Err("Contract already accepted.".into()); }
    // Fetch unit meta for snapshot
    let unit_master = sqlx::query_as!(Unit, "SELECT unit_id, name, description, base_attack, base_defense, base_health, is_recruitable, kind as \"kind: UnitKind\", rarity as \"rarity: UnitRarity\" FROM units WHERE unit_id = $1", unit_id).fetch_one(&mut *tx).await.map_err(|_| "Unit missing.".to_string())?;
    let profile = sqlx::query_as!(Profile, "SELECT balance, last_work, work_streak, fishing_xp, fishing_level, mining_xp, mining_level, coding_xp, coding_level FROM profiles WHERE user_id = $1 FOR UPDATE", user_id_i64).fetch_one(&mut *tx).await.map_err(|_| "Profile missing.".to_string())?;
    if profile.balance < offer_row.cost { tx.rollback().await.ok(); return Err("Not enough coins.".into()); }
    add_balance(&mut tx, user_id, -offer_row.cost).await.map_err(|_| "Payment failed.".to_string())?;
    // Insert player unit (Humans always eligible for party if space; reuse logic light)
    let party_size: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM player_units WHERE user_id = $1 AND is_in_party = TRUE", user_id_i64).fetch_one(&mut *tx).await.unwrap_or(Some(0)).unwrap_or(0);
    let mut is_in_party = false; if party_size < crate::constants::MAX_PARTY_SIZE { is_in_party = true; }
    sqlx::query!("INSERT INTO player_units (user_id, unit_id, nickname, current_attack, current_defense, current_health, rarity, is_in_party) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)", user_id_i64, unit_id, &unit_master.name, unit_master.base_attack, unit_master.base_defense, unit_master.base_health, unit_master.rarity as _, is_in_party).execute(&mut *tx).await.map_err(|_| "Failed to add unit.".to_string())?;
    sqlx::query!("UPDATE human_contract_offers SET accepted_at = NOW() WHERE user_id = $1 AND unit_id = $2", user_id_i64, unit_id).execute(&mut *tx).await.map_err(|_| "Failed to finalize contract.".to_string())?;
    tx.commit().await.map_err(|_| "Commit failed.".to_string())?;
    Ok(unit_master.name)
}
