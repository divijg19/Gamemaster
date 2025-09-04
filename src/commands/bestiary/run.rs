use crate::AppState;
use crate::commands::bestiary::ui::{BestiaryEntry, create_bestiary_embed};
use crate::database;
use serenity::builder::{
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::*;

pub fn register() -> CreateCommand {
    CreateCommand::new("bestiary").description("View discovered creatures")
}

async fn gather_entries(
    db: &sqlx::PgPool,
    user: serenity::model::id::UserId,
) -> Vec<BestiaryEntry> {
    // Simple heuristic: list Pet units plus defeat counts & research inventory if any.
    // Defeats tracked only for humans now; for pets we show research item counts.
    if let Ok(units) = database::units::get_all_units(db).await {
        let mut out = Vec::new();
        for u in units
            .into_iter()
            .filter(|u| matches!(u.kind, database::models::UnitKind::Pet))
        {
            // Count research item in inventory
            let research_item =
                crate::commands::economy::core::item::Item::research_item_for_unit(&u.name);
            let research_owned = if let Some(item) = research_item {
                // Use existing helper get_inventory_item (returns struct with quantity) via a temp tx for read.
                // Simple read-only query without an explicit transaction to avoid ownership issues
                database::economy::get_inventory_item_simple(db, user, item)
                    .await
                    .ok()
                    .flatten()
                    .map(|i| i.quantity)
                    .unwrap_or(0)
            } else {
                0
            };
            out.push(BestiaryEntry {
                unit: u,
                defeated: 0,
                research_owned,
            });
        }
        out
    } else {
        Vec::new()
    }
}

pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new()),
        )
        .await
        .ok();
    let Some(app_state) = AppState::from_ctx(ctx).await else {
        return;
    };
    let db = &app_state.db;
    let entries = gather_entries(db, interaction.user.id).await;
    let (embed, components) = create_bestiary_embed(&entries);
    interaction
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new()
                .embed(embed)
                .components(components),
        )
        .await
        .ok();
}

pub async fn run_prefix(ctx: &Context, msg: &Message, _args: Vec<&str>) {
    let Some(app_state) = AppState::from_ctx(ctx).await else {
        return;
    };
    let db = &app_state.db;
    let entries = gather_entries(db, msg.author.id).await;
    let (embed, components) = create_bestiary_embed(&entries);
    msg.channel_id
        .send_message(
            &ctx.http,
            serenity::builder::CreateMessage::new()
                .embed(embed)
                .components(components)
                .reference_message(msg),
        )
        .await
        .ok();
}
