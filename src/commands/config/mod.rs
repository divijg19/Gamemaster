use crate::database::settings::set_config_value;
use crate::model::AppState;
use serenity::builder::{
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
    EditInteractionResponse,
};
use serenity::model::application::{CommandDataOptionValue, CommandInteraction, CommandOptionType};
use serenity::prelude::*;
use tracing::warn;

// FUTURE: Move admin id / role id to secrets or DB config table.
// Updated admin user id per user request.
fn is_admin(user_id: u64) -> bool {
    matches!(user_id, 637126486423371778)
}

pub fn register() -> CreateCommand {
    CreateCommand::new("config")
        .description("View or update bot configuration (admin only).")
        .add_option(
            serenity::builder::CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "starter_unit",
                "Set the tutorial starter unit id",
            )
            .add_sub_option(
                serenity::builder::CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "id",
                    "Unit ID to set",
                )
                .required(true),
            ),
        )
        .add_option(serenity::builder::CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "show",
            "Show current config",
        ))
        .add_option(serenity::builder::CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "bonds",
            "List active bonds for this user (admin)",
        ))
}

pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
        .ok();
    let data_read = ctx.data.read().await;
    let Some(app_state) = data_read.get::<AppState>().cloned() else {
        warn!(command = "config", "missing_app_state");
        interaction
            .create_response(
                &ctx.http,
                serenity::builder::CreateInteractionResponse::Message(
                    serenity::builder::CreateInteractionResponseMessage::new()
                        .content("Internal error: missing app state")
                        .ephemeral(true),
                ),
            )
            .await
            .ok();
        return;
    };
    let user_id = interaction.user.id.get();
    if let Some(sub) = interaction.data.options.first() {
        match &sub.value {
            CommandDataOptionValue::SubCommand(nested) => {
                match sub.name.as_str() {
                    "starter_unit" => {
                        if !is_admin(user_id) {
                            interaction
                                .edit_response(
                                    &ctx.http,
                                    EditInteractionResponse::new()
                                        .content("You are not permitted to set config."),
                                )
                                .await
                                .ok();
                            return;
                        }
                        if let Some(first) = nested.first()
                            && let CommandDataOptionValue::Integer(val) = first.value
                        {
                            *app_state.starter_unit_id.write().await = val as i32;
                            if let Err(e) =
                                set_config_value(&app_state.db, "starter_unit_id", &val.to_string())
                                    .await
                            {
                                interaction.edit_response(&ctx.http, EditInteractionResponse::new().content(format!("Starter unit updated in memory but failed to persist: {}", e))).await.ok();
                            } else {
                                interaction
                                    .edit_response(
                                        &ctx.http,
                                        EditInteractionResponse::new()
                                            .content(format!("Starter unit id set to {}", val)),
                                    )
                                    .await
                                    .ok();
                            }
                            return;
                        }
                        interaction
                            .edit_response(
                                &ctx.http,
                                EditInteractionResponse::new().content("Invalid unit id."),
                            )
                            .await
                            .ok();
                    }
                    "show" => {
                        let starter = *app_state.starter_unit_id.read().await;
                        interaction
                            .edit_response(
                                &ctx.http,
                                EditInteractionResponse::new()
                                    .content(format!("Current starter_unit_id: {}", starter)),
                            )
                            .await
                            .ok();
                    }
                    "bonds" => {
                        if !is_admin(user_id) {
                            interaction
                                .edit_response(
                                    &ctx.http,
                                    EditInteractionResponse::new().content("Not permitted."),
                                )
                                .await
                                .ok();
                            return;
                        }
                        match crate::database::units::list_active_bonds_detailed(
                            &app_state.db,
                            interaction.user.id,
                        )
                        .await
                        {
                            Ok(rows) if !rows.is_empty() => {
                                use chrono::Utc;
                                let mut out = String::from("Active Bonds:\n");
                                for r in rows {
                                    let age = Utc::now() - r.created_at;
                                    let mins = age.num_minutes();
                                    out.push_str(&format!(
                                        "[#{}] Host {} <- Equipped {} • {}m • equipped:{}\n",
                                        r.bond_id,
                                        r.host_player_unit_id,
                                        r.equipped_player_unit_id,
                                        mins,
                                        r.is_equipped
                                    ));
                                }
                                interaction
                                    .edit_response(
                                        &ctx.http,
                                        EditInteractionResponse::new().content(out),
                                    )
                                    .await
                                    .ok();
                            }
                            Ok(_) => {
                                interaction
                                    .edit_response(
                                        &ctx.http,
                                        EditInteractionResponse::new().content("No active bonds."),
                                    )
                                    .await
                                    .ok();
                            }
                            Err(_) => {
                                interaction
                                    .edit_response(
                                        &ctx.http,
                                        EditInteractionResponse::new()
                                            .content("Failed to load bonds."),
                                    )
                                    .await
                                    .ok();
                            }
                        }
                    }
                    _ => {
                        interaction
                            .edit_response(
                                &ctx.http,
                                EditInteractionResponse::new()
                                    .content("Unknown config subcommand."),
                            )
                            .await
                            .ok();
                    }
                }
            }
            _ => {
                interaction
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new().content("Malformed config command."),
                    )
                    .await
                    .ok();
            }
        }
    } else {
        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().content("No subcommand provided."),
            )
            .await
            .ok();
    }
}
