//! Handles all `ComponentInteraction` events for the Blackjack game.

use super::state::{BlackjackGame, GamePhase, Hand, HandStatus, Player};
use crate::commands::games::card::Rank;
use crate::commands::games::engine::GameUpdate; // (✓) FIXED: GameUpdate is from the engine module
use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;
use sqlx::PgPool;
use std::sync::Arc;

// (✓) FIXED: This function is now more robust. It handles users not in the DB
// and also correctly handles the non-optional `balance` field.
async fn get_balance(db: &PgPool, user_id: serenity::model::id::UserId) -> i64 {
    match sqlx::query!(
        "SELECT balance FROM profiles WHERE user_id = $1",
        user_id.get() as i64
    )
    .fetch_optional(db) // Use `fetch_optional` to gracefully handle users not in the DB.
    .await
    {
        Ok(Some(record)) => record.balance, // The balance field is NOT NULL, so it's a direct i64.
        _ => 0,                             // If no user or DB error, assume 0 balance.
    }
}

// Moved here from game.rs to solve potential borrow checker errors.
async fn send_ephemeral_response(ctx: &Context, interaction: &ComponentInteraction, content: &str) {
    let builder = CreateInteractionResponseMessage::new()
        .content(content)
        .ephemeral(true);
    let response = CreateInteractionResponse::Message(builder);
    interaction.create_response(&ctx.http, response).await.ok();
}

impl BlackjackGame {
    pub(super) async fn handle_lobby(
        &mut self,
        ctx: &Context,
        interaction: &mut ComponentInteraction,
        db: &PgPool,
    ) -> GameUpdate {
        match interaction.data.custom_id.as_str() {
            "bj_join" => {
                if !self
                    .players
                    .iter()
                    .any(|p| p.user.id == interaction.user.id)
                {
                    if self.min_bet > 0 {
                        let balance = get_balance(db, interaction.user.id).await;
                        if balance < self.min_bet {
                            send_ephemeral_response(
                                ctx,
                                interaction,
                                &format!(
                                    "You cannot afford the minimum bet of **💰{}** to join.",
                                    self.min_bet
                                ),
                            )
                            .await;
                            return GameUpdate::NoOp;
                        }
                    }
                    self.players.push(Player {
                        user: Arc::new(interaction.user.clone()),
                        hands: Vec::new(),
                        insurance: 0,
                        current_bet: self.min_bet,
                        insurance_decision_made: false,
                    });
                    interaction.defer(&ctx.http).await.ok();
                    GameUpdate::ReRender
                } else {
                    send_ephemeral_response(ctx, interaction, "You have already joined.").await;
                    GameUpdate::NoOp
                }
            }
            "bj_start" => {
                if interaction.user.id.get() == self.host_id {
                    self.start_game();
                    interaction.defer(&ctx.http).await.ok();
                    GameUpdate::ReRender
                } else {
                    send_ephemeral_response(ctx, interaction, "Only the host can start.").await;
                    GameUpdate::NoOp
                }
            }
            _ => GameUpdate::NoOp,
        }
    }

    pub(super) async fn handle_betting(
        &mut self,
        ctx: &Context,
        interaction: &mut ComponentInteraction,
        db: &PgPool,
    ) -> GameUpdate {
        let player = match self
            .players
            .iter_mut()
            .find(|p| p.user.id == interaction.user.id)
        {
            Some(p) => p,
            None => {
                send_ephemeral_response(ctx, interaction, "You are not in this game.").await;
                return GameUpdate::NoOp;
            }
        };
        if self.ready_players.contains(&player.user.id) {
            send_ephemeral_response(ctx, interaction, "You have already confirmed your bet.").await;
            return GameUpdate::NoOp;
        }
        let balance = get_balance(db, player.user.id).await;

        match interaction.data.custom_id.as_str() {
            "bj_bet_10" => player.current_bet = (player.current_bet + 10).min(balance),
            "bj_bet_100" => player.current_bet = (player.current_bet + 100).min(balance),
            "bj_bet_1000" => player.current_bet = (player.current_bet + 1000).min(balance),
            "bj_bet_all_in" => player.current_bet = balance,
            "bj_bet_clear" => player.current_bet = self.min_bet.min(balance),
            "bj_bet_confirm" => {
                if player.current_bet > balance {
                    send_ephemeral_response(ctx, interaction, "You cannot bet more than you have.")
                        .await;
                    player.current_bet = balance;
                    return GameUpdate::ReRender;
                }
                if player.current_bet < self.min_bet {
                    send_ephemeral_response(
                        ctx,
                        interaction,
                        &format!(
                            "Your bet must be at least the table minimum of 💰{}.",
                            self.min_bet
                        ),
                    )
                    .await;
                    return GameUpdate::NoOp;
                }
                self.ready_players.insert(interaction.user.id);
            }
            _ => return GameUpdate::NoOp,
        }

        interaction.defer(&ctx.http).await.ok();
        if self.ready_players.len() == self.players.len() {
            self.deal_new_round();
        }

        GameUpdate::ReRender
    }

    pub(super) async fn handle_insurance(
        &mut self,
        ctx: &Context,
        interaction: &mut ComponentInteraction,
        db: &PgPool,
    ) -> GameUpdate {
        let player = match self
            .players
            .iter_mut()
            .find(|p| p.user.id == interaction.user.id)
        {
            Some(p) => p,
            None => return GameUpdate::NoOp,
        };
        if player.insurance_decision_made {
            send_ephemeral_response(
                ctx,
                interaction,
                "You have already made your insurance decision.",
            )
            .await;
            return GameUpdate::NoOp;
        }

        match interaction.data.custom_id.as_str() {
            "bj_insure_yes" => {
                let insurance_cost = player.hands[0].bet / 2;
                let balance = get_balance(db, interaction.user.id).await;
                if balance < player.hands[0].bet + insurance_cost {
                    send_ephemeral_response(
                        ctx,
                        interaction,
                        &format!(
                            "You cannot afford the insurance cost of **💰{}**.",
                            insurance_cost
                        ),
                    )
                    .await;
                    return GameUpdate::NoOp;
                }
                player.insurance = insurance_cost;
                player.insurance_decision_made = true;
            }
            "bj_insure_no" => {
                player.insurance = 0;
                player.insurance_decision_made = true;
            }
            _ => return GameUpdate::NoOp,
        };

        interaction.defer(&ctx.http).await.ok();
        let all_decided = self
            .players
            .iter()
            .all(|p| p.insurance_decision_made || p.hands[0].status == HandStatus::Blackjack);
        if all_decided {
            if self.dealer_hand.score() == 21 && self.dealer_hand.cards.len() == 2 {
                self.phase = GamePhase::GameOver;
            } else {
                self.phase = GamePhase::PlayerTurns;
                self.find_next_hand();
            }
        }
        if self.phase == GamePhase::GameOver {
            let (message, payouts) = self.calculate_payouts();
            GameUpdate::GameOver { message, payouts }
        } else {
            GameUpdate::ReRender
        }
    }

    pub(super) async fn handle_player_turn(
        &mut self,
        ctx: &Context,
        interaction: &mut ComponentInteraction,
        db: &PgPool,
    ) -> GameUpdate {
        if interaction.user.id != self.players[self.current_player_index].user.id {
            send_ephemeral_response(ctx, interaction, "It's not your turn.").await;
            return GameUpdate::NoOp;
        }
        interaction.defer(&ctx.http).await.ok();

        let balance = get_balance(db, interaction.user.id).await;
        let player = &mut self.players[self.current_player_index];
        let total_bet_so_far: i64 = player.hands.iter().map(|h| h.bet).sum();

        match interaction.data.custom_id.as_str() {
            "bj_hit" => {
                let hand = &mut player.hands[self.current_hand_index];
                if let Some(card) = self.deck.deal_one() {
                    hand.add_card(card);
                }
                if hand.score() >= 21 {
                    hand.status = if hand.score() > 21 {
                        HandStatus::Busted
                    } else {
                        HandStatus::Stood
                    };
                    self.advance_turn();
                }
            }
            "bj_stand" => {
                player.hands[self.current_hand_index].status = HandStatus::Stood;
                self.advance_turn();
            }
            "bj_double" => {
                let hand = &mut player.hands[self.current_hand_index];
                if hand.can_double_down() {
                    if balance < total_bet_so_far + hand.bet {
                        send_ephemeral_response(
                            ctx,
                            interaction,
                            "You cannot afford to double your bet.",
                        )
                        .await;
                        return GameUpdate::NoOp;
                    }
                    self.pot += hand.bet;
                    hand.bet *= 2;
                    if let Some(card) = self.deck.deal_one() {
                        hand.add_card(card);
                    }
                    hand.status = if hand.score() > 21 {
                        HandStatus::Busted
                    } else {
                        HandStatus::Stood
                    };
                    self.advance_turn();
                }
            }
            "bj_split" => {
                if player.hands[self.current_hand_index].can_split() {
                    if balance < total_bet_so_far + self.min_bet {
                        send_ephemeral_response(
                            ctx,
                            interaction,
                            "You cannot afford to place a bet for a new hand.",
                        )
                        .await;
                        return GameUpdate::NoOp;
                    }
                    let hand = &mut player.hands[self.current_hand_index];
                    if let Some(split_card) = hand.cards.pop() {
                        let mut new_hand = Hand::new(self.min_bet);
                        new_hand.add_card(split_card);
                        if let Some(card) = self.deck.deal_one() {
                            hand.add_card(card);
                        }
                        if let Some(card) = self.deck.deal_one() {
                            new_hand.add_card(card);
                        }
                        self.pot += new_hand.bet;
                        if hand.cards[0].rank == Rank::Ace {
                            hand.status = HandStatus::Stood;
                            new_hand.status = HandStatus::Stood;
                        }
                        if hand.score() == 21 {
                            hand.status = HandStatus::Stood;
                        }
                        if new_hand.score() == 21 {
                            new_hand.status = HandStatus::Stood;
                        }
                        player.hands.insert(self.current_hand_index + 1, new_hand);
                        if player.hands[self.current_hand_index].status == HandStatus::Stood
                            // (✓) FIXED: Use `is_some_and` for a cleaner check.
                            && player.hands.get(self.current_hand_index + 1).is_some_and(|h| h.status == HandStatus::Stood)
                        {
                            self.advance_turn();
                        }
                    }
                }
            }
            "bj_surrender" => {
                let hand = &mut player.hands[self.current_hand_index];
                if hand.can_surrender() {
                    hand.status = HandStatus::Surrendered;
                    self.advance_turn();
                }
            }
            _ => return GameUpdate::NoOp,
        }
        if self.phase == GamePhase::GameOver {
            let (message, payouts) = self.calculate_payouts();
            GameUpdate::GameOver { message, payouts }
        } else {
            GameUpdate::ReRender
        }
    }

    pub(super) async fn handle_game_over(
        &mut self,
        ctx: &Context,
        interaction: &mut ComponentInteraction,
    ) -> GameUpdate {
        if interaction.data.custom_id == "bj_next_round"
            && interaction.user.id.get() == self.host_id
        {
            interaction.defer(&ctx.http).await.ok();
            self.reset_for_next_round();
            GameUpdate::ReRender
        } else {
            send_ephemeral_response(ctx, interaction, "Only the host can start the next round.")
                .await;
            GameUpdate::NoOp
        }
    }
}
