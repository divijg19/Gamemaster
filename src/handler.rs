use crate::{AppState, commands, interactions};
use serenity::async_trait;
use serenity::client::Context;
use serenity::model::application::Interaction;
use serenity::model::{channel::Message, gateway::Ready, id::GuildId};
use serenity::prelude::EventHandler;
use std::str::FromStr;

enum Command {
    Ping,
    Prefix,
    Rps,
    Profile,
    Work,
    Inventory,
    Sell,
    Shop,
    Give,
    Open,
    Saga,
    Leaderboard,
    Train,
    Party,
    Craft,
    Tasks,
    Quests,
    QuestLog,
    Help,
    Blackjack,
    Poker,
    Unknown,
    Bond,
    Config,
    Contracts,
    Bestiary,
    Research,
    Progress,
    AdminUtil,
}

impl FromStr for Command {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ping" => Ok(Command::Ping),
            "prefix" => Ok(Command::Prefix),
            "rps" => Ok(Command::Rps),
            "profile" | "p" => Ok(Command::Profile),
            "work" | "w" => Ok(Command::Work),
            "inventory" | "inv" | "i" => Ok(Command::Inventory),
            "sell" => Ok(Command::Sell),
            "shop" => Ok(Command::Shop),
            "give" | "gift" => Ok(Command::Give),
            "open" | "o" => Ok(Command::Open),
            "saga" | "play" => Ok(Command::Saga),
            "leaderboard" | "lb" => Ok(Command::Leaderboard),
            "train" | "tr" => Ok(Command::Train),
            "party" | "army" => Ok(Command::Party),
            "craft" | "c" => Ok(Command::Craft),
            "tasks" | "t" => Ok(Command::Tasks),
            "quests" | "q" => Ok(Command::Quests),
            "questlog" | "ql" => Ok(Command::QuestLog),
            "help" | "h" => Ok(Command::Help),
            "blackjack" | "bj" => Ok(Command::Blackjack),
            "poker" | "pk" => Ok(Command::Poker),
            "bond" => Ok(Command::Bond),
            "config" => Ok(Command::Config),
            "contracts" => Ok(Command::Contracts),
            "bestiary" => Ok(Command::Bestiary),
            "research" => Ok(Command::Research),
            "progress" => Ok(Command::Progress),
            "adminutil" => Ok(Command::AdminUtil),
            _ => Ok(Command::Unknown),
        }
    }
}

pub struct Handler {
    pub allowed_guild_id: GuildId,
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, mut interaction: Interaction) {
        let app_state = {
            ctx.data
                .read()
                .await
                .get::<AppState>()
                .expect("Expected AppState in TypeMap.")
                .clone()
        };
        if let Interaction::Command(command) = &mut interaction {
            // (✓) DEFINITIVE FIX: Reverted to explicit, correct paths for all commands.
            match command.data.name.as_str() {
                "ping" => commands::ping::run_slash(&ctx, command).await,
                "prefix" => commands::prefix::run_slash(&ctx, command).await,
                "profile" => commands::economy::profile::run::run_slash(&ctx, command).await,
                "work" => commands::economy::work::run::run_slash(&ctx, command).await,
                "inventory" => commands::economy::inventory::run::run_slash(&ctx, command).await,
                "sell" => commands::economy::sell::run::run_slash(&ctx, command).await,
                "shop" => commands::economy::shop::run::run_slash(&ctx, command).await,
                "give" => commands::economy::give::run::run_slash(&ctx, command).await,
                "open" => commands::open::run::run_slash(&ctx, command).await,
                "saga" => commands::saga::run::run_slash(&ctx, command).await,
                "play" => commands::saga::run::run_slash(&ctx, command).await,
                "leaderboard" => commands::leaderboard::run::run_slash(&ctx, command).await,
                "train" => commands::train::run::run_slash(&ctx, command).await,
                "party" => commands::party::run::run_slash(&ctx, command).await,
                "craft" => commands::craft::run::run_slash(&ctx, command).await,
                "tasks" => commands::tasks::run::run_slash(&ctx, command).await,
                "quests" => commands::quests::run::run_slash(&ctx, command).await,
                "questlog" => commands::questlog::run::run_slash(&ctx, command).await,
                "help" => commands::help::run_slash(&ctx, command).await,
                "blackjack" => commands::blackjack::run::run_slash(&ctx, command).await,
                "poker" => commands::poker::run::run_slash(&ctx, command).await,
                "bond" => commands::bond::run::run_slash(&ctx, command).await,
                "config" => commands::config::run_slash(&ctx, command).await,
                "contracts" => commands::contracts::run::run_slash(&ctx, command).await,
                "bestiary" => commands::bestiary::run::run_slash(&ctx, command).await,
                "research" => commands::research::run::run_slash(&ctx, command).await,
                "progress" => commands::progress::run::run_slash(&ctx, command).await,
                "adminutil" => commands::admin::run_slash(&ctx, command).await,
                "rps" => {
                    commands::rps::run::run_slash(&ctx, command, app_state.game_manager.clone())
                        .await
                }
                _ => {}
            }
        } else if let Interaction::Component(component) = &mut interaction {
            let command_family = component.data.custom_id.split('_').next().unwrap_or("");
            match command_family {
                "rps" | "bj" | "poker" | "shop" | "battle" => {
                    interactions::game_handler::handle(&ctx, component, app_state).await
                }
                "help" => commands::help::handle_interaction(&ctx, component).await,
                "saga" => interactions::saga_handler::handle(&ctx, component, app_state).await,
                "leaderboard" => {
                    interactions::leaderboard_handler::handle(&ctx, component, app_state).await
                }
                "train" => interactions::train_handler::handle(&ctx, component, app_state).await,
                "party" => interactions::party_handler::handle(&ctx, component, app_state).await,
                "craft" => interactions::craft_handler::handle(&ctx, component, app_state).await,
                "task" => interactions::task_handler::handle(&ctx, component, app_state).await,
                "quest" => interactions::quest_handler::handle(&ctx, component, app_state).await,
                "questlog" => {
                    interactions::questlog_handler::handle(&ctx, component, app_state).await
                }
                "bond" => interactions::bond_handler::handle(&ctx, component, app_state).await,
                "contracts" => {
                    interactions::contracts_handler::handle(&ctx, component, app_state).await
                }
                "bestiary" => {
                    interactions::bestiary_handler::handle(&ctx, component, app_state).await
                }
                "research" => {
                    interactions::research_handler::handle(&ctx, component, app_state).await
                }
                _ => {}
            }
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
        let Some(command_body) = msg.content.strip_prefix(&prefix_string) else {
            return;
        };
        let mut args = command_body.split_whitespace();
        let Some(command_str) = args.next() else {
            return;
        };
        let command = Command::from_str(command_str).unwrap_or(Command::Unknown);
        let args_vec: Vec<&str> = args.collect();
        match command {
            Command::Ping => commands::ping::run_prefix(&ctx, &msg).await,
            Command::Prefix => commands::prefix::run_prefix(&ctx, &msg, args_vec).await,
            Command::Rps => {
                commands::rps::run::run_prefix(&ctx, &msg, args_vec, app_state.game_manager.clone())
                    .await
            }
            Command::Profile => {
                commands::economy::profile::run::run_prefix(&ctx, &msg, args_vec).await
            }
            Command::Work => commands::economy::work::run::run_prefix(&ctx, &msg, args_vec).await,
            Command::Inventory => {
                commands::economy::inventory::run::run_prefix(&ctx, &msg, args_vec).await
            }
            Command::Sell => commands::economy::sell::run::run_prefix(&ctx, &msg, args_vec).await,
            Command::Shop => commands::economy::shop::run::run_prefix(&ctx, &msg, args_vec).await,
            Command::Give => commands::economy::give::run::run_prefix(&ctx, &msg, args_vec).await,
            Command::Open => commands::open::run::run_prefix(&ctx, &msg, args_vec).await,
            Command::Saga => commands::saga::run::run_prefix(&ctx, &msg, args_vec).await,
            Command::Leaderboard => {
                commands::leaderboard::run::run_prefix(&ctx, &msg, args_vec).await
            }
            Command::Train => commands::train::run::run_prefix(&ctx, &msg, args_vec).await,
            Command::Party => commands::party::run::run_prefix(&ctx, &msg, args_vec).await,
            Command::Craft => commands::craft::run::run_prefix(&ctx, &msg, args_vec).await,
            // (✓) DEFINITIVE FIX: Removed the incorrect extra `args_vec` argument.
            Command::Tasks => commands::tasks::run::run_prefix(&ctx, &msg).await,
            Command::Quests => commands::quests::run::run_prefix(&ctx, &msg, args_vec).await,
            Command::QuestLog => commands::questlog::run::run_prefix(&ctx, &msg, args_vec).await,
            Command::Help => commands::help::run_prefix(&ctx, &msg, args_vec).await,
            Command::Blackjack => commands::blackjack::run::run_prefix(&ctx, &msg, args_vec).await,
            Command::Poker => commands::poker::run::run_prefix(&ctx, &msg, args_vec).await,
            Command::Bond => commands::bond::run::run_prefix(&ctx, &msg, args_vec).await,
            Command::Contracts => commands::contracts::run::run_prefix(&ctx, &msg, args_vec).await,
            Command::Bestiary => commands::bestiary::run::run_prefix(&ctx, &msg, args_vec).await,
            Command::Research => commands::research::run::run_prefix(&ctx, &msg, args_vec).await,
            Command::Progress => commands::progress::run::run_prefix(&ctx, &msg, args_vec).await,
            Command::AdminUtil => {
                msg.reply(&ctx.http, "Use /adminutil (slash command only). Optional: right-click > Apps if not visible.").await.ok();
            }
            Command::Config => {
                msg.reply(
                    &ctx.http,
                    "Use /config (slash command only; restricted to admin).",
                )
                .await
                .ok();
            }
            Command::Unknown => {}
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected and ready!", ready.user.name);
        let commands_to_register = vec![
            // (✓) DEFINITIVE FIX: Use the full, correct path to each `register` function.
            commands::ping::register(),
            commands::prefix::register(),
            commands::economy::profile::run::register(),
            commands::economy::work::run::register(),
            commands::economy::inventory::run::register(),
            commands::economy::sell::run::register(),
            commands::economy::shop::run::register(),
            commands::economy::give::run::register(),
            commands::open::run::register(),
            commands::saga::run::register(),
            commands::saga::run::register_play(),
            commands::leaderboard::run::register(),
            commands::train::run::register(),
            commands::party::run::register(),
            commands::craft::run::register(),
            commands::tasks::register(),
            commands::quests::register(),
            commands::questlog::register(),
            commands::blackjack::run::register(),
            commands::poker::run::register(),
            commands::rps::run::register(),
            commands::bond::run::register(),
            commands::contracts::run::register(),
            commands::bestiary::run::register(),
            commands::research::run::register(),
            commands::progress::run::register(),
            commands::config::register(),
            commands::help::register(),
            commands::admin::register(),
        ];
        if let Err(e) = self
            .allowed_guild_id
            .set_commands(&ctx.http, commands_to_register)
            .await
        {
            println!("[HANDLER] Error creating guild commands: {:?}", e);
        }
        println!("[HANDLER] Successfully registered guild commands.");
    }
}
