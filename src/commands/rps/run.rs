//! This module implements the `rps` command, supporting both prefix and slash commands.

use super::game::RpsGame;
use super::state::{DuelFormat, GameState};
use crate::commands::games::engine::{Game, GameManager};
use serenity::builder::{
    CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage, EditInteractionResponse, EditMessage,
};
use serenity::model::application::{
    CommandDataOption, CommandDataOptionValue, CommandInteraction, CommandOptionType,
};
use serenity::model::channel::Message;
use serenity::model::user::User; // (✓) MODIFIED: UserId is no longer needed directly
use serenity::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Registers the `/rps` slash command with betting.
pub fn register() -> CreateCommand {
    CreateCommand::new("rps")
        .description("Challenge a user to a game of Rock, Paper, Scissors.")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::User,
                "opponent",
                "The user to challenge.",
            )
            .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "format",
                "Format: 'bestof <num>' or 'raceto <num>'. Defaults to a single duel.",
            )
            .required(false),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "bet", "The amount to bet.")
                .required(false)
                .min_int_value(1),
        )
}

/// Entry point for the `/rps` slash command.
pub async fn run_slash(
    ctx: &Context,
    command: &CommandInteraction,
    game_manager: Arc<RwLock<GameManager>>,
) {
    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new()),
        )
        .await
    {
        println!("[RPS] Failed to defer slash command: {:?}", e);
        return;
    }

    let opponent = match get_opponent_from_options(command) {
        Ok(user) => user,
        Err(err_msg) => {
            send_ephemeral_error(ctx, command, &err_msg).await;
            return;
        }
    };

    let duel_format = get_format_from_options(&command.data.options).unwrap_or_default();
    let bet = get_bet_from_options(&command.data.options).unwrap_or(0);

    let game_state = GameState::new(
        Arc::new(command.user.clone()),
        Arc::new(opponent),
        duel_format,
        bet,
    );
    let rps_game = RpsGame { state: game_state };

    let (embed, components) = rps_game.render();
    let builder = EditInteractionResponse::new()
        .embed(embed)
        .components(components);

    if let Ok(game_msg) = command.edit_response(&ctx.http, builder).await {
        game_manager
            .write()
            .await
            .start_game(game_msg.id, Box::new(rps_game));
        spawn_timeout_handler(ctx.clone(), game_manager, game_msg);
    }
}

/// Entry point for the `$rps` prefix command.
pub async fn run(
    ctx: &Context,
    msg: &Message,
    args: Vec<&str>,
    game_manager: Arc<RwLock<GameManager>>,
) {
    let opponent = match msg.mentions.first() {
        Some(user) if user.id != msg.author.id && !user.bot => user.clone(),
        _ => {
            msg.reply(
                ctx,
                "You must mention a valid opponent (not a bot or yourself).",
            )
            .await
            .ok();
            return;
        }
    };

    let duel_format = parse_duel_format_from_args(&args).unwrap_or_default();
    let bet = parse_bet_from_args(&args).unwrap_or(0);

    let game_state = GameState::new(
        Arc::new(msg.author.clone()),
        Arc::new(opponent),
        duel_format,
        bet,
    );
    let rps_game = RpsGame { state: game_state };

    let (embed, components) = rps_game.render();
    let builder = CreateMessage::new()
        .embed(embed)
        .components(components)
        .reference_message(msg);

    if let Ok(game_msg) = msg.channel_id.send_message(&ctx.http, builder).await {
        game_manager
            .write()
            .await
            .start_game(game_msg.id, Box::new(rps_game));
        spawn_timeout_handler(ctx.clone(), game_manager, game_msg);
    }
}

/// Spawns a task to handle the 30-second challenge timeout.
fn spawn_timeout_handler(
    ctx: Context,
    game_manager: Arc<RwLock<GameManager>>,
    mut game_msg: Message,
) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(30)).await;
        let mut manager = game_manager.write().await;

        // (✓) FIXED: Collapsed the nested `if` statements into one.
        if let Some(game_box) = manager.get_game_mut(&game_msg.id)
            && let Some(rps_game) = game_box.as_any().downcast_ref::<RpsGame>()
            && !rps_game.state.accepted
        {
            let (embed, components) = RpsGame::render_timeout_message(&rps_game.state);
            let builder = EditMessage::new().embed(embed).components(components);
            if let Err(e) = game_msg.edit(&ctx.http, builder).await {
                println!("[RPS] Error editing timeout message: {:?}", e);
            }
            manager.remove_game(&game_msg.id);
        }
    });
}

/// Sends an ephemeral error message in response to a slash command.
async fn send_ephemeral_error(ctx: &Context, command: &CommandInteraction, content: &str) {
    let builder = EditInteractionResponse::new().content(content);
    if let Err(e) = command.edit_response(&ctx.http, builder).await {
        println!("[RPS] Error sending ephemeral error: {:?}", e);
    }
}

// --- Argument & Option Parsing Helpers ---

// (✓) FIXED: This function is now much more concise.
fn get_opponent_from_options(command: &CommandInteraction) -> Result<User, String> {
    for opt in &command.data.options {
        if opt.name == "opponent"
            && let CommandDataOptionValue::User(user_id) = opt.value
            && let Some(user) = command.data.resolved.users.get(&user_id)
        {
            if user.bot {
                return Err("You cannot challenge a bot.".to_string());
            }
            if user.id == command.user.id {
                return Err("You cannot challenge yourself.".to_string());
            }
            return Ok(user.clone());
        }
    }
    Err("A valid opponent is required.".to_string())
}

fn get_format_from_options(options: &[CommandDataOption]) -> Option<DuelFormat> {
    options.iter().find_map(|opt| {
        if opt.name == "format"
            && let CommandDataOptionValue::String(s) = &opt.value
        {
            parse_single_duel_format(s)
        } else {
            None
        }
    })
}

fn get_bet_from_options(options: &[CommandDataOption]) -> Option<i64> {
    options.iter().find_map(|opt| {
        if opt.name == "bet"
            && let CommandDataOptionValue::Integer(val) = opt.value
        {
            Some(val)
        } else {
            None
        }
    })
}

// (✓) FIXED: This function is now more concise.
fn parse_duel_format_from_args(args: &[&str]) -> Option<DuelFormat> {
    let mut args_iter = args.iter();
    while let Some(arg) = args_iter.next() {
        let is_format_flag =
            *arg == "-b" || *arg == "--bestof" || *arg == "-r" || *arg == "--raceto";
        if is_format_flag
            && let Some(num_str) = args_iter.next()
            && let Ok(num) = num_str.parse::<u32>()
            && num > 0
        {
            return match *arg {
                "-b" | "--bestof" => Some(DuelFormat::BestOf(num)),
                "-r" | "--raceto" => Some(DuelFormat::RaceTo(num)),
                _ => None,
            };
        }
    }
    None
}

fn parse_bet_from_args(args: &[&str]) -> Option<i64> {
    args.iter()
        .filter_map(|s| s.parse::<i64>().ok())
        .find(|&num| num > 0)
}

fn parse_single_duel_format(s: &str) -> Option<DuelFormat> {
    let lowercased = s.to_lowercase();
    let parts: Vec<&str> = lowercased.split_whitespace().collect();
    if parts.len() != 2 {
        return None;
    }

    let num = parts[1].parse::<u32>().ok()?;
    if num == 0 {
        return None;
    }

    match parts[0] {
        "bestof" => Some(DuelFormat::BestOf(num)),
        "raceto" => Some(DuelFormat::RaceTo(num)),
        _ => None,
    }
}
