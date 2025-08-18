use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serenity::builder::{
    CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter, EditMessage,
};
use serenity::model::application::{ButtonStyle, ComponentInteraction};
use serenity::model::id::{MessageId, UserId};
use serenity::prelude::*;
use tokio::sync::RwLock;

use super::state::{GameState, Move, RoundOutcome};

const SUCCESS_COLOR: u32 = 0x00FF00;
const ERROR_COLOR: u32 = 0xFF0000;
const ACTIVE_COLOR: u32 = 0x5865F2;

fn build_game_embed(game: &GameState) -> CreateEmbed {
    let log_content = if game.history.is_empty() {
        "The duel has begun! Make your move.".to_string()
    } else {
        game.history
            .iter()
            .enumerate()
            .map(|(i, record)| {
                let outcome_text = match &record.outcome {
                    RoundOutcome::Tie => "Draw!".to_string(),
                    RoundOutcome::Winner(id) => format!("<@{}> won!", id),
                };
                format!(
                    "`{}.` {} vs {} — {}",
                    i + 1,
                    record.p1_move.to_emoji(),
                    record.p2_move.to_emoji(),
                    outcome_text
                )
            })
            .collect::<Vec<String>>()
            .join("\n")
    };

    let (p1_status, p2_status) = if game.is_over() {
        if let Some(last_round) = game.history.last() {
            (
                last_round.p1_move.to_emoji().to_string(),
                last_round.p2_move.to_emoji().to_string(),
            )
        } else {
            ("—".to_string(), "—".to_string())
        }
    } else {
        let p1 = if game.p1_move.is_some() {
            "✅ Move Locked"
        } else {
            "🕰️ Waiting"
        };
        let p2 = if game.p2_move.is_some() {
            "✅ Move Locked"
        } else {
            "🕰️ Waiting"
        };
        (p1.to_string(), p2.to_string())
    };

    let footer_text = if game.is_over() {
        let winner = if game.scores.p1 > game.scores.p2 {
            &game.player1
        } else {
            &game.player2
        };
        format!("`{}` is the winner!", winner.name)
    } else {
        match (game.p1_move, game.p2_move) {
            (None, None) => "Waiting for both players...".to_string(),
            (Some(_), None) => format!("Waiting for {}...", game.player2.name),
            (None, Some(_)) => format!("Waiting for {}...", game.player1.name),
            (Some(_), Some(_)) => "Processing round...".to_string(),
        }
    };

    // DEFINITIVE FIX: Simplified format call to use the Display trait.
    CreateEmbed::new()
        .title(format!("Rock Paper Scissors | {}", game.format))
        .color(if game.is_over() {
            SUCCESS_COLOR
        } else {
            ACTIVE_COLOR
        })
        .field(
            game.player1.name.clone(),
            format!("Status: {}", p1_status),
            true,
        )
        .field(
            format!("`{}` vs `{}`", game.scores.p1, game.scores.p2),
            "\u{200B}",
            true,
        )
        .field(
            game.player2.name.clone(),
            format!("Status: {}", p2_status),
            true,
        )
        .field("\u{200B}", log_content, false)
        .footer(CreateEmbedFooter::new(footer_text))
}

fn parse_id(s: &str) -> UserId {
    UserId::new(s.parse().unwrap_or(0))
}

async fn send_ephemeral_error(
    ctx: &Context,
    interaction: &ComponentInteraction,
    description: &str,
) {
    let embed = CreateEmbed::new()
        .title("Invalid Action")
        .description(description)
        .color(ERROR_COLOR);
    let builder = serenity::builder::CreateInteractionResponseFollowup::new()
        .embed(embed)
        .ephemeral(true);
    let _ = interaction.create_followup(&ctx.http, builder).await;
}

pub async fn handle_accept(
    ctx: &Context,
    interaction: &mut ComponentInteraction,
    parts: &[&str],
    active_games: &Arc<RwLock<HashMap<MessageId, GameState>>>,
) {
    interaction.defer(&ctx.http).await.ok();
    let p2_id = parse_id(parts.get(3).unwrap_or(&""));
    if interaction.user.id != p2_id {
        send_ephemeral_error(
            ctx,
            interaction,
            "You cannot accept a challenge that was not meant for you.",
        )
        .await;
        return;
    }

    let game = {
        let mut games = active_games.write().await;
        match games.get_mut(&interaction.message.id) {
            Some(g) => {
                g.accepted = true;
                g.clone()
            }
            None => {
                return;
            }
        }
    };

    let content = format!(
        "`[ROUND {}]` <@{}> vs <@{}>",
        game.round, game.player1.id, game.player2.id
    );
    let embed = build_game_embed(&game);

    let components = vec![CreateActionRow::Buttons(vec![
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
    ])];

    let builder = EditMessage::new()
        .content(content)
        .embed(embed)
        .components(components);
    let _ = interaction.message.edit(&ctx.http, builder).await;
}

pub async fn handle_decline(
    ctx: &Context,
    interaction: &mut ComponentInteraction,
    parts: &[&str],
    active_games: &Arc<RwLock<HashMap<MessageId, GameState>>>,
) {
    interaction.defer(&ctx.http).await.ok();
    let p1_id = parse_id(parts.get(2).unwrap_or(&""));
    let p2_id = parse_id(parts.get(3).unwrap_or(&""));

    if interaction.user.id != p2_id {
        send_ephemeral_error(
            ctx,
            interaction,
            "You cannot decline a challenge that was not meant for you.",
        )
        .await;
        return;
    }

    if let Some(game) = active_games.write().await.remove(&interaction.message.id) {
        let content = format!("<@{}> declined the challenge from <@{}>.", p2_id, p1_id);

        let embed = CreateEmbed::new()
            .title(format!("Rock Paper Scissors | {}", game.format))
            .color(ERROR_COLOR)
            .field(game.player1.name.clone(), "Status: —", true)
            .field(
                format!("`{}` vs `{}`", game.scores.p1, game.scores.p2),
                "\u{200B}",
                true,
            )
            .field(game.player2.name.clone(), "Status: Declined", true)
            .field(
                "\u{200B}",
                format!("The duel was declined by <@{}>.", p2_id),
                false,
            );

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
        let builder = EditMessage::new()
            .content(content)
            .embed(embed)
            .components(vec![disabled_buttons]);
        let _ = interaction.message.edit(&ctx.http, builder).await;
    }
}

pub async fn handle_move(
    ctx: &Context,
    interaction: &mut ComponentInteraction,
    parts: &[&str],
    active_games: &Arc<RwLock<HashMap<MessageId, GameState>>>,
) {
    interaction.defer(&ctx.http).await.ok();

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

    {
        let games = active_games.read().await;
        let game = match games.get(&game_message_id) {
            Some(g) => g,
            None => return,
        };

        if interaction.user.id != game.player1.id && interaction.user.id != game.player2.id {
            send_ephemeral_error(ctx, interaction, "You are not a participant in this duel.").await;
            return;
        }

        if (interaction.user.id == game.player1.id && game.p1_move.is_some())
            || (interaction.user.id == game.player2.id && game.p2_move.is_some())
        {
            send_ephemeral_error(
                ctx,
                interaction,
                "You have already selected a move for this round.",
            )
            .await;
            return;
        }
    }

    let is_over;
    let game_clone;

    {
        let mut games = active_games.write().await;
        let game = match games.get_mut(&game_message_id) {
            Some(g) => g,
            None => return,
        };

        if interaction.user.id == game.player1.id {
            game.p1_move = Some(player_move);
        } else {
            game.p2_move = Some(player_move);
        }

        if game.p1_move.is_some() && game.p2_move.is_some() {
            game.process_round();
        }

        is_over = game.is_over();
        game_clone = game.clone();
    }

    let p1_mention = if game_clone.p1_move.is_some() && !is_over {
        format!("~~<@{}>~~", game_clone.player1.id)
    } else {
        format!("<@{}>", game_clone.player1.id)
    };

    let p2_mention = if game_clone.p2_move.is_some() && !is_over {
        format!("~~<@{}>~~", game_clone.player2.id)
    } else {
        format!("<@{}>", game_clone.player2.id)
    };

    let content = format!(
        "[ROUND {}] {} vs {}",
        game_clone.round, p1_mention, p2_mention
    );

    let embed = build_game_embed(&game_clone);

    let components = if is_over {
        vec![]
    } else {
        vec![CreateActionRow::Buttons(vec![
            CreateButton::new(format!("rps_move_rock_{}", game_message_id))
                .label("Rock")
                .emoji('✊')
                .style(ButtonStyle::Secondary),
            CreateButton::new(format!("rps_move_paper_{}", game_message_id))
                .label("Paper")
                .emoji('✋')
                .style(ButtonStyle::Secondary),
            CreateButton::new(format!("rps_move_scissors_{}", game_message_id))
                .label("Scissors")
                .emoji('✌')
                .style(ButtonStyle::Secondary),
        ])]
    };

    if let Ok(mut original_message) = interaction
        .channel_id
        .message(&ctx.http, game_message_id)
        .await
    {
        let builder = EditMessage::new()
            .content(content)
            .embed(embed)
            .components(components);
        if let Err(e) = original_message.edit(&ctx.http, builder).await {
            println!("Error editing game message: {:?}", e);
        }
    }

    if is_over {
        let active_games_clone = active_games.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(15)).await;
            active_games_clone.write().await.remove(&game_message_id);
        });
    }
}
