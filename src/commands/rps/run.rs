//! This module implements the `!rps` command, which is responsible for
//! parsing user input, creating a new `RpsGame`, and registering it
//! with the global `GameManager`.

use super::game::RpsGame;
use super::state::{DuelFormat, GameState};
use crate::commands::games::{Game, GameManager};
use serenity::builder::{CreateMessage, EditMessage};
use serenity::model::channel::Message;
use serenity::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// The entry point for the `!rps` prefix command.
pub async fn run(
    ctx: &Context,
    msg: &Message,
    args: Vec<&str>,
    game_manager: Arc<RwLock<GameManager>>,
) {
    let opponent = match msg.mentions.first() {
        Some(user) if user.id != msg.author.id => user,
        _ => {
            if let Err(e) = msg
                .reply(
                    &ctx.http,
                    "You need to mention a valid opponent to challenge.",
                )
                .await
            {
                println!("[RPS] Error sending opponent error message: {:?}", e);
            }
            return;
        }
    };

    let mut duel_format = DuelFormat::BestOf(3);
    let mut args_iter = args.iter();
    while let Some(arg) = args_iter.next() {
        match *arg {
            "-b" | "--bestof" => {
                if let Some(num_str) = args_iter.next()
                    && let Ok(num) = num_str.parse::<u32>()
                        && num > 0 {
                            duel_format = DuelFormat::BestOf(num);
                        }
            }
            "-r" | "--raceto" => {
                if let Some(num_str) = args_iter.next()
                    && let Ok(num) = num_str.parse::<u32>()
                        && num > 0 {
                            duel_format = DuelFormat::RaceTo(num);
                        }
            }
            _ => {}
        }
    }

    let bet = 0;

    let game_state = GameState::new(
        Arc::new(msg.author.clone()),
        Arc::new(opponent.clone()),
        duel_format,
        bet,
    );
    let rps_game = RpsGame { state: game_state };

    let (embed, components) = rps_game.render();
    let builder = CreateMessage::new()
        .embed(embed)
        .components(components)
        .reference_message(msg);

    let game_msg = match msg.channel_id.send_message(&ctx.http, builder).await {
        Ok(m) => m,
        Err(e) => {
            println!("[RPS] Error sending initial game message: {:?}", e);
            return;
        }
    };

    game_manager
        .write()
        .await
        .start_game(game_msg.id, Box::new(rps_game));

    let game_manager_clone = game_manager.clone();
    let ctx_clone = ctx.clone();
    // (✓) CORRECTED: The message object is now declared as mutable.
    let mut game_msg_clone = game_msg.clone();

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(30)).await;

        let mut manager = game_manager_clone.write().await;

        if let Some(game_box) = manager.get_game_mut(&game_msg_clone.id)
            && let Some(rps_game) = game_box.as_any().downcast_ref::<RpsGame>()
                && !rps_game.state.accepted {
                    println!(
                        "[RPS] Challenge for message {} timed out.",
                        game_msg_clone.id
                    );

                    let (embed, components) = RpsGame::render_timeout_message(&rps_game.state);
                    let builder = EditMessage::new()
                        .content("")
                        .embed(embed)
                        .components(components);
                    // (✓) This call is now valid because `game_msg_clone` is mutable.
                    if let Err(e) = game_msg_clone.edit(&ctx_clone.http, builder).await {
                        println!("[RPS] Error editing timeout message: {:?}", e);
                    }

                    manager.remove_game(&game_msg_clone.id);
                }
    });
}
