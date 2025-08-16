use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serenity::builder::{
    CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter, CreateMessage, EditMessage,
};
use serenity::model::application::ButtonStyle;
use serenity::model::channel::Message;
use serenity::model::id::MessageId;
use serenity::prelude::*;
use tokio::sync::RwLock;

// Use `super` to access sibling modules like `state`.
use super::state::{DuelFormat, GameState};

/// Entry point for the `!rps` command. Creates the initial challenge.
// The function signature is updated to accept the active_games state directly.
pub async fn run(
    ctx: &Context,
    msg: &Message,
    args: Vec<&str>,
    active_games: &Arc<RwLock<HashMap<MessageId, GameState>>>,
) {
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

    let mut duel_format = DuelFormat::BestOf(1);
    let mut format_str = "Single Round".to_string();
    let mut args_iter = args.iter();

    while let Some(arg) = args_iter.next() {
        match *arg {
            "-b" | "--bestof" => {
                if let Some(num_str) = args_iter.next()
                    && let Ok(num) = num_str.parse::<u32>()
                    && num > 0
                {
                    duel_format = DuelFormat::BestOf(num);
                    format_str = format!("Best of {}", num);
                }
            }
            "-r" | "--raceto" => {
                if let Some(num_str) = args_iter.next()
                    && let Ok(num) = num_str.parse::<u32>()
                    && num > 0
                {
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

    // We no longer need to fetch AppState; we use the `active_games` parameter.
    let mut games = active_games.write().await;
    games.insert(
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

    // Drop the write lock before spawning the task.
    drop(games);

    let ctx_clone = ctx.clone();
    // Clone the Arc for the spawned task.
    let games_clone = Arc::clone(active_games);
    let game_msg_id = game_msg.id;
    let channel_id = game_msg.channel_id;

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(30)).await;
        let mut games = games_clone.write().await;

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
