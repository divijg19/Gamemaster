use crate::database;
use serenity::builder::{CreateActionRow, CreateCommand, CreateEmbed};
use crate::ui::buttons::Btn;
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::Context;

pub fn register() -> CreateCommand {
    CreateCommand::new("research").description("View pet research / taming progress")
}

pub async fn build_view_cached(
    app_state: &crate::AppState,
    user: serenity::model::id::UserId,
) -> CreateEmbed {
    // Use underlying db reference but leverage cached progress list
    let db = &app_state.db;
    let mut embed = CreateEmbed::new().title("Research Progress").description(
        "Taming progress for sub-Legendary pets. Defeat creatures to gain research drops.",
    );
    if let Ok(units) = database::units::get_all_units(db).await
        && let Ok(progress) = database::units::list_research_progress_cached(app_state, user).await
    {
        use std::collections::HashMap;
        let map: HashMap<i32, i32> = progress.into_iter().collect();
        for u in units
            .into_iter()
            .filter(|u| matches!(u.kind, database::models::UnitKind::Pet))
            .take(25)
        {
            let count = map.get(&u.unit_id).cloned().unwrap_or(0);
            let target = database::units::research_target_for_rarity(db, u.rarity).await;
            let field_title = format!("{} ({:?})", u.name, u.rarity);
            if target == 0 {
                embed = embed.field(
                    field_title,
                    "Party Eligible (no research required)".to_string(),
                    true,
                );
            } else {
                let pct = (count as f32 / target as f32).min(1.0);
                let filled = (pct * 10.0).round() as i32;
                let mut bar = String::with_capacity(10);
                for i in 0..10 {
                    if i < filled {
                        bar.push('#');
                    } else {
                        bar.push('-');
                    }
                }
                let status = if count >= target { "Ready" } else { "Progress" };
                embed = embed.field(
                    field_title,
                    format!("[{bar}] {count}/{target} {status}"),
                    true,
                );
            }
        }
    }
    embed
}

pub async fn run_slash(ctx: &Context, interaction: &mut CommandInteraction) {
    interaction.defer_ephemeral(&ctx.http).await.ok();
    let Some(state) = crate::AppState::from_ctx(ctx).await else {
        return;
    };
    let embed = build_view_cached(&state, interaction.user.id).await;
    let row = CreateActionRow::Buttons(vec![
        Btn::secondary("bestiary_refresh", "📚 Bestiary"),
        Btn::secondary("contracts_refresh", "📜 Contracts"),
    ]);
    let resp = serenity::builder::CreateInteractionResponseMessage::new()
        .embed(embed)
        .components(vec![row]);
    let _ = interaction
        .create_response(
            &ctx.http,
            serenity::builder::CreateInteractionResponse::Message(resp),
        )
        .await;
}

pub async fn run_prefix(ctx: &Context, msg: &Message, _args: Vec<&str>) {
    let Some(state) = crate::AppState::from_ctx(ctx).await else {
        return;
    };
    let embed = build_view_cached(&state, msg.author.id).await;
    let row = CreateActionRow::Buttons(vec![
        Btn::secondary("bestiary_refresh", "📚 Bestiary"),
        Btn::secondary("contracts_refresh", "📜 Contracts"),
    ]);
    let _ = msg
        .channel_id
        .send_message(
            &ctx.http,
            serenity::builder::CreateMessage::new()
                .embed(embed)
                .components(vec![row]),
        )
        .await;
}
