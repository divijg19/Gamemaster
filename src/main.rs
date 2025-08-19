use serenity::model::gateway::GatewayIntents;
use serenity::model::id::GuildId;
use serenity::prelude::*;
use shuttle_runtime::SecretStore;
use shuttle_serenity::ShuttleSerenity;
use sqlx::PgPool;
use std::sync::Arc;

// These are our application modules
mod commands;
mod database;
mod handler;
mod model;

// (✓) Import our centralized data structures from the new model.rs file.
use crate::handler::Handler;
use crate::model::{AppState, ShardManagerContainer};

#[shuttle_runtime::main]
async fn serenity(
    #[shuttle_shared_db::Postgres] pool: PgPool,
    #[shuttle_runtime::Secrets] secrets: SecretStore,
) -> ShuttleSerenity {
    // Run database migrations on startup. This is a critical step to ensure
    // the database schema is always in sync with the application code.
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations.");
    println!("Database migrations run successfully.");

    // Load secrets from the Shuttle secret store.
    let token = secrets
        .get("DISCORD_TOKEN")
        .expect("'DISCORD_TOKEN' was not found in the secret store.");
    let server_id_str = secrets
        .get("SERVER_ID")
        .expect("'SERVER_ID' was not found in the secret store.");

    let server_id = server_id_str
        .parse::<u64>()
        .expect("SERVER_ID must be a valid number.");
    let allowed_guild_id = GuildId::new(server_id);

    // (✓) All shared state is now neatly bundled into the AppState struct from our model.
    let app_state = Arc::new(AppState {
        active_games: Default::default(), // A cleaner way to initialize an empty HashMap
        db: pool,
        prefix: Arc::new(tokio::sync::RwLock::new("$".to_string())),
    });

    // Set gateway intents, which decides what events the bot will be notified about.
    // (✓) Corrected a typo where GUILDS was listed twice.
    let intents =
        GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let client = Client::builder(&token, intents)
        .event_handler(Handler { allowed_guild_id })
        .await
        .expect("Error creating the Discord client.");

    // Insert the shared state containers into the client's global data TypeMap.
    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
        data.insert::<AppState>(app_state);
    }

    Ok(client.into())
}
