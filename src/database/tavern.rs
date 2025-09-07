use chrono::Utc;
use serenity::model::id::UserId;
use sqlx::PgPool;

pub async fn get_or_create_favor(
    pool: &PgPool,
    user: UserId,
) -> sqlx::Result<(i32, i32, Option<chrono::DateTime<chrono::Utc>>)> {
    let uid = user.get() as i64;
    let rec = sqlx::query!(
        "SELECT favor, daily_rerolls, last_reroll FROM tavern_favor WHERE user_id=$1",
        uid
    )
    .fetch_optional(pool)
    .await?;
    if let Some(r) = rec {
        return Ok((r.favor, r.daily_rerolls, r.last_reroll));
    }
    sqlx::query!(
        "INSERT INTO tavern_favor (user_id) VALUES ($1) ON CONFLICT DO NOTHING",
        uid
    )
    .execute(pool)
    .await?;
    Ok((0, 0, None))
}

pub async fn add_favor(pool: &PgPool, user: UserId, amount: i32) -> sqlx::Result<()> {
    let uid = user.get() as i64;
    sqlx::query!("INSERT INTO tavern_favor (user_id,favor) VALUES ($1,$2) ON CONFLICT (user_id) DO UPDATE SET favor = tavern_favor.favor + $2", uid, amount)
        .execute(pool).await?;
    Ok(())
}

pub async fn can_reroll(pool: &PgPool, user: UserId, max_daily: i32) -> sqlx::Result<bool> {
    let uid = user.get() as i64;
    let row = sqlx::query!("SELECT daily_rerolls, COALESCE(last_reroll::date, CURRENT_DATE - INTERVAL '1 day') as d FROM tavern_favor WHERE user_id=$1", uid)
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

pub async fn consume_reroll(pool: &PgPool, user: UserId) -> sqlx::Result<()> {
    let uid = user.get() as i64;
    let today = Utc::now();
    sqlx::query!("INSERT INTO tavern_favor (user_id, daily_rerolls, last_reroll) VALUES ($1,1,$2) ON CONFLICT (user_id) DO UPDATE SET daily_rerolls = CASE WHEN tavern_favor.last_reroll::date = CURRENT_DATE THEN tavern_favor.daily_rerolls + 1 ELSE 1 END, last_reroll = $2", uid, today)
        .execute(pool).await?;
    Ok(())
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
        && r.day == today {
            return Ok(r.rotation);
        }
    // store new rotation
    sqlx::query!("INSERT INTO tavern_user_rotation (user_id, rotation, day) VALUES ($1,$2,$3) ON CONFLICT (user_id) DO UPDATE SET rotation = EXCLUDED.rotation, day=EXCLUDED.day, generated_at = NOW()", uid, &global_daily, today)
        .execute(pool).await?;
    Ok(global_daily.to_vec())
}

pub async fn overwrite_rotation(pool: &PgPool, user: UserId, new_rot: &[i32]) -> sqlx::Result<()> {
    let uid = user.get() as i64;
    let today = Utc::now().date_naive();
    sqlx::query!("INSERT INTO tavern_user_rotation (user_id, rotation, day) VALUES ($1,$2,$3) ON CONFLICT (user_id) DO UPDATE SET rotation=$2, day=$3, generated_at=NOW()", uid, &new_rot, today)
        .execute(pool).await?;
    Ok(())
}
