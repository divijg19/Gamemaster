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

use super::state::{DuelFormat, GameState, Move};

// UI/UX: Establish the full color palette for consistent branding.
const SUCCESS_COLOR: u32 = 0x00FF00; // Green - for wins and successful actions.
const ERROR_COLOR: u32 = 0xFF0000; // Red - for errors and negative outcomes.
const ACTIVE_COLOR: u32 = 0x5865F2; // Blue - for standard in-progress game states.
const PENDING_COLOR: u32 = 0xFFA500; // Orange - for ties or actions needing user attention.

fn parse_id(s: &str) -> UserId {
    UserId::new(s.parse().unwrap_or(0))
}

// UI/UX: A helper function to create styled, ephemeral error messages.
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
    let response = CreateInteractionResponse::Message(builder);
    let _ = interaction.create_response(&ctx.http, response).await;
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
            "You cannot accept a challenge that was not meant for you.",
        )
        .await;
        return;
    }

    interaction.defer(&ctx.http).await.ok();

    let mut games = active_games.write().await;
    if let Some(game) = games.get_mut(&interaction.message.id) {
        game.accepted = true;
    } else {
        // Since we deferred, we can't create a new response. We must edit the original one.
        let embed = CreateEmbed::new()
            .title("Challenge Expired")
            .description("This duel is no longer active and cannot be accepted.")
            .color(ERROR_COLOR);
        let builder = EditInteractionResponse::new().embed(embed);
        let _ = interaction.edit_response(&ctx.http, builder).await;
        return;
    }
    drop(games);

    let bot_user = ctx.cache.current_user().clone();
    let author =
        CreateEmbedAuthor::new(&bot_user.name).icon_url(bot_user.avatar_url().unwrap_or_default());

    let embed = CreateEmbed::new()
        // CORRECTED: The method is .author(), not .set_author()
        .author(author)
        .title("The Duel Begins!")
        .description(
            "The challenge was accepted. **Round 1 is underway!**\n\nBoth players, click the button below to choose your move privately.",
        )
        .color(ACTIVE_COLOR);

    let buttons = CreateActionRow::Buttons(vec![
        CreateButton::new(format!("rps_prompt_{}", interaction.message.id))
            .label("Make Your Move")
            .style(ButtonStyle::Primary),
    ]);

    let builder = EditInteractionResponse::new()
        .embed(embed)
        .components(vec![buttons]);
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
            "You cannot decline a challenge that was not meant for you.",
        )
        .await;
        return;
    }

    interaction.defer(&ctx.http).await.ok();

    let bot_user = ctx.cache.current_user().clone();
    let author =
        CreateEmbedAuthor::new(&bot_user.name).icon_url(bot_user.avatar_url().unwrap_or_default());

    let embed = CreateEmbed::new()
        // CORRECTED: The method is .author(), not .set_author()
        .author(author)
        .title("Challenge Declined")
        .description(format!(
            "The duel was declined by <@{}>. Maybe next time!",
            p2_id
        ))
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
    let games = active_games.read().await;
    if let Some(game) = games.get(&interaction.message.id) {
        if interaction.user.id != game.player1.id && interaction.user.id != game.player2.id {
            send_ephemeral_error(
                ctx,
                interaction,
                "Not Your Game",
                "You are not a participant in this duel.",
            )
            .await;
            return;
        }
    } else {
        return;
    }

    let embed = CreateEmbed::new()
        .title("Choose Your Move")
        .description("Your choice will be hidden from your opponent.")
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
    let response = CreateInteractionResponse::Message(builder);
    let _ = interaction.create_response(&ctx.http, response).await;
}

pub async fn handle_move(
    ctx: &Context,
    interaction: &mut ComponentInteraction,
    parts: &[&str],
    active_games: &Arc<RwLock<HashMap<MessageId, GameState>>>,
) {
    let embed = CreateEmbed::new()
        .title("Move Confirmed")
        .description("Your move has been locked in. Waiting for your opponent...")
        .color(SUCCESS_COLOR);
    let builder = CreateInteractionResponseMessage::new()
        .embed(embed)
        .ephemeral(true);
    let response = CreateInteractionResponse::Message(builder);
    let _ = interaction.create_response(&ctx.http, response).await;

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
        // CORRECTED: Cannot use the helper here because a response has already been sent.
        // We must create a followup message instead.
        let embed = CreateEmbed::new()
            .title("Move Already Made")
            .description("You have already selected your move for this round.")
            .color(ERROR_COLOR);
        let builder = serenity::builder::CreateInteractionResponseFollowup::new()
            .embed(embed)
            .ephemeral(true);
        let _ = interaction.create_followup(&ctx.http, builder).await;
        return;
    }

    if interaction.user.id == game.player1.id {
        game.p1_move = Some(player_move);
    } else if interaction.user.id == game.player2.id {
        game.p2_move = Some(player_move);
    }

    let bot_user = ctx.cache.current_user().clone();

    if let (Some(p1_move), Some(p2_move)) = (game.p1_move, game.p2_move) {
        let (round_winner, result_text) = match (p1_move, p2_move) {
            (u, b) if u == b => (0, "The round is a **Tie!**".to_string()),
            (Move::Rock, Move::Scissors)
            | (Move::Paper, Move::Rock)
            | (Move::Scissors, Move::Paper) => {
                game.scores.0 += 1;
                (1, format!("**<@{}>** wins the round!", game.player1.id))
            }
            _ => {
                game.scores.1 += 1;
                (2, format!("**<@{}>** wins the round!", game.player2.id))
            }
        };

        let target_score = match game.format {
            DuelFormat::BestOf(n) => (n / 2) + 1,
            DuelFormat::RaceTo(n) => n,
        };
        let duel_over = game.scores.0 >= target_score || game.scores.1 >= target_score;

        let author = CreateEmbedAuthor::new(&bot_user.name)
            .icon_url(bot_user.avatar_url().unwrap_or_default());

        if duel_over {
            let final_winner = if game.scores.0 > game.scores.1 {
                &game.player1
            } else {
                &game.player2
            };

            let embed = CreateEmbed::new()
                .author(author)
                .title("Victory!")
                .description(format!(
                    "The duel is over! **<@{}>** is the winner!",
                    final_winner.id
                ))
                .field(
                    &game.player1.name,
                    format!("{} (Score: {})", p1_move.to_emoji(), game.scores.0),
                    true,
                )
                .field(
                    &game.player2.name,
                    format!("{} (Score: {})", p2_move.to_emoji(), game.scores.1),
                    true,
                )
                .color(SUCCESS_COLOR);

            let builder = EditInteractionResponse::new()
                .embed(embed)
                .components(vec![]);
            // CORRECTED: The method is .edit_response(), not .edit_original_response()
            let _ = interaction.edit_response(&ctx.http, builder).await;

            games.remove(&game_message_id);
        } else {
            game.p1_move = None;
            game.p2_move = None;

            let title = if round_winner == 0 {
                format!("Round {} - Tie!", game.round)
            } else {
                game.round += 1;
                format!("Round {} Results", game.round - 1)
            };

            let embed = CreateEmbed::new()
                .author(author)
                .title(title)
                .description(format!(
                    "{}\n\nStarting **Round {}!** Make your move.",
                    result_text, game.round
                ))
                .field(
                    &game.player1.name,
                    format!("{} (Score: {})", p1_move.to_emoji(), game.scores.0),
                    true,
                )
                .field(
                    &game.player2.name,
                    format!("{} (Score: {})", p2_move.to_emoji(), game.scores.1),
                    true,
                )
                .color(if round_winner == 0 {
                    PENDING_COLOR
                } else {
                    ACTIVE_COLOR
                });

            let builder = EditInteractionResponse::new().embed(embed);
            // CORRECTED: The method is .edit_response(), not .edit_original_response()
            let _ = interaction.edit_response(&ctx.http, builder).await;
        }
    } else {
        let author = CreateEmbedAuthor::new(&bot_user.name)
            .icon_url(bot_user.avatar_url().unwrap_or_default());

        let embed = CreateEmbed::new()
            .author(author)
            .title("The Duel is On!")
            .description(format!(
                "**Round {} is underway!**\n\n<@{}> has locked in their move. Waiting for the other player...",
                game.round, interaction.user.id
            ))
            .color(ACTIVE_COLOR);

        let builder = EditInteractionResponse::new().embed(embed);
        // CORRECTED: The method is .edit_response(), not .edit_original_response()
        let _ = interaction.edit_response(&ctx.http, builder).await;
    }
}
