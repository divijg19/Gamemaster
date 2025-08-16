use std::collections::HashMap;
use std::sync::Arc;

use serenity::gateway::ShardManager;
use serenity::model::gateway::GatewayIntents;
use serenity::model::id::{GuildId, MessageId};
use serenity::prelude::*;
use shuttle_runtime::SecretStore;
use shuttle_serenity::ShuttleSerenity;
use sqlx::PgPool;
use tokio::sync::RwLock;

mod commands;
mod handler;

use crate::commands::rps::state::GameState;

pub struct ShardManagerContainer;
impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<ShardManager>;
}

// The AppState now includes the database connection pool for use in commands and events.
pub struct AppState {
    pub active_games: Arc<RwLock<HashMap<MessageId, GameState>>>,
    pub db: PgPool,
}

impl TypeMapKey for AppState {
    type Value = Arc<AppState>;
}

#[shuttle_runtime::main]
async fn serenity(
    #[shuttle_shared_db::Postgres] pool: PgPool,
    #[shuttle_runtime::Secrets] secrets: SecretStore,
) -> ShuttleSerenity {
    // It's good practice to run database migrations here.
    // For example:
    // sqlx::migrate!().run(&pool).await.expect("Migrations failed");

    // Get the Discord token and server ID from the Shuttle secret store.
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
    let initial_prefix = Arc::new(RwLock::new("!".to_string()));

    let app_state = Arc::new(AppState {
        active_games: Arc::new(RwLock::new(HashMap::new())),
        db: pool,
    });

    // Set gateway intents, which specify which events the bot will receive.
    let intents =
        GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    // Build the Serenity client.
    let client = Client::builder(&token, intents)
        .event_handler(handler::Handler {
            allowed_guild_id,
            prefix: initial_prefix,
        })
        .await
        .expect("Error creating the Discord client.");

    // Store the ShardManager and AppState in the client's data TypeMap.
    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
        data.insert::<AppState>(app_state);
    }

    // The shuttle_serenity crate manages the client's lifecycle.
    Ok(client.into())
}
