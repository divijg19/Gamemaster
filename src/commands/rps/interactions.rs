use serenity::builder::{
    CreateActionRow, CreateButton, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseFollowup, CreateInteractionResponseMessage, EditInteractionResponse,
    EditMessage,
};
use serenity::model::application::{ButtonStyle, ComponentInteraction};
use serenity::model::id::{MessageId, UserId};
use serenity::prelude::*;

// --- CORRECTED: Removed unused `GameState` import ---
use super::state::{DuelFormat, Move};
use crate::AppState;

fn parse_id(s: &str) -> UserId {
    UserId::new(s.parse().unwrap_or(0))
}

pub async fn handle_accept(ctx: &Context, interaction: &mut ComponentInteraction, parts: &[&str]) {
    interaction.defer(&ctx.http).await.ok();

    let p2_id = parse_id(parts.get(3).unwrap_or(&""));
    if interaction.user.id != p2_id {
        let builder = CreateInteractionResponseFollowup::new()
            .content("This is not your challenge!")
            .ephemeral(true);
        let _ = interaction.create_followup(&ctx.http, builder).await;
        return;
    }

    let data = ctx.data.read().await;
    let app_state = data.get::<AppState>().unwrap();
    let mut games = app_state.active_games.write().await;
    if let Some(game) = games.get_mut(&interaction.message.id) {
        game.accepted = true;
    } else {
        let builder = CreateInteractionResponseFollowup::new()
            .content("This challenge has expired.")
            .ephemeral(true);
        let _ = interaction.create_followup(&ctx.http, builder).await;
        return;
    }

    let embed = CreateEmbed::new()
        .title("The duel has begun!")
        .description(
            "**Round 1!**\nBoth players, click the button below to make your move!".to_string(),
        )
        .color(0xFFA500);

    let buttons = CreateActionRow::Buttons(vec![
        CreateButton::new(format!("rps_prompt_{}", interaction.message.id))
            .label("Make Your Move")
            .style(ButtonStyle::Primary),
    ]);

    let builder = EditInteractionResponse::new()
        .embed(embed)
        .components(vec![buttons]);
    if let Err(e) = interaction.edit_response(&ctx.http, builder).await {
        println!("Error editing message on accept: {:?}", e);
    }
}

pub async fn handle_decline(ctx: &Context, interaction: &mut ComponentInteraction, parts: &[&str]) {
    interaction.defer(&ctx.http).await.ok();

    let p2_id = parse_id(parts.get(3).unwrap_or(&""));
    if interaction.user.id != p2_id {
        let builder = CreateInteractionResponseFollowup::new()
            .content("This is not your challenge!")
            .ephemeral(true);
        let _ = interaction.create_followup(&ctx.http, builder).await;
        return;
    }

    let embed = CreateEmbed::new()
        .title("Challenge Declined")
        .description(format!("<@{}> declined the challenge.", p2_id))
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

    let builder = EditInteractionResponse::new()
        .embed(embed)
        .components(vec![disabled_buttons]);
    if let Err(e) = interaction.edit_response(&ctx.http, builder).await {
        println!("Error editing message on decline: {:?}", e);
    }

    let data = ctx.data.read().await;
    let app_state = data.get::<AppState>().unwrap();
    app_state
        .active_games
        .write()
        .await
        .remove(&interaction.message.id);
}

pub async fn handle_prompt(ctx: &Context, interaction: &ComponentInteraction) {
    let data = ctx.data.read().await;
    let app_state = data.get::<AppState>().unwrap();
    let games = app_state.active_games.read().await;

    if let Some(game) = games.get(&interaction.message.id) {
        if interaction.user.id != game.player1.id && interaction.user.id != game.player2.id {
            let response_data = CreateInteractionResponseMessage::new()
                .content("This is not your game!")
                .ephemeral(true);
            let response = CreateInteractionResponse::Message(response_data);
            let _ = interaction.create_response(&ctx.http, response).await;
            return;
        }
    } else {
        return;
    }

    let buttons = CreateActionRow::Buttons(vec![
        CreateButton::new(format!("rps_move_rock_{}", interaction.message.id))
            .label("Rock")
            .emoji('✊')
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("rps_move_paper_{}", interaction.message.id))
            .label("Paper")
            .emoji('✋')
            .style(ButtonStyle::Secondary),
        // --- THIS IS THE FINAL FIX ---
        // The emoji is now a valid, single-codepoint `char`.
        CreateButton::new(format!("rps_move_scissors_{}", interaction.message.id))
            .label("Scissors")
            .emoji('✌')
            .style(ButtonStyle::Secondary),
    ]);

    let response_data = CreateInteractionResponseMessage::new()
        .content("Make your move!")
        .components(vec![buttons])
        .ephemeral(true);
    let response = CreateInteractionResponse::Message(response_data);
    if let Err(e) = interaction.create_response(&ctx.http, response).await {
        println!("Error sending move prompt: {:?}", e);
    }
}

pub async fn handle_move(ctx: &Context, interaction: &mut ComponentInteraction, parts: &[&str]) {
    let response_data = CreateInteractionResponseMessage::new()
        .content("Your move is locked in!")
        .components(vec![])
        .ephemeral(true);
    let response = CreateInteractionResponse::Message(response_data);
    interaction.create_response(&ctx.http, response).await.ok();

    let game_message_id = match parts.get(3).and_then(|id_str| id_str.parse::<u64>().ok()) {
        Some(id) => MessageId::new(id),
        None => return,
    };

    let move_str = parts.get(2).unwrap_or(&"");
    let player_move = match *move_str {
        "rock" => Move::Rock,
        "paper" => Move::Paper,
        "scissors" => Move::Scissors,
        _ => return,
    };

    let data = ctx.data.read().await;
    let app_state = data.get::<AppState>().unwrap();
    let mut games = app_state.active_games.write().await;
    let game = match games.get_mut(&game_message_id) {
        Some(g) => g,
        None => return,
    };

    if (interaction.user.id == game.player1.id && game.p1_move.is_some())
        || (interaction.user.id == game.player2.id && game.p2_move.is_some())
    {
        let builder = CreateInteractionResponseFollowup::new()
            .content("You have already moved for this round.")
            .ephemeral(true);
        let _ = interaction.create_followup(&ctx.http, builder).await;
        return;
    }

    if interaction.user.id == game.player1.id {
        game.p1_move = Some(player_move);
    } else if interaction.user.id == game.player2.id {
        game.p2_move = Some(player_move);
    }

    let mut public_message = match ctx
        .http
        .get_message(interaction.channel_id, game_message_id)
        .await
    {
        Ok(msg) => msg,
        Err(_) => return,
    };

    if let (Some(p1_move), Some(p2_move)) = (game.p1_move, game.p2_move) {
        let (round_winner, result_text) = match (p1_move, p2_move) {
            (u, b) if u == b => (0, "The round is a tie!".to_string()),
            (Move::Rock, Move::Scissors)
            | (Move::Paper, Move::Rock)
            | (Move::Scissors, Move::Paper) => {
                (1, format!("<@{}> wins the round!", game.player1.id))
            }
            _ => (2, format!("<@{}> wins the round!", game.player2.id)),
        };

        if round_winner == 1 {
            game.scores.0 += 1;
        } else if round_winner == 2 {
            game.scores.1 += 1;
        }

        let target_score = match game.format {
            DuelFormat::BestOf(n) => (n / 2) + 1,
            DuelFormat::RaceTo(n) => n,
        };

        let duel_over = game.scores.0 >= target_score || game.scores.1 >= target_score;

        let new_description = public_message.embeds[0]
            .description
            .clone()
            .unwrap_or_default();

        if duel_over {
            let final_winner = if game.scores.0 > game.scores.1 {
                &game.player1
            } else {
                &game.player2
            };
            let final_embed = CreateEmbed::new()
                .title("Duel Over!")
                .description(format!(
                    "{}\n\n**Result:**\n<@{}>: {}\n<@{}>: {}\n\n**<@{}> is the winner!**",
                    new_description,
                    game.player1.id,
                    game.scores.0,
                    game.player2.id,
                    game.scores.1,
                    final_winner.id
                ))
                .field(
                    format!("{}'s Move", game.player1.name),
                    p1_move.to_emoji(),
                    true,
                )
                .field(
                    format!("{}'s Move", game.player2.name),
                    p2_move.to_emoji(),
                    true,
                )
                .color(0x00FF00);

            let disabled_buttons = CreateActionRow::Buttons(vec![
                CreateButton::new("disabled_gg")
                    .label("Game Over")
                    .style(ButtonStyle::Primary)
                    .disabled(true),
            ]);
            let builder = EditMessage::new()
                .embed(final_embed)
                .components(vec![disabled_buttons]);
            public_message.edit(&ctx.http, builder).await.ok();

            games.remove(&game_message_id);
        } else if round_winner == 0 {
            // TIEBREAKER
            game.p1_move = None;
            game.p2_move = None;
            let embed = CreateEmbed::new()
                .title("It's a Tie!")
                .description(format!(
                    "**Round {} was a draw!** {} vs {}\n\n**Score:**\n<@{}>: {}\n<@{}>: {}\n\nRedo the round! Make your move.",
                    game.round, p1_move.to_emoji(), p2_move.to_emoji(), game.player1.id, game.scores.0, game.player2.id, game.scores.1
                ))
                .color(0xFFFF00);
            let builder = EditMessage::new().embed(embed);
            public_message.edit(&ctx.http, builder).await.ok();
        } else {
            // NEXT ROUND
            game.p1_move = None;
            game.p2_move = None;
            game.round += 1;
            let embed = CreateEmbed::new()
                .title(format!("Round {} Results", game.round - 1))
                .description(format!(
                    "{}\n({} vs {})\n\n**Score:**\n<@{}>: {}\n<@{}>: {}\n\nStarting **Round {}!** Make your move.",
                    result_text, p1_move.to_emoji(), p2_move.to_emoji(), game.player1.id, game.scores.0, game.player2.id, game.scores.1, game.round
                ))
                .color(0x5865F2);
            let builder = EditMessage::new().embed(embed);
            public_message.edit(&ctx.http, builder).await.ok();
        }
    } else {
        // Only one player has moved, update the public message.
        let mut new_description = public_message.embeds[0]
            .description
            .clone()
            .unwrap_or_default();
        if !new_description.contains(&format!("<@{}> has", interaction.user.id)) {
            new_description = format!(
                "{}\n<@{}> has locked in their move!",
                new_description, interaction.user.id
            );
        }
        let embed = CreateEmbed::new()
            .title("The duel is on!")
            .description(new_description)
            .color(0xFFA500);

        let builder = EditMessage::new().embed(embed);
        public_message.edit(&ctx.http, builder).await.ok();
    }
}
