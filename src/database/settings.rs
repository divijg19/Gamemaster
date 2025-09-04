use sqlx::PgPool;

pub async fn get_config_value(pool: &PgPool, key: &str) -> sqlx::Result<Option<String>> {
    let row = sqlx::query!("SELECT value FROM bot_config WHERE key = $1", key)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.value))
}

pub async fn set_config_value(pool: &PgPool, key: &str, value: &str) -> sqlx::Result<()> {
    sqlx::query!("INSERT INTO bot_config (key, value) VALUES ($1, $2) ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value", key, value)
        .execute(pool)
        .await?;
    Ok(())
}
