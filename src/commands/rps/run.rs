use std::sync::Arc;
use std::time::Duration;

use serenity::builder::{
    CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter, CreateMessage, EditMessage,
};
use serenity::model::application::ButtonStyle;
use serenity::model::channel::Message;
use serenity::prelude::*;

// Use `super` to access sibling modules like `state`.
use super::state::{DuelFormat, GameState};
use crate::AppState;

/// Entry point for the `!rps` command. Creates the initial challenge.
pub async fn run(ctx: &Context, msg: &Message, args: Vec<&str>) {
    let opponent = match msg.mentions.first() {
        Some(user) if user.id != msg.author.id => user,
        _ => {
            let _ = msg
                .reply(
                    ctx,
                    "You must mention another user! e.g., `!rps @user [-b|-r] [number]`",
                )
                .await;
            return;
        }
    };

    // --- POLISHED: Argument parsing is now cleaner and more idiomatic. ---
    let mut duel_format = DuelFormat::BestOf(1);
    let mut format_str = "Single Round".to_string();
    let mut args_iter = args.iter();

    while let Some(arg) = args_iter.next() {
        match *arg {
            "-b" | "--bestof" => {
                if let Some(num_str) = args_iter.next()
                    && let Ok(num) = num_str.parse::<u32>()
                        && num > 0 {
                            duel_format = DuelFormat::BestOf(num);
                            format_str = format!("Best of {}", num);
                        }
            }
            "-r" | "--raceto" => {
                if let Some(num_str) = args_iter.next()
                    && let Ok(num) = num_str.parse::<u32>()
                        && num > 0 {
                            duel_format = DuelFormat::RaceTo(num);
                            format_str = format!("Race to {}", num);
                        }
            }
            _ => {}
        }
    }

    let embed = CreateEmbed::new()
        .title("Rock, Paper, Scissors!")
        .description(format!(
            "<@{}> has challenged <@{}>!",
            msg.author.id, opponent.id
        ))
        .field("Format", &format_str, false)
        .footer(CreateEmbedFooter::new(
            "This challenge will expire in 30 seconds.",
        ))
        .color(0x5865F2);

    let buttons = CreateActionRow::Buttons(vec![
        CreateButton::new(format!("rps_accept_{}_{}", msg.author.id, opponent.id))
            .label("Accept")
            .style(ButtonStyle::Success),
        CreateButton::new(format!("rps_decline_{}_{}", msg.author.id, opponent.id))
            .label("Decline")
            .style(ButtonStyle::Danger),
    ]);

    let builder = CreateMessage::new().embed(embed).components(vec![buttons]);
    let game_msg = match msg.channel_id.send_message(&ctx.http, builder).await {
        Ok(msg) => msg,
        Err(e) => {
            println!("Error sending game invite: {:?}", e);
            return;
        }
    };

    // --- POLISHED: Replaced `.unwrap()` with a graceful return for 100% safety. ---
    let data = ctx.data.read().await;
    let app_state = match data.get::<AppState>() {
        Some(state) => state.clone(),
        None => {
            println!("Error: AppState not found in client data.");
            return;
        }
    };

    let mut active_games = app_state.active_games.write().await;

    active_games.insert(
        game_msg.id,
        GameState {
            player1: Arc::new(msg.author.clone()),
            player2: Arc::new(opponent.clone()),
            p1_move: None,
            p2_move: None,
            accepted: false,
            format: duel_format,
            scores: (0, 0),
            round: 1,
        },
    );

    let ctx_clone = ctx.clone();
    let app_state_clone = app_state.clone();
    let game_msg_id = game_msg.id;
    let channel_id = game_msg.channel_id;

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(30)).await;
        let app_state = app_state_clone;
        let mut games = app_state.active_games.write().await;

        // POLISHED: `if` statement is collapsed per clippy's suggestion.
        if let Some(game) = games.get(&game_msg_id)
            && !game.accepted
        {
            games.remove(&game_msg_id);

            let embed = CreateEmbed::new()
                .title("Challenge Timed Out")
                .description("The challenge was not accepted in time.")
                .color(0xFF0000);

            let disabled_buttons = CreateActionRow::Buttons(vec![
                CreateButton::new("disabled_accept")
                    .label("Accept")
                    .style(ButtonStyle::Success)
                    .disabled(true),
                CreateButton::new("disabled_decline")
                    .label("Decline")
                    .style(ButtonStyle::Danger)
                    .disabled(true),
            ]);

            if let Ok(mut message) = channel_id.message(&ctx_clone.http, game_msg_id).await {
                let builder = EditMessage::new()
                    .embed(embed)
                    .components(vec![disabled_buttons]);
                let _ = message.edit(&ctx_clone.http, builder).await;
            }
        }
    });
}
