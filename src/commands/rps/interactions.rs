use std::collections::HashMap;
use std::sync::Arc;

use serenity::builder::{
    CreateActionRow, CreateButton, CreateEmbed, CreateEmbedAuthor, CreateInteractionResponse,
    CreateInteractionResponseMessage, EditInteractionResponse,
};
use serenity::model::application::{ButtonStyle, ComponentInteraction};
use serenity::model::id::{MessageId, UserId};
use serenity::prelude::*;
use tokio::sync::RwLock;

use super::state::{GameState, Move, RoundOutcome};

const SUCCESS_COLOR: u32 = 0x00FF00;
const ERROR_COLOR: u32 = 0xFF0000;
const ACTIVE_COLOR: u32 = 0x5865F2;

fn build_game_embed(
    bot_user: &serenity::model::user::CurrentUser,
    game: &GameState,
) -> CreateEmbed {
    let author =
        CreateEmbedAuthor::new(&bot_user.name).icon_url(bot_user.avatar_url().unwrap_or_default());

    let format_str = match game.format {
        super::state::DuelFormat::BestOf(n) => format!("Best of {}", n),
        super::state::DuelFormat::RaceTo(n) => format!("Race to {}", n),
    };

    let score_header = format!(
        "<@{}> **{}** vs **{}** <@{}>",
        game.player1.id, game.scores.p1, game.scores.p2, game.player2.id
    );

    let mut log_entries: Vec<String> = game
        .history
        .iter()
        .enumerate()
        .map(|(i, record)| {
            let outcome_text = match &record.outcome {
                RoundOutcome::Tie => "Draw!".to_string(),
                RoundOutcome::Winner(id) => format!("<@{}> won!", id),
            };
            format!(
                "`{}.` {} vs {} {}",
                i + 1,
                record.p1_move.to_emoji(),
                record.p2_move.to_emoji(),
                outcome_text
            )
        })
        .collect();

    let final_description = if game.is_over() {
        let winner = if game.scores.p1 > game.scores.p2 {
            &game.player1
        } else {
            &game.player2
        };
        log_entries.push(format!("\n**<@{}> is the winner!**", winner.id));
        format!(
            "**{}**\n{}\n\n{}",
            format_str,
            score_header,
            log_entries.join("\n")
        )
    } else {
        let status_line = match (game.p1_move, game.p2_move) {
            (None, None) => format!("**Round {}: Make your move!** [ … vs … ]", game.round),
            (Some(_), None) => format!(
                "**Round {}: Waiting for <@{}>...** [ ✅ vs … ]",
                game.round, game.player2.id
            ),
            (None, Some(_)) => format!(
                "**Round {}: Waiting for <@{}>...** [ … vs ✅ ]",
                game.round, game.player1.id
            ),
            (Some(_), Some(_)) => format!("**Round {}: Processing...**", game.round),
        };
        log_entries.push(format!("\n{}", status_line));
        format!(
            "**{}**\n{}\n\n{}",
            format_str,
            score_header,
            log_entries.join("\n")
        )
    };

    CreateEmbed::new()
        .author(author)
        .color(if game.is_over() {
            SUCCESS_COLOR
        } else {
            ACTIVE_COLOR
        })
        .description(final_description)
}

fn parse_id(s: &str) -> UserId {
    UserId::new(s.parse().unwrap_or(0))
}

async fn send_ephemeral_error(
    ctx: &Context,
    interaction: &ComponentInteraction,
    title: &str,
    description: &str,
) {
    let embed = CreateEmbed::new()
        .title(title)
        .description(description)
        .color(ERROR_COLOR);
    let builder = CreateInteractionResponseMessage::new()
        .embed(embed)
        .ephemeral(true);
    let _ = interaction
        .create_response(&ctx.http, CreateInteractionResponse::Message(builder))
        .await;
}

pub async fn handle_accept(
    ctx: &Context,
    interaction: &mut ComponentInteraction,
    parts: &[&str],
    active_games: &Arc<RwLock<HashMap<MessageId, GameState>>>,
) {
    let p2_id = parse_id(parts.get(3).unwrap_or(&""));
    if interaction.user.id != p2_id {
        send_ephemeral_error(
            ctx,
            interaction,
            "Not Your Duel",
            "You cannot accept a challenge.",
        )
        .await;
        return;
    }

    interaction.defer(&ctx.http).await.ok();

    let mut games = active_games.write().await;
    if let Some(game) = games.get_mut(&interaction.message.id) {
        game.accepted = true;
    } else {
        let embed = CreateEmbed::new()
            .title("Challenge Expired")
            .description("This duel is no longer active.")
            .color(ERROR_COLOR);
        let builder = EditInteractionResponse::new()
            .embed(embed)
            .components(vec![]);
        let _ = interaction.edit_response(&ctx.http, builder).await;
        return;
    }

    let game = games.get(&interaction.message.id).unwrap().clone();
    drop(games);

    let bot_user = ctx.cache.current_user().clone();
    let embed = build_game_embed(&bot_user, &game);

    let components = vec![CreateActionRow::Buttons(vec![
        CreateButton::new(format!("rps_prompt_{}", interaction.message.id))
            .label("Make Your Move")
            .style(ButtonStyle::Primary),
    ])];

    let builder = EditInteractionResponse::new()
        .embed(embed)
        .components(components);
    let _ = interaction.edit_response(&ctx.http, builder).await;
}

pub async fn handle_decline(
    ctx: &Context,
    interaction: &mut ComponentInteraction,
    parts: &[&str],
    active_games: &Arc<RwLock<HashMap<MessageId, GameState>>>,
) {
    let p2_id = parse_id(parts.get(3).unwrap_or(&""));
    if interaction.user.id != p2_id {
        send_ephemeral_error(
            ctx,
            interaction,
            "Not Your Duel",
            "You cannot decline a challenge.",
        )
        .await;
        return;
    }
    interaction.defer(&ctx.http).await.ok();
    let bot_user = ctx.cache.current_user().clone();
    let author =
        CreateEmbedAuthor::new(&bot_user.name).icon_url(bot_user.avatar_url().unwrap_or_default());
    let embed = CreateEmbed::new()
        .author(author)
        .title("Challenge Declined")
        .description(format!("The duel was declined by <@{}>.", p2_id))
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
    let builder = EditInteractionResponse::new()
        .embed(embed)
        .components(vec![disabled_buttons]);
    let _ = interaction.edit_response(&ctx.http, builder).await;
    active_games.write().await.remove(&interaction.message.id);
}

pub async fn handle_prompt(
    ctx: &Context,
    interaction: &ComponentInteraction,
    active_games: &Arc<RwLock<HashMap<MessageId, GameState>>>,
) {
    if let Some(game) = active_games.read().await.get(&interaction.message.id) {
        if interaction.user.id != game.player1.id && interaction.user.id != game.player2.id {
            send_ephemeral_error(
                ctx,
                interaction,
                "Not Your Game",
                "You are not a participant.",
            )
            .await;
            return;
        }
        let embed = CreateEmbed::new()
            .title("Choose Your Move")
            .description(format!(
                "Round {} - Your choice will be hidden.",
                game.round
            ))
            .color(ACTIVE_COLOR);
        let buttons = CreateActionRow::Buttons(vec![
            CreateButton::new(format!("rps_prompt_{}", interaction.message.id))
                .label("Rock")
                .emoji('✊')
                .style(ButtonStyle::Secondary),
            CreateButton::new(format!("rps_move_paper_{}", interaction.message.id))
                .label("Paper")
                .emoji('✋')
                .style(ButtonStyle::Secondary),
            CreateButton::new(format!("rps_move_scissors_{}", interaction.message.id))
                .label("Scissors")
                .emoji('✌')
                .style(ButtonStyle::Secondary),
        ]);
        let builder = CreateInteractionResponseMessage::new()
            .embed(embed)
            .components(vec![buttons])
            .ephemeral(true);
        let _ = interaction
            .create_response(&ctx.http, CreateInteractionResponse::Message(builder))
            .await;
    }
}

pub async fn handle_move(
    ctx: &Context,
    interaction: &mut ComponentInteraction,
    parts: &[&str],
    active_games: &Arc<RwLock<HashMap<MessageId, GameState>>>,
) {
    let game_message_id = match parts.get(3).and_then(|id_str| id_str.parse::<u64>().ok()) {
        Some(id) => MessageId::new(id),
        None => return,
    };
    let player_move = match *parts.get(2).unwrap_or(&"") {
        "rock" => Move::Rock,
        "paper" => Move::Paper,
        "scissors" => Move::Scissors,
        _ => return,
    };

    let mut games = active_games.write().await;
    let game = match games.get_mut(&game_message_id) {
        Some(g) => g,
        None => {
            let _ = interaction
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("This game has expired.")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }
    };

    if (interaction.user.id == game.player1.id && game.p1_move.is_some())
        || (interaction.user.id == game.player2.id && game.p2_move.is_some())
    {
        send_ephemeral_error(
            ctx,
            interaction,
            "Move Already Made",
            "You have already selected a move for this round.",
        )
        .await;
        return;
    }

    if interaction.user.id == game.player1.id {
        game.p1_move = Some(player_move);
    } else {
        game.p2_move = Some(player_move);
    }

    if game.p1_move.is_some() && game.p2_move.is_some() {
        game.process_round();
    }

    let is_over = game.is_over();
    let bot_user = ctx.cache.current_user().clone();
    let embed = build_game_embed(&bot_user, game);

    let components = if is_over {
        vec![]
    } else {
        vec![CreateActionRow::Buttons(vec![
            CreateButton::new(format!("rps_prompt_{}", interaction.message.id))
                .label("Make Your Move")
                .style(ButtonStyle::Primary),
        ])]
    };

    // FINAL FIX: Use the correct `CreateInteractionResponseMessage` builder.
    let builder = CreateInteractionResponseMessage::new()
        .embed(embed)
        .components(components);
    if let Err(e) = interaction
        .create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(builder))
        .await
    {
        println!("Error updating game message: {:?}", e);
    }

    if is_over {
        games.remove(&game_message_id);
    }
}
