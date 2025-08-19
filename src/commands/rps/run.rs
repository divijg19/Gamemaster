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

use super::state::{DuelFormat, GameState};

const PENDING_COLOR: u32 = 0xFFA500;
const ERROR_COLOR: u32 = 0xFF0000;

pub async fn run(
    ctx: &Context,
    msg: &Message,
    args: Vec<&str>,
    active_games: Arc<RwLock<HashMap<MessageId, GameState>>>,
) {
    let opponent = match msg.mentions.first() {
        Some(user) if user.id != msg.author.id => user,
        _ => {
            let embed = CreateEmbed::new()
                .title("Invalid Command Usage")
                .description("To start a duel, you must mention a valid opponent.")
                .field("Example", "`!rps @username [-b 3]`", false)
                .color(ERROR_COLOR);
            let builder = CreateMessage::new().embed(embed);
            let _ = msg.channel_id.send_message(&ctx.http, builder).await;
            return;
        }
    };

    let mut duel_format = DuelFormat::BestOf(1);
    let mut args_iter = args.iter();

    while let Some(arg) = args_iter.next() {
        match *arg {
            "-b" | "--bestof" => {
                if let Some(num_str) = args_iter.next()
                    && let Ok(num) = num_str.parse::<u32>()
                    && num > 0
                {
                    duel_format = DuelFormat::BestOf(num);
                }
            }
            "-r" | "--raceto" => {
                if let Some(num_str) = args_iter.next()
                    && let Ok(num) = num_str.parse::<u32>()
                    && num > 0
                {
                    duel_format = DuelFormat::RaceTo(num);
                }
            }
            _ => {}
        }
    }

    let content = format!(
        "<@{}> has challenged <@{}> to a duel!",
        msg.author.id, opponent.id
    );

    // DEFINITIVE FIX: Simplified format call to use the Display trait.
    let embed = CreateEmbed::new()
        .title(format!("Rock Paper Scissors | {}", duel_format))
        .color(PENDING_COLOR)
        .field(msg.author.name.clone(), "Status: 🕰️ Waiting", true)
        .field("`0` vs `0`", "\u{200B}", true)
        .field(opponent.name.clone(), "Status: 🕰️ Waiting", true)
        .field("\u{200B}", "A challenge has been issued!", false)
        .footer(CreateEmbedFooter::new(format!(
            "{}, you have 30 seconds to respond.",
            opponent.name
        )));

    let buttons = CreateActionRow::Buttons(vec![
        CreateButton::new(format!("rps_accept_{}_{}", msg.author.id, opponent.id))
            .label("Accept")
            .style(ButtonStyle::Success),
        CreateButton::new(format!("rps_decline_{}_{}", msg.author.id, opponent.id))
            .label("Decline")
            .style(ButtonStyle::Danger),
    ]);

    let builder = CreateMessage::new()
        .content(content)
        .embed(embed)
        .components(vec![buttons]);
    let game_msg = match msg.channel_id.send_message(&ctx.http, builder).await {
        Ok(msg) => msg,
        Err(e) => {
            println!("Error sending game invite: {:?}", e);
            return;
        }
    };

    let game_state = GameState::new(
        Arc::new(msg.author.clone()),
        Arc::new(opponent.clone()),
        duel_format,
    );

    active_games.write().await.insert(game_msg.id, game_state);
    let ctx_clone = ctx.clone();
    let active_games_clone = active_games.clone();

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(30)).await;
        if let Some(game) = active_games_clone.read().await.get(&game_msg.id)
            && !game.accepted
        {
            active_games_clone.write().await.remove(&game_msg.id);

            let content = format!(
                "Challenge between <@{}> and <@{}> expired.",
                game.player1.id, game.player2.id
            );

            // DEFINITIVE FIX: Simplified format call to use the Display trait.
            let embed = CreateEmbed::new()
                .title(format!("Rock Paper Scissors | {}", game.format))
                .color(ERROR_COLOR)
                .field(game.player1.name.clone(), "Status: —", true)
                .field(
                    format!("`{}` vs `{}`", game.scores.p1, game.scores.p2),
                    "\u{200B}",
                    true,
                )
                .field(game.player2.name.clone(), "Status: Did not respond", true)
                .field("\u{200B}", "The challenge was not accepted in time.", false);

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

            if let Ok(mut message) = game_msg
                .channel_id
                .message(&ctx_clone.http, game_msg.id)
                .await
            {
                let builder = EditMessage::new()
                    .content(content)
                    .embed(embed)
                    .components(vec![disabled_buttons]);
                let _ = message.edit(&ctx_clone.http, builder).await;
            }
        }
    });
}
