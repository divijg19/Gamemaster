use crate::commands::games::engine::GameManager;
use crate::handler::Handler;
use crate::model::{AppState, ShardManagerContainer};
use serenity::model::gateway::GatewayIntents;
use serenity::model::id::GuildId;
use serenity::prelude::*;
use shuttle_runtime::SecretStore;
use shuttle_serenity::ShuttleSerenity;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;

// These are our application modules
mod commands;
mod database;
mod handler;
mod interactions;
mod model;
mod saga;

#[shuttle_runtime::main]
async fn serenity(
    #[shuttle_shared_db::Postgres] pool: PgPool,
    #[shuttle_runtime::Secrets] secrets: SecretStore,
) -> ShuttleSerenity {
    // 1. Run database migrations on startup.
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations.");
    println!("[SETUP] Database migrations run successfully.");

    // 2. Load secrets from the Shuttle secret store.
    let token = secrets
        .get("DISCORD_TOKEN")
        .expect("'DISCORD_TOKEN' was not found.");
    let server_id_str = secrets
        .get("SERVER_ID")
        .expect("'SERVER_ID' was not found.");
    println!("[SETUP] Secrets loaded successfully.");

    let server_id = server_id_str
        .parse::<u64>()
        .expect("SERVER_ID must be a valid number.");
    let allowed_guild_id = GuildId::new(server_id);

    // 3. Initialize the shared application state.
    let app_state = Arc::new(AppState {
        // GameManager is wrapped for interior mutability across async tasks.
        game_manager: Arc::new(RwLock::new(GameManager::new())),
        // PgPool is already thread-safe (it's an Arc internally).
        db: pool,
        // The prefix needs to be mutable at runtime by admins.
        prefix: Arc::new(RwLock::new("$".to_string())),
    });
    println!("[SETUP] Shared application state initialized.");

    // 4. Set gateway intents required for the bot's functionality.
    let intents =
        GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    // 5. Build the Serenity client.
    let client = Client::builder(&token, intents)
        .event_handler(Handler { allowed_guild_id })
        .await
        .expect("Error creating the Discord client.");
    println!("[SETUP] Serenity client built successfully.");

    // 6. Insert the shared state and shard manager into the client's data TypeMap.
    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
        data.insert::<AppState>(app_state);
    }
    println!("[SETUP] Global data state has been inserted.");

    // 7. Return the client to the Shuttle runtime.
    Ok(client.into())
}
