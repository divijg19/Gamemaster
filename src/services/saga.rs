//! Saga service layer: centralizes profile retrieval with short-lived caching.
use super::cache;
use crate::{AppState, database};
use serenity::model::id::UserId;
use std::time::Duration;
use tracing::{debug, instrument};

pub const SAGA_PROFILE_CACHE_TTL_SECS: u64 = 3;

/// Fetch the player's up-to-date saga profile, optionally using short TTL cache.
/// force_refresh bypasses cache and always performs DB update (AP/TP & training completion logic).
#[instrument(level="debug", skip(app_state), fields(user_id = user_id.get(), force = force_refresh))]
pub async fn get_saga_profile(
    app_state: &AppState,
    user_id: UserId,
    force_refresh: bool,
) -> Option<crate::database::models::SagaProfile> {
    let ttl = Duration::from_secs(SAGA_PROFILE_CACHE_TTL_SECS);
    if !force_refresh {
        if let Some(profile) =
            cache::get_with_ttl(&app_state.saga_profile_cache, &user_id.get(), ttl).await
        {
            debug!(target = "cache.saga_profile", hit = true, force = false);
            return Some(profile);
        } else {
            debug!(
                target = "cache.saga_profile",
                hit = false,
                reason = "miss_or_expired"
            );
        }
    } else {
        debug!(target = "cache.saga_profile", bypass = true);
    }
    match database::saga::update_and_get_saga_profile(&app_state.db, user_id).await {
        Ok(p) => {
            cache::insert(&app_state.saga_profile_cache, user_id.get(), p.clone()).await;
            Some(p)
        }
        Err(e) => {
            debug!(target="cache.saga_profile", error=%e, "db_error");
            None
        }
    }
}

/// Batched helper: returns (SagaProfile, Units) minimizing duplicate queries.
/// Uses cached profile when valid; always queries units (since unit changes happen frequently via training, recruiting, bonding).
#[instrument(level="debug", skip(app_state), fields(user_id = user_id.get()))]
pub async fn get_profile_and_units(
    app_state: &AppState,
    user_id: UserId,
) -> Option<(
    crate::database::models::SagaProfile,
    Vec<crate::database::models::PlayerUnit>,
)> {
    let ttl = Duration::from_secs(SAGA_PROFILE_CACHE_TTL_SECS);
    if let Some(profile) =
        cache::get_with_ttl(&app_state.saga_profile_cache, &user_id.get(), ttl).await
    {
        debug!(target = "cache.saga_profile", hit = true, combined = true);
        if let Ok(units) = database::units::get_player_units(&app_state.db, user_id).await {
            return Some((profile, units));
        } else {
            return None;
        }
    }
    match database::saga::update_get_profile_and_units(&app_state.db, user_id).await {
        Ok((profile, units)) => {
            cache::insert(
                &app_state.saga_profile_cache,
                user_id.get(),
                profile.clone(),
            )
            .await;
            Some((profile, units))
        }
        Err(e) => {
            debug!(target="cache.saga_profile", error=%e, combined=true, "db_error");
            None
        }
    }
}
