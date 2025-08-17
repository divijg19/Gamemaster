use std::str::FromStr;
use std::sync::Arc;

use serenity::async_trait;
use serenity::client::Context;
use serenity::model::{channel::Message, gateway::Ready, id::GuildId, prelude::Interaction};
use serenity::prelude::EventHandler;
use tokio::sync::RwLock;

use crate::{AppState, commands};

enum Command {
    Ping,
    Prefix,
    Rps,
    Unknown,
}

impl FromStr for Command {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ping" => Ok(Command::Ping),
            "prefix" => Ok(Command::Prefix),
            "rps" => Ok(Command::Rps),
            _ => Ok(Command::Unknown),
        }
    }
}

pub struct Handler {
    pub allowed_guild_id: GuildId,
    pub prefix: Arc<RwLock<String>>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, mut interaction: Interaction) {
        if let Interaction::Component(component) = &mut interaction {
            let app_state = {
                let data = ctx.data.read().await;
                data.get::<AppState>()
                    .expect("Expected AppState in TypeMap.")
                    .clone()
            };

            let command_family = component.data.custom_id.split('_').next().unwrap_or("");
            if command_family == "rps" {
                commands::rps::handle_interaction(&ctx, component, app_state.active_games.clone())
                    .await;
            }
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.guild_id != Some(self.allowed_guild_id) || msg.author.bot {
            return;
        }

        let prefix_string = self.prefix.read().await.clone();
        if !msg.content.starts_with(&prefix_string) {
            return;
        }

        // DEFINITIVE FIX: Corrected the typo from `...` to `..`
        let command_body = &msg.content[prefix_string.len()..];
        let mut args = command_body.split_whitespace();
        let command_str = match args.next() {
            Some(cmd) => cmd,
            None => return,
        };

        let app_state = {
            let data = ctx.data.read().await;
            data.get::<AppState>()
                .expect("Expected AppState in TypeMap.")
                .clone()
        };

        let command = Command::from_str(command_str).unwrap_or(Command::Unknown);
        let args_vec: Vec<&str> = args.collect();

        match command {
            Command::Ping => commands::ping::run(&ctx, &msg).await,
            Command::Prefix => {
                commands::prefix::run(&ctx, &msg, self.prefix.clone(), args_vec).await
            }
            Command::Rps => {
                commands::rps::run(&ctx, &msg, args_vec, app_state.active_games.clone()).await
            }
            Command::Unknown => {}
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected and ready!", ready.user.name);
    }
}
