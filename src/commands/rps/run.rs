use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serenity::builder::{
    CreateActionRow, CreateButton, CreateEmbed, CreateEmbedAuthor, CreateMessage, EditMessage,
};
use serenity::model::application::ButtonStyle;
use serenity::model::channel::Message;
use serenity::model::id::MessageId;
use serenity::prelude::*;
use tokio::sync::RwLock;

use super::state::{DuelFormat, GameState};

const PENDING_COLOR: u32 = 0xFFA500;
const ERROR_COLOR: u32 = 0xFF0000;

/// Entry point for the `!rps` command. Creates the initial challenge.
pub async fn run(
    ctx: &Context,
    msg: &Message,
    args: Vec<&str>,
    active_games: &Arc<RwLock<HashMap<MessageId, GameState>>>,
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

    let bot_user = ctx.cache.current_user().clone();
    let author =
        CreateEmbedAuthor::new(&bot_user.name).icon_url(bot_user.avatar_url().unwrap_or_default());

    let embed = CreateEmbed::new()
        .author(author.clone())
        .title("Rock, Paper, Scissors Duel!")
        .description("A challenge has been issued!")
        .field("Challenger", format!("<@{}>", msg.author.id), true)
        .field("Opponent", format!("<@{}>", opponent.id), true)
        .field("Format", &format_str, true)
        .footer(serenity::builder::CreateEmbedFooter::new(format!(
            "{}, you have 30 seconds to respond.",
            opponent.name
        )))
        .color(PENDING_COLOR);

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

    // CORRECTED: Use the new constructor from the refactored state.rs.
    // This is cleaner and ensures the game state is always created correctly.
    let game_state = GameState::new(
        Arc::new(msg.author.clone()),
        Arc::new(opponent.clone()),
        duel_format,
    );

    active_games.write().await.insert(game_msg.id, game_state);
    let games_clone = Arc::clone(active_games);
    let ctx_clone = ctx.clone();

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(30)).await;
        let mut games = games_clone.write().await;

        let should_remove = games.get(&game_msg.id).is_some_and(|g| !g.accepted);

        if should_remove {
            if let Some(game) = games.remove(&game_msg.id) {
                let embed = CreateEmbed::new()
                    .author(author)
                    .title("Challenge Expired")
                    .description("The challenge was not accepted in time.")
                    .field("Challenger", format!("<@{}>", game.player1.id), true)
                    .field("Opponent", format!("<@{}>", game.player2.id), true)
                    .color(ERROR_COLOR);

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
        }
    });
}
