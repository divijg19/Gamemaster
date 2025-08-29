use crate::saga::leaderboard::LeaderboardType;
use crate::{AppState, commands};
use serenity::async_trait;
use serenity::builder::EditInteractionResponse;
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
    Help,
    Blackjack,
    Poker,
    Unknown,
}

impl FromStr for Command {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ping" => Ok(Command::Ping),
            "prefix" => Ok(Command::Prefix),
            "rps" => Ok(Command::Rps),
            "profile" | "p" => Ok(Command::Profile),
            "work" => Ok(Command::Work),
            "inventory" | "inv" | "i" => Ok(Command::Inventory),
            "sell" => Ok(Command::Sell),
            "shop" => Ok(Command::Shop),
            "give" | "gift" => Ok(Command::Give),
            "open" => Ok(Command::Open),
            "saga" | "play" => Ok(Command::Saga),
            "leaderboard" | "lb" => Ok(Command::Leaderboard),
            "train" => Ok(Command::Train),
            "party" | "army" => Ok(Command::Party),
            "help" => Ok(Command::Help),
            "blackjack" | "bj" => Ok(Command::Blackjack),
            "poker" => Ok(Command::Poker),
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

        match &mut interaction {
            Interaction::Command(command) => {
                println!("[HANDLER] Received slash command: {}", command.data.name);
                match command.data.name.as_str() {
                    "ping" => commands::ping::run_slash(&ctx, command).await,
                    "prefix" => commands::prefix::run_slash(&ctx, command).await,
                    "profile" => commands::economy::profile_slash(&ctx, command).await,
                    "work" => commands::economy::work_slash(&ctx, command).await,
                    "inventory" => commands::economy::inventory_slash(&ctx, command).await,
                    "sell" => commands::economy::sell_slash(&ctx, command).await,
                    "shop" => commands::economy::shop_slash(&ctx, command).await,
                    "give" => commands::economy::give_slash(&ctx, command).await,
                    "open" => commands::open::run_slash(&ctx, command).await,
                    "saga" => commands::saga::run_slash(&ctx, command).await,
                    "leaderboard" => commands::leaderboard::run_slash(&ctx, command).await,
                    "train" => commands::train::run_slash(&ctx, command).await,
                    "party" => commands::party::run_slash(&ctx, command).await,
                    "help" => commands::help::run_slash(&ctx, command).await,
                    "blackjack" => commands::blackjack::run_slash(&ctx, command).await,
                    "poker" => commands::poker::run_slash(&ctx, command).await,
                    "rps" => {
                        commands::rps::run_slash(&ctx, command, app_state.game_manager.clone())
                            .await
                    }
                    _ => {}
                }
            }
            Interaction::Component(component) => {
                let command_family = component.data.custom_id.split('_').next().unwrap_or("");

                match command_family {
                    "rps" | "bj" | "poker" | "shop" => {
                        let db = app_state.db.clone();
                        let mut game_manager = app_state.game_manager.write().await;
                        game_manager.on_interaction(&ctx, component, &db).await;
                    }
                    "help" => {
                        commands::help::handle_interaction(&ctx, component).await;
                    }
                    "saga" => {
                        let db = app_state.db.clone();
                        let custom_id_parts: Vec<&str> =
                            component.data.custom_id.split('_').collect();
                        match custom_id_parts.get(1) {
                            Some(&"map") => {
                                component.defer(&ctx.http).await.ok();
                                let spend_result = crate::database::profile::spend_action_points(
                                    &db,
                                    component.user.id,
                                    1,
                                )
                                .await;
                                let mut builder = EditInteractionResponse::new();
                                match spend_result {
                                    Ok(true) => {
                                        builder = builder.content("You spend 1 AP and venture into the world...\n\n_(Battle System Coming Soon!)_").components(vec![]);
                                    }
                                    Ok(false) => {
                                        builder =
                                            builder.content("You don't have enough Action Points!");
                                    }
                                    Err(_) => {
                                        builder = builder.content("A database error occurred.");
                                    }
                                }
                                component.edit_response(&ctx.http, builder).await.ok();
                            }
                            Some(&"tavern") => {
                                component.defer_ephemeral(&ctx.http).await.ok();
                                let profile = crate::database::profile::get_or_create_profile(
                                    &db,
                                    component.user.id,
                                )
                                .await
                                .unwrap();
                                let recruits = crate::database::profile::get_pets_by_ids(
                                    &db,
                                    &crate::commands::saga::tavern::TAVERN_RECRUITS,
                                )
                                .await
                                .unwrap_or_default();
                                let (embed, components) =
                                    crate::commands::saga::tavern::create_tavern_menu(
                                        &recruits,
                                        profile.balance,
                                    );
                                let builder = EditInteractionResponse::new()
                                    .embed(embed)
                                    .components(components);
                                component.edit_response(&ctx.http, builder).await.ok();
                            }
                            Some(&"hire") => {
                                component.defer_ephemeral(&ctx.http).await.ok();
                                let pet_id_to_hire = custom_id_parts[2].parse::<i32>().unwrap();
                                let result = crate::database::profile::hire_mercenary(
                                    &db,
                                    component.user.id,
                                    pet_id_to_hire,
                                    crate::commands::saga::tavern::HIRE_COST,
                                )
                                .await;
                                let mut builder = EditInteractionResponse::new().components(vec![]);
                                match result {
                                    Ok(pet_name) => {
                                        builder = builder.content(format!("You slide {} coins across the table. **{}** joins your army!", crate::commands::saga::tavern::HIRE_COST, pet_name));
                                    }
                                    Err(e) => {
                                        builder = builder.content(format!("Hiring failed: {}", e));
                                    }
                                }
                                component.edit_response(&ctx.http, builder).await.ok();
                            }
                            Some(&"team") => {
                                component.defer_ephemeral(&ctx.http).await.ok();
                                crate::database::profile::update_and_get_saga_profile(
                                    &db,
                                    component.user.id,
                                )
                                .await
                                .ok();
                                let pets = crate::database::profile::get_player_pets(
                                    &db,
                                    component.user.id,
                                )
                                .await
                                .unwrap_or_default();
                                let (embed, components) =
                                    crate::commands::party::ui::create_party_view(&pets);
                                let builder = EditInteractionResponse::new()
                                    .embed(embed)
                                    .components(components);
                                component.edit_response(&ctx.http, builder).await.ok();
                            }
                            _ => {}
                        }
                    }
                    "leaderboard" => {
                        let db = app_state.db.clone();
                        component.defer(&ctx.http).await.ok();
                        let board_type = match component.data.custom_id.as_str() {
                            "leaderboard_wealth" => LeaderboardType::Wealth,
                            "leaderboard_streak" => LeaderboardType::WorkStreak,
                            _ => LeaderboardType::Gamemaster,
                        };
                        let entries = match board_type {
                            LeaderboardType::Gamemaster => {
                                crate::database::leaderboard::get_gamemaster_leaderboard(&db, 10)
                                    .await
                            }
                            LeaderboardType::Wealth => {
                                crate::database::leaderboard::get_wealth_leaderboard(&db, 10).await
                            }
                            LeaderboardType::WorkStreak => {
                                crate::database::leaderboard::get_streak_leaderboard(&db, 10).await
                            }
                        }
                        .unwrap_or_default();
                        let embed = crate::commands::leaderboard::ui::create_leaderboard_embed(
                            &ctx, &entries, board_type,
                        )
                        .await;
                        let components = vec![
                            crate::commands::leaderboard::ui::create_leaderboard_buttons(
                                board_type,
                            ),
                        ];
                        let builder = EditInteractionResponse::new()
                            .embed(embed)
                            .components(components);
                        component.edit_response(&ctx.http, builder).await.ok();
                    }
                    "train" => {
                        let db = app_state.db.clone();
                        component.defer_ephemeral(&ctx.http).await.ok();
                        let custom_id_parts: Vec<&str> =
                            component.data.custom_id.split('_').collect();
                        match custom_id_parts.get(1) {
                            Some(&"select") => {
                                let pet_id_str = if let serenity::model::application::ComponentInteractionDataKind::StringSelect { values } = &component.data.kind { &values[0] } else { return; };
                                let pet_id = pet_id_str.parse::<i32>().unwrap();
                                let (embed, components) =
                                    crate::commands::train::ui::create_stat_selection_menu(pet_id);
                                let builder = EditInteractionResponse::new()
                                    .embed(embed)
                                    .components(components);
                                component.edit_response(&ctx.http, builder).await.ok();
                            }
                            Some(&"stat") => {
                                let stat = custom_id_parts[2];
                                let pet_id = custom_id_parts[3].parse::<i32>().unwrap();
                                let success = crate::database::profile::start_training(
                                    &db,
                                    component.user.id,
                                    pet_id,
                                    stat,
                                    2,
                                    1,
                                )
                                .await
                                .unwrap_or(false);
                                let mut builder = EditInteractionResponse::new().components(vec![]);
                                if success {
                                    builder = builder.content(format!(
                                        "Training has begun! Your pet will gain +1 {} in 2 hours.",
                                        stat
                                    ));
                                } else {
                                    builder = builder.content("Failed to start training. You may not have enough Training Points, or the pet is already training.");
                                }
                                component.edit_response(&ctx.http, builder).await.ok();
                            }
                            _ => {}
                        }
                    }
                    "party" => {
                        let db = app_state.db.clone();
                        component.defer_ephemeral(&ctx.http).await.ok();
                        let action = component.data.custom_id.split('_').nth(1).unwrap_or("");
                        let is_adding = action == "add";
                        let pet_id_str = if let serenity::model::application::ComponentInteractionDataKind::StringSelect { values } = &component.data.kind { &values[0] } else { return; };
                        let pet_id = pet_id_str.parse::<i32>().unwrap();
                        let result = crate::database::profile::set_pet_party_status(
                            &db,
                            component.user.id,
                            pet_id,
                            is_adding,
                        )
                        .await;
                        let pets =
                            crate::database::profile::get_player_pets(&db, component.user.id)
                                .await
                                .unwrap_or_default();
                        let (embed, components) =
                            crate::commands::party::ui::create_party_view(&pets);
                        let mut builder = EditInteractionResponse::new()
                            .embed(embed)
                            .components(components);
                        if let Ok(false) = result
                            && is_adding
                        {
                            builder =
                                builder.content("Could not add pet: Your party is full (5/5).");
                        }
                        component.edit_response(&ctx.http, builder).await.ok();
                    }
                    _ => {}
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
                commands::rps::run(&ctx, &msg, args_vec, app_state.game_manager.clone()).await;
            }
            Command::Profile => commands::economy::profile_prefix(&ctx, &msg, args_vec).await,
            Command::Work => commands::economy::work_prefix(&ctx, &msg, args_vec).await,
            Command::Inventory => commands::economy::inventory_prefix(&ctx, &msg, args_vec).await,
            Command::Sell => commands::economy::sell_prefix(&ctx, &msg, args_vec).await,
            Command::Shop => commands::economy::shop_prefix(&ctx, &msg, args_vec).await,
            Command::Give => commands::economy::give_prefix(&ctx, &msg, args_vec).await,
            Command::Open => commands::open::run_prefix(&ctx, &msg, args_vec).await,
            Command::Saga => commands::saga::run_prefix(&ctx, &msg, args_vec).await,
            Command::Leaderboard => commands::leaderboard::run_prefix(&ctx, &msg, args_vec).await,
            Command::Train => commands::train::run_prefix(&ctx, &msg, args_vec).await,
            Command::Party => commands::party::run_prefix(&ctx, &msg, args_vec).await,
            Command::Help => commands::help::run_prefix(&ctx, &msg, args_vec).await,
            Command::Blackjack => commands::blackjack::run_prefix(&ctx, &msg, args_vec).await,
            Command::Poker => commands::poker::run_prefix(&ctx, &msg, args_vec).await,
            Command::Unknown => {}
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected and ready!", ready.user.name);

        use serenity::builder::{CreateCommand, CreateCommandOption};
        use serenity::model::application::CommandOptionType;

        let mut commands_to_register = vec![
            CreateCommand::new("ping").description("Checks the bot's latency."),
            CreateCommand::new("prefix").description("Check the bot's current command prefix."),
            CreateCommand::new("profile")
                .description("View your or another user's economy profile.")
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::User,
                        "user",
                        "The user whose profile you want to see.",
                    )
                    .required(false),
                ),
            CreateCommand::new("work")
                .description("Work a job to earn coins and resources.")
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "job",
                        "The type of job you want to do.",
                    )
                    .required(true)
                    .add_string_choice("Fishing", "fishing")
                    .add_string_choice("Mining", "mining")
                    .add_string_choice("Coding", "coding"),
                ),
        ];

        commands_to_register.extend(vec![
            commands::economy::inventory::register(),
            commands::economy::sell::register(),
            commands::economy::shop::register(),
            commands::economy::give::register(),
            commands::open::register(),
            commands::saga::register(),
            commands::leaderboard::register(),
            commands::train::register(),
            commands::party::register(),
            commands::blackjack::register(),
            commands::poker::register(),
            commands::rps::register(),
            commands::help::register(),
        ]);

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
