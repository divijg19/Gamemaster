use std::str::FromStr;
use std::sync::Arc;

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::GuildId;
use serenity::model::prelude::Interaction;
use serenity::prelude::*;
use tokio::sync::RwLock;

use crate::commands;

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
    // --- THIS IS THE FINAL FIX ---
    // 1. We declare `interaction` as `mut`.
    async fn interaction_create(&self, ctx: Context, mut interaction: Interaction) {
        // 2. We get a mutable reference to the component.
        if let Interaction::Component(component) = &mut interaction {
            let command_family = component.data.custom_id.split('_').next().unwrap_or("");
            if command_family == "rps" {
                // 3. We pass the mutable reference down.
                commands::rps::handle_interaction(&ctx, component).await;
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
        let command_body = &msg.content[prefix_string.len()..];
        let mut args = command_body.split_whitespace();
        let command_str = match args.next() {
            Some(cmd) => cmd,
            None => return,
        };
        let command = Command::from_str(command_str).unwrap_or(Command::Unknown);
        let args_vec: Vec<&str> = args.collect();
        match command {
            Command::Ping => commands::ping::run(&ctx, &msg).await,
            Command::Prefix => commands::prefix::run(&ctx, &msg, &self.prefix, args_vec).await,
            Command::Rps => commands::rps::run(&ctx, &msg, args_vec).await,
            Command::Unknown => {}
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected and ready!", ready.user.name);
    }
}
