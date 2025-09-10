//! Implements the `Game` trait for an interactive shop session.

use super::logic::buy_item;
use super::state::ShopSession;
use crate::commands::economy::core::item::{Item, ItemCategory};
use crate::commands::games::{Game, GameUpdate};
use serenity::async_trait;
use serenity::builder::{
    CreateActionRow, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseFollowup,
    CreateInteractionResponseMessage,
};
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;
use sqlx::PgPool;
use std::any::Any;
use std::str::FromStr;

pub struct ShopGame {
    pub session: ShopSession,
}

#[async_trait]
impl Game for ShopGame {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn render(&self) -> (String, CreateEmbed, Vec<CreateActionRow>) {
        let (embed, components) = self.session.render_shop();
        ("".to_string(), embed, components)
    }

    async fn handle_interaction(
        &mut self,
        ctx: &Context,
        interaction: &mut ComponentInteraction,
        db: &PgPool,
    ) -> GameUpdate {
        // (✓) ADDED: Security check to ensure only the session owner can interact.
        if interaction.user.id.get() != self.session.user_id {
            let response_msg = CreateInteractionResponseMessage::new()
                .content("This is not your shop session!")
                .ephemeral(true);
            let response = CreateInteractionResponse::Message(response_msg);
            interaction.create_response(&ctx.http, response).await.ok();
            return GameUpdate::NoOp;
        }

        if interaction.data.custom_id != "shop_buy" {
            interaction.defer(&ctx.http).await.ok();
        }

        match interaction.data.custom_id.as_str() {
            // (✓) ADDED: Handlers for the new category buttons.
            "shop_cat_resources" => {
                self.session.current_category = ItemCategory::Resource;
                self.session.current_page = 0;
                GameUpdate::ReRender
            }
            "shop_cat_special" => {
                self.session.current_category = ItemCategory::Special;
                self.session.current_page = 0;
                GameUpdate::ReRender
            }
            "shop_cat_consumables" => {
                self.session.current_category = ItemCategory::Consumable;
                self.session.current_page = 0;
                GameUpdate::ReRender
            }
            "shop_prev_page" => {
                if self.session.current_page > 0 {
                    self.session.current_page -= 1;
                }
                GameUpdate::ReRender
            }
            "shop_next_page" => {
                self.session.current_page += 1;
                GameUpdate::ReRender
            }
            "shop_buy" => {
                interaction.defer_ephemeral(&ctx.http).await.ok();

                let selected_item_str = if let serenity::model::application::ComponentInteractionDataKind::StringSelect { values } = &interaction.data.kind {
                    &values[0]
                } else {
                    return GameUpdate::NoOp;
                };

                let item = match Item::from_str(selected_item_str) {
                    Ok(item) => item,
                    Err(_) => return GameUpdate::NoOp,
                };

                let embed = buy_item(db, &interaction.user, item, 1).await;
                let builder = CreateInteractionResponseFollowup::new().embed(embed);
                interaction.create_followup(&ctx.http, builder).await.ok();

                // (✓) MODIFIED: The shop session no longer ends after one purchase.
                GameUpdate::NoOp
            }
            _ => GameUpdate::NoOp,
        }
    }
}
