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
const PENDING_COLOR: u32 = 0xFFA500;

// CORRECTED: The function now takes a reference to the outcome to avoid the move error.
fn build_game_embed(
    bot_user: &serenity::model::user::CurrentUser,
    game: &GameState,
    outcome: &Option<RoundOutcome>,
) -> CreateEmbed {
    let author =
        CreateEmbedAuthor::new(&bot_user.name).icon_url(bot_user.avatar_url().unwrap_or_default());

    let mut embed = CreateEmbed::new().author(author);

    if game.is_over() {
        let winner = if game.scores.p1 > game.scores.p2 {
            &game.player1
        } else {
            &game.player2
        };
        embed = embed
            .title("Victory!")
            .description(format!(
                "The duel is over! **<@{}>** is the winner!",
                winner.id
            ))
            .color(SUCCESS_COLOR);
    } else if let Some(round_outcome) = outcome {
        let result_text = match round_outcome {
            RoundOutcome::Tie => "The round was a **Tie!**".to_string(),
            RoundOutcome::Winner(u) => format!("**{}** wins the round!", u.name),
        };
        embed = embed
            .title(format!("Round {} Results", game.round - 1))
            .description(format!(
                "{}\n\nStarting **Round {}!** Make your move.",
                result_text, game.round
            ))
            .color(if matches!(round_outcome, RoundOutcome::Tie) {
                PENDING_COLOR
            } else {
                ACTIVE_COLOR
            });
    } else {
        embed = embed
            .title(format!("Round {}", game.round))
            .description("Both players, make your move.")
            .color(ACTIVE_COLOR);
    }

    let p1_status = if game.p1_move.is_some() {
        "Move Locked In"
    } else {
        "Waiting..."
    };
    let p2_status = if game.p2_move.is_some() {
        "Move Locked In"
    } else {
        "Waiting..."
    };

    embed
        .field(
            &game.player1.name,
            format!("**Move:** {}\n**Score:** {}", p1_status, game.scores.p1),
            true,
        )
        .field(
            &game.player2.name,
            format!("**Move:** {}\n**Score:** {}", p2_status, game.scores.p2),
            true,
        )
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
    let embed = build_game_embed(&bot_user, &game, &None);

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
            CreateButton::new(format!("rps_move_rock_{}", interaction.message.id))
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
    let embed = CreateEmbed::new()
        .title("Move Confirmed")
        .description("Waiting for your opponent...")
        .color(SUCCESS_COLOR);
    let builder = CreateInteractionResponseMessage::new()
        .embed(embed)
        .ephemeral(true);
    let _ = interaction
        .create_response(&ctx.http, CreateInteractionResponse::Message(builder))
        .await;

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
        None => return,
    };

    if (interaction.user.id == game.player1.id && game.p1_move.is_some())
        || (interaction.user.id == game.player2.id && game.p2_move.is_some())
    {
        let embed = CreateEmbed::new()
            .title("Move Already Made")
            .description("You have already selected a move.")
            .color(ERROR_COLOR);
        let builder = serenity::builder::CreateInteractionResponseFollowup::new()
            .embed(embed)
            .ephemeral(true);
        let _ = interaction.create_followup(&ctx.http, builder).await;
        return;
    }

    if interaction.user.id == game.player1.id {
        game.p1_move = Some(player_move);
    } else {
        game.p2_move = Some(player_move);
    }

    // CORRECTED: The borrow checker errors are now resolved.
    let round_outcome = game.process_round(); // This can now happen without holding a problematic borrow.
    let is_over = game.is_over();

    let bot_user = ctx.cache.current_user().clone();
    // CORRECTED: Pass a reference to fix the move error.
    let embed = build_game_embed(&bot_user, game, &round_outcome);

    let components = if is_over {
        vec![]
    } else {
        vec![CreateActionRow::Buttons(vec![
            CreateButton::new(format!("rps_prompt_{}", interaction.message.id))
                .label("Make Your Move")
                .style(ButtonStyle::Primary),
        ])]
    };

    // CORRECTED: This check now works because `round_outcome` was not moved.
    if round_outcome.is_some() {
        game.prepare_for_next_round();
    }

    if is_over {
        games.remove(&game_message_id);
    }

    drop(games);

    let builder = EditInteractionResponse::new()
        .embed(embed)
        .components(components);
    let _ = interaction.edit_response(&ctx.http, builder).await;
}
