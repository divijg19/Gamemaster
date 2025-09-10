use chrono::Utc;
use serenity::model::id::UserId;
use sqlx::PgPool;
use sqlx::{Postgres, Transaction};

pub async fn get_or_create_fame(
    pool: &PgPool,
    user: UserId,
) -> sqlx::Result<(i32, i32, Option<chrono::DateTime<chrono::Utc>>)> {
    let uid = user.get() as i64;
    let rec = sqlx::query!(
        "SELECT fame, daily_rerolls, last_reroll FROM tavern_fame WHERE user_id=$1",
        uid
    )
    .fetch_optional(pool)
    .await?;
    if let Some(r) = rec {
        return Ok((r.fame, r.daily_rerolls, r.last_reroll));
    }
    sqlx::query!(
        "INSERT INTO tavern_fame (user_id) VALUES ($1) ON CONFLICT DO NOTHING",
        uid
    )
    .execute(pool)
    .await?;
    Ok((0, 0, None))
}

pub async fn add_fame(pool: &PgPool, user: UserId, amount: i32) -> sqlx::Result<()> {
    let uid = user.get() as i64;
    sqlx::query!("INSERT INTO tavern_fame (user_id,fame) VALUES ($1,$2) ON CONFLICT (user_id) DO UPDATE SET fame = tavern_fame.fame + $2", uid, amount)
        .execute(pool).await?;
    Ok(())
}

pub async fn can_reroll(pool: &PgPool, user: UserId, max_daily: i32) -> sqlx::Result<bool> {
    let uid = user.get() as i64;
    let row = sqlx::query!("SELECT daily_rerolls, COALESCE(last_reroll::date, CURRENT_DATE - INTERVAL '1 day') as d FROM tavern_fame WHERE user_id=$1", uid)
        .fetch_optional(pool).await?;
    if let Some(r) = row {
        let today = Utc::now().date_naive();
        // r.d is Option<NaiveDateTime>; safe unwrap due to COALESCE in query
        let last_day = r.d.unwrap().date();
        if last_day != today {
            return Ok(true);
        }
        return Ok(r.daily_rerolls < max_daily);
    }
    Ok(true)
}

pub async fn get_or_generate_rotation(
    pool: &PgPool,
    user: UserId,
    global_daily: &[i32],
) -> sqlx::Result<Vec<i32>> {
    let uid = user.get() as i64;
    let today = Utc::now().date_naive();
    if let Some(r) = sqlx::query!(
        "SELECT rotation, day FROM tavern_user_rotation WHERE user_id=$1",
        uid
    )
    .fetch_optional(pool)
    .await?
        && r.day == today
    {
        return Ok(r.rotation);
    }
    // store new rotation
    sqlx::query!("INSERT INTO tavern_user_rotation (user_id, rotation, day) VALUES ($1,$2,$3) ON CONFLICT (user_id) DO UPDATE SET rotation = EXCLUDED.rotation, day=EXCLUDED.day, generated_at = NOW()", uid, &global_daily, today)
        .execute(pool).await?;
    Ok(global_daily.to_vec())
}

// Note: legacy non-transactional helpers were removed in favor of `transactional_reroll`.

/// Performs a full reroll atomically:
/// - Validates reroll availability for today (locks fame row)
/// - Deducts balance (fails if insufficient)
/// - Overwrites today's rotation
/// - Increments daily_rerolls and updates last_reroll
pub async fn transactional_reroll(
    pool: &PgPool,
    user: UserId,
    new_rot: &[i32],
    cost: i64,
    max_daily: i32,
) -> sqlx::Result<()> {
    let uid = user.get() as i64;
    let today = Utc::now();
    let today_date = today.date_naive();
    let mut tx: Transaction<'_, Postgres> = pool.begin().await?;

    // Lock or initialize fame row
    let fame_row = sqlx::query_as::<_, (i32, Option<chrono::DateTime<chrono::Utc>>)>(
        "SELECT daily_rerolls, last_reroll FROM tavern_fame WHERE user_id=$1 FOR UPDATE",
    )
    .bind(uid)
    .fetch_optional(&mut *tx)
    .await?;
    let (mut daily_rerolls, last_reroll) = if let Some((dr, lr)) = fame_row {
        (dr, lr)
    } else {
        // Initialize row with defaults
        sqlx::query("INSERT INTO tavern_fame (user_id, fame, daily_rerolls, last_reroll) VALUES ($1,0,0,NULL) ON CONFLICT DO NOTHING")
            .bind(uid)
            .execute(&mut *tx)
            .await?;
        (0, None)
    };
    // Check reroll availability for today
    let can_today = match last_reroll {
        Some(ts) => ts.date_naive() != today_date || daily_rerolls < max_daily,
        None => true,
    };
    if !can_today {
        // Mimic RowNotFound to bubble a simple error upstream
        return Err(sqlx::Error::RowNotFound);
    }

    // Deduct balance (will fail if insufficient)
    crate::database::economy::add_balance(&mut tx, user, -cost).await?;

    // Overwrite rotation for today
    sqlx::query(
        "INSERT INTO tavern_user_rotation (user_id, rotation, day) VALUES ($1,$2,$3) ON CONFLICT (user_id) DO UPDATE SET rotation=$2, day=$3, generated_at=NOW()",
    )
    .bind(uid)
    .bind(new_rot)
    .bind(today_date)
    .execute(&mut *tx)
    .await?;

    // Increment reroll counters
    if let Some(ts) = last_reroll {
        if ts.date_naive() == today_date {
            daily_rerolls += 1;
        } else {
            daily_rerolls = 1;
        }
    } else {
        daily_rerolls = 1;
    }
    sqlx::query("UPDATE tavern_fame SET daily_rerolls=$2, last_reroll=$3 WHERE user_id=$1")
        .bind(uid)
        .bind(daily_rerolls)
        .bind(today)
        .execute(&mut *tx)
        .await?;

    tx.commit().await
}
