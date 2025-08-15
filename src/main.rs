use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use serenity::gateway::ShardManager;
use serenity::model::gateway::GatewayIntents;
use serenity::model::id::{GuildId, MessageId};
use serenity::prelude::*;
use tokio::sync::RwLock;

mod commands;
mod handler;

use crate::commands::rps::state::GameState;

pub struct ShardManagerContainer;
impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<ShardManager>;
}

pub struct AppState {
    pub active_games: Arc<RwLock<HashMap<MessageId, GameState>>>,
}

impl TypeMapKey for AppState {
    type Value = Arc<AppState>;
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to load .env file.");

    let token = env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN in the .env file.");
    let server_id_str = env::var("SERVER_ID").expect("Expected SERVER_ID in the .env file.");

    let server_id = server_id_str
        .parse::<u64>()
        .expect("SERVER_ID must be a valid number.");
    let allowed_guild_id = GuildId::new(server_id);
    let initial_prefix = Arc::new(RwLock::new("!".to_string()));

    let app_state = Arc::new(AppState {
        active_games: Arc::new(RwLock::new(HashMap::new())),
    });

    // In Serenity v0.12, interactions are received by default with GUILDS.
    let intents =
        GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(handler::Handler {
            allowed_guild_id,
            prefix: initial_prefix,
        })
        .await
        .expect("Error creating the Discord client.");

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
        data.insert::<AppState>(app_state);
    }

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
