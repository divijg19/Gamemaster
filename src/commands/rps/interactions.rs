use std::collections::HashMap;
use std::sync::Arc;

use serenity::builder::{
    CreateActionRow, CreateButton, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage, EditMessage,
};
use serenity::model::application::{ButtonStyle, ComponentInteraction};
use serenity::model::id::{MessageId, UserId};
use serenity::prelude::*;
use tokio::sync::RwLock;

use super::state::{GameState, Move, RoundOutcome};

const SUCCESS_COLOR: u32 = 0x00FF00;
const ERROR_COLOR: u32 = 0xFF0000;
const ACTIVE_COLOR: u32 = 0x5865F2;

// --- FINAL UI REWRITE: This renderer now builds a single description string for the definitive layout. ---
fn build_game_embed(game: &GameState) -> CreateEmbed {
    let format_str = match game.format {
        super::state::DuelFormat::BestOf(n) => format!("Best of {}", n),
        super::state::DuelFormat::RaceTo(n) => format!("Race to {}", n),
    };
    let author = CreateEmbedAuthor::new(format!("RPS | {}", format_str));

    // Part 1: The Player Status Block
    let (p1_status, p2_status) = if game.is_over() {
        (
            game.history.last().unwrap().p1_move.to_emoji().to_string(),
            game.history.last().unwrap().p2_move.to_emoji().to_string(),
        )
    } else {
        let p1 = if game.p1_move.is_some() {
            "✅ Move Locked"
        } else {
            "… Waiting"
        };
        let p2 = if game.p2_move.is_some() {
            "✅ Move Locked"
        } else {
            "… Waiting"
        };
        (p1.to_string(), p2.to_string())
    };

    let player_block = format!(
        "<@{}> - `{}`\nStatus: {}\n\n<@{}> - `{}`\nStatus: {}",
        game.player1.id, game.scores.p1, p1_status, game.player2.id, game.scores.p2, p2_status
    );

    // Part 2: The Game Log Block
    let log_block = if game.history.is_empty() {
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

    // Part 3: The Footer and Final Assembly
    let footer_text = if game.is_over() {
        let winner = if game.scores.p1 > game.scores.p2 {
            &game.player1
        } else {
            &game.player2
        };
        format!("{} is the winner!", winner.name)
    } else {
        let status = match (game.p1_move, game.p2_move) {
            (None, None) => "Waiting for both players...".to_string(),
            (Some(_), None) => format!("Waiting for {}...", game.player2.name),
            (None, Some(_)) => format!("Waiting for {}...", game.player1.name),
            (Some(_), Some(_)) => "Processing round...".to_string(),
        };
        format!("Round {} | {}", game.round, status)
    };

    // The final description combines the blocks without a separator.
    CreateEmbed::new()
        .author(author)
        .color(if game.is_over() {
            SUCCESS_COLOR
        } else {
            ACTIVE_COLOR
        })
        .description(format!("{}\n\n{}", player_block, log_block))
        .footer(CreateEmbedFooter::new(footer_text))
}

fn parse_id(s: &str) -> UserId {
    UserId::new(s.parse().unwrap_or(0))
}

async fn send_ephemeral_followup_error(
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
        send_ephemeral_followup_error(
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
                let embed = CreateEmbed::new()
                    .title("Challenge Expired")
                    .description("This duel is no longer active.")
                    .color(ERROR_COLOR);
                let builder = EditMessage::new().embed(embed).components(vec![]);
                let _ = interaction.message.edit(&ctx.http, builder).await;
                return;
            }
        }
    };

    let embed = build_game_embed(&game);
    let components = vec![CreateActionRow::Buttons(vec![
        CreateButton::new(format!("rps_prompt_{}", interaction.message.id))
            .label("Make Your Move")
            .style(ButtonStyle::Primary),
    ])];

    let builder = EditMessage::new().embed(embed).components(components);
    let _ = interaction.message.edit(&ctx.http, builder).await;
}

pub async fn handle_decline(
    ctx: &Context,
    interaction: &mut ComponentInteraction,
    parts: &[&str],
    active_games: &Arc<RwLock<HashMap<MessageId, GameState>>>,
) {
    interaction.defer(&ctx.http).await.ok();

    let p2_id = parse_id(parts.get(3).unwrap_or(&""));
    if interaction.user.id != p2_id {
        send_ephemeral_followup_error(
            ctx,
            interaction,
            "You cannot decline a challenge that was not meant for you.",
        )
        .await;
        return;
    }

    active_games.write().await.remove(&interaction.message.id);

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
    let builder = EditMessage::new()
        .embed(embed)
        .components(vec![disabled_buttons]);
    let _ = interaction.message.edit(&ctx.http, builder).await;
}

pub async fn handle_prompt(
    ctx: &Context,
    interaction: &ComponentInteraction,
    active_games: &Arc<RwLock<HashMap<MessageId, GameState>>>,
) {
    if let Some(game) = active_games.read().await.get(&interaction.message.id) {
        if interaction.user.id != game.player1.id && interaction.user.id != game.player2.id {
            let embed = CreateEmbed::new()
                .title("Not Your Game")
                .description("You are not a participant in this duel.")
                .color(ERROR_COLOR);
            let builder = CreateInteractionResponseMessage::new()
                .embed(embed)
                .ephemeral(true);
            let _ = interaction
                .create_response(&ctx.http, CreateInteractionResponse::Message(builder))
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

    let mut games = active_games.write().await;
    let game = match games.get_mut(&game_message_id) {
        Some(g) => g,
        None => {
            send_ephemeral_followup_error(
                ctx,
                interaction,
                "This game has expired or could not be found.",
            )
            .await;
            return;
        }
    };

    if (interaction.user.id == game.player1.id && game.p1_move.is_some())
        || (interaction.user.id == game.player2.id && game.p2_move.is_some())
    {
        send_ephemeral_followup_error(
            ctx,
            interaction,
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

    let round_just_finished = game.p1_move.is_some() && game.p2_move.is_some();
    if round_just_finished {
        game.process_round();
    }

    let is_over = game.is_over();
    let game_clone = game.clone();

    if is_over {
        games.remove(&game_message_id);
    }

    drop(games);

    let embed = build_game_embed(&game_clone);

    let components = if is_over {
        vec![]
    } else {
        vec![CreateActionRow::Buttons(vec![
            CreateButton::new(format!("rps_prompt_{}", interaction.message.id))
                .label("Make Your Move")
                .style(ButtonStyle::Primary),
        ])]
    };

    if let Ok(mut original_message) = interaction
        .channel_id
        .message(&ctx.http, game_message_id)
        .await
    {
        let builder = EditMessage::new().embed(embed).components(components);
        if let Err(e) = original_message.edit(&ctx.http, builder).await {
            println!("Error editing game message: {:?}", e);
        }
    }
}
