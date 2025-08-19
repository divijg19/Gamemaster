use crate::{AppState, commands};
use serenity::async_trait;
// (✓) CORRECTED: Removed unused imports to resolve the compiler warning.
use serenity::builder::{CreateCommand, CreateCommandOption};
use serenity::client::Context;
use serenity::model::application::{CommandOptionType, Interaction};
use serenity::model::{channel::Message, gateway::Ready, id::GuildId};
use serenity::prelude::EventHandler;
use std::str::FromStr;

enum Command {
    Ping,
    Prefix,
    Rps,
    Profile,
    Work,
    Unknown,
}

impl FromStr for Command {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ping" => Ok(Command::Ping),
            "prefix" => Ok(Command::Prefix),
            "rps" => Ok(Command::Rps),
            "profile" => Ok(Command::Profile),
            "work" => Ok(Command::Work),
            _ => Ok(Command::Unknown),
        }
    }
}

pub struct Handler {
    pub allowed_guild_id: GuildId,
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let app_state = {
            ctx.data
                .read()
                .await
                .get::<AppState>()
                .expect("Expected AppState in TypeMap.")
                .clone()
        };

        match interaction {
            Interaction::Command(command) => {
                println!("[HANDLER] Received slash command: {}", command.data.name);

                match command.data.name.as_str() {
                    "ping" => commands::ping::run_slash(&ctx, &command).await,
                    "prefix" => commands::prefix::run_slash(&ctx, &command).await,
                    "profile" => commands::economy::profile::run_slash(&ctx, &command).await,
                    "work" => commands::economy::work::run_slash(&ctx, &command).await,
                    _ => {
                        // The builders for this response are inlined to avoid unused import warnings.
                        let response = serenity::builder::CreateInteractionResponseMessage::new()
                            .content("Command not implemented yet.");
                        let builder =
                            serenity::builder::CreateInteractionResponse::Message(response);
                        command.create_response(&ctx.http, builder).await.ok();
                    }
                }
            }
            Interaction::Component(mut component) => {
                let command_family = component.data.custom_id.split('_').next().unwrap_or("");
                if command_family == "rps" {
                    // (✓) CORRECTED: Cloned the Arc to pass a new reference, not move the original.
                    commands::rps::handle_interaction(
                        &ctx,
                        &mut component,
                        app_state.active_games.clone(),
                    )
                    .await;
                }
            }
            _ => {}
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.guild_id != Some(self.allowed_guild_id) || msg.author.bot {
            return;
        }

        let app_state = {
            ctx.data
                .read()
                .await
                .get::<AppState>()
                .expect("Expected AppState in TypeMap.")
                .clone()
        };
        let prefix_string = app_state.prefix.read().await.clone();

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
            Command::Ping => commands::ping::run_prefix(&ctx, &msg).await,
            Command::Prefix => commands::prefix::run_prefix(&ctx, &msg, args_vec).await,
            Command::Rps => {
                // (✓) CORRECTED: Cloned the Arc to pass a new reference, not move the original.
                commands::rps::run(&ctx, &msg, args_vec, app_state.active_games.clone()).await
            }
            Command::Profile => commands::economy::profile::run_prefix(&ctx, &msg).await,
            Command::Work => commands::economy::work::run_prefix(&ctx, &msg, args_vec).await,
            Command::Unknown => {}
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected and ready!", ready.user.name);

        if let Err(e) = self
            .allowed_guild_id
            .set_commands(
                &ctx.http,
                vec![
                    CreateCommand::new("ping").description("A simple ping command"),
                    CreateCommand::new("prefix")
                        .description("Check the bot's current command prefix"),
                    CreateCommand::new("profile")
                        .description("View your or another user's economy profile")
                        .add_option(
                            CreateCommandOption::new(
                                CommandOptionType::User,
                                "user",
                                "The user whose profile you want to see",
                            )
                            .required(false),
                        ),
                    CreateCommand::new("work")
                        .description("Work to earn coins")
                        .add_option(
                            CreateCommandOption::new(
                                CommandOptionType::String,
                                "job",
                                "The type of job you want to do",
                            )
                            .required(true)
                            .add_string_choice("Fishing", "fishing")
                            .add_string_choice("Mining", "mining")
                            .add_string_choice("Coding", "coding"),
                        ),
                ],
            )
            .await
        {
            println!("[HANDLER] Error creating guild commands: {:?}", e);
        }
        println!("[HANDLER] Successfully registered guild commands.");
    }
}
