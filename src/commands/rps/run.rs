use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serenity::builder::{
    CreateActionRow, CreateButton, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter,
    CreateMessage, EditMessage,
};
use serenity::model::application::ButtonStyle;
use serenity::model::channel::Message;
use serenity::model::id::MessageId;
// The Mention trait is not needed as .mention() is available by default on User
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

    let author = CreateEmbedAuthor::new(format!("RPS | {}", format_str));

    let embed = CreateEmbed::new()
        .author(author.clone())
        .color(PENDING_COLOR)
        .field(
            format!("{} - `0`", msg.author.mention()),
            "Status: … Waiting",
            true,
        )
        .field(
            format!("{} - `0`", opponent.mention()),
            "Status: … Waiting",
            true,
        )
        .description("A challenge has been issued!")
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

    let builder = CreateMessage::new().embed(embed).components(vec![buttons]);
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

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(30)).await;

        // CLIPPY FIX: The nested `if` statement has been collapsed for better readability.
        if let Some(game) = active_games.write().await.remove(&game_msg.id)
            && !game.accepted
        {
            let embed = CreateEmbed::new()
                .author(author)
                .color(ERROR_COLOR)
                .field(
                    format!("{} - `{}`", game.player1.mention(), game.scores.p1),
                    "Status: —",
                    true,
                )
                .field(
                    format!("{} - `{}`", game.player2.mention(), game.scores.p2),
                    "Status: Did not respond",
                    true,
                )
                .description("The challenge was not accepted in time.")
                .footer(CreateEmbedFooter::new("Challenge expired."));

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
                    .embed(embed)
                    .components(vec![disabled_buttons]);
                let _ = message.edit(&ctx_clone.http, builder).await;
            }
        }
    });
}
