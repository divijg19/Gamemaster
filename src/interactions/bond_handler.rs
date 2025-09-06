//! Handles bonding component interactions.

use crate::{AppState, database};
use serenity::builder::{CreateActionRow, CreateButton, EditInteractionResponse};
use crate::ui::style::pad_label;
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;
use std::sync::Arc;
use tracing::{instrument, warn};

#[instrument(level = "info", skip(ctx, component, app_state), fields(user_id = component.user.id.get(), custom_id = %component.data.custom_id))]
pub async fn handle(ctx: &Context, component: &mut ComponentInteraction, app_state: Arc<AppState>) {
    component.defer_ephemeral(&ctx.http).await.ok();
    let pool = app_state.db.clone();
    let custom_id = component.data.custom_id.clone();
    let user_id = component.user.id;

    // Expect two-phase selection: first host via bond_host, then equippable via bond_equippable
    if custom_id == "bond_open" {
        // Build host select of current party units (eligible hosts are those in party)
        if let Ok(units) = database::units::get_player_units(&pool, user_id).await {
            use serenity::builder::{
                CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
            };
            let hosts: Vec<_> = units.into_iter().filter(|u| u.is_in_party).collect();
            if hosts.is_empty() {
                component
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new()
                            .content("No party units available to host a bond."),
                    )
                    .await
                    .ok();
                return;
            }
            let host_opts = hosts
                .iter()
                .map(|u| {
                    let name = u.nickname.as_deref().unwrap_or(&u.name);
                    CreateSelectMenuOption::new(
                        format!("{} (Lvl {} {:?})", name, u.current_level, u.rarity),
                        u.player_unit_id.to_string(),
                    )
                })
                .collect();
            let host_menu = CreateSelectMenu::new(
                "bond_host:select",
                CreateSelectMenuKind::String { options: host_opts },
            )
            .placeholder("Select a host unit...");
            component
                .edit_response(
                    &ctx.http,
                    EditInteractionResponse::new()
                        .content("Select a host unit for bonding.")
                        .components(vec![CreateActionRow::SelectMenu(host_menu)]),
                )
                .await
                .ok();
            return;
        }
    }
    if custom_id.starts_with("bond_host:") {
        if let serenity::model::application::ComponentInteractionDataKind::StringSelect { values } =
            &component.data.kind
            && let Ok(host_id) = values[0].parse::<i32>()
        {
            // Fetch candidates (units not in party and not already bonded) simplistic reuse
            if let Ok(units) = database::units::get_player_units(&pool, user_id).await {
                let candidates: Vec<_> = units.into_iter().filter(|u| !u.is_in_party).collect();
                use serenity::builder::{
                    CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
                };
                let cand_opts = candidates
                    .iter()
                    .map(|u| {
                        let name = u.nickname.as_deref().unwrap_or(&u.name);
                        CreateSelectMenuOption::new(
                            format!("{} (R{:?})", name, u.rarity),
                            u.player_unit_id.to_string(),
                        )
                    })
                    .collect();
                let cand_menu = CreateSelectMenu::new(
                    format!("bond_equippable:{}", host_id),
                    CreateSelectMenuKind::String { options: cand_opts },
                )
                .placeholder("Select unit to equip...");
                component
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new()
                            .content(format!(
                                "Host selected (id {}). Now pick a unit to equip.",
                                host_id
                            ))
                            .components(vec![CreateActionRow::SelectMenu(cand_menu)]),
                    )
                    .await
                    .ok();
                return;
            }
        }
        component
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().content("Failed to load candidates."),
            )
            .await
            .ok();
        return;
    }
    if let Some(host_part) = custom_id.strip_prefix("bond_equippable:") {
        // format bond_equippable:<host_id>
        let host_id: i32 = match host_part.parse() {
            Ok(v) => v,
            Err(_) => {
                component
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new().content("Invalid host id."),
                    )
                    .await
                    .ok();
                return;
            }
        };
        let equipped_id = match &component.data.kind {
            serenity::model::application::ComponentInteractionDataKind::StringSelect { values } => {
                values[0].parse::<i32>().ok()
            }
            _ => None,
        };
        if let Some(equipped_id) = equipped_id {
            match database::units::bond_unit_as_equippable(&pool, user_id, host_id, equipped_id)
                .await
            {
                Ok(_) => {
                    // Invalidate caches for this user so next battle/party view recalculates bonuses.
                    app_state.invalidate_user_caches(user_id).await;
                    // Provide Unequip button (not yet wired to DB toggle) placeholder.
                    let unequip_button =
                        CreateButton::new(format!("bond_unequip:{}", host_id)).label(pad_label("🗑 Unequip", 14));
                    component
                        .edit_response(
                            &ctx.http,
                            EditInteractionResponse::new()
                                .content("Bond created successfully.")
                                .components(vec![CreateActionRow::Buttons(vec![unequip_button])]),
                        )
                        .await
                        .ok();
                }
                Err(e) => {
                    warn!(error = %e, "bond_failed");
                    component
                        .edit_response(
                            &ctx.http,
                            EditInteractionResponse::new()
                                .content(format!("Failed to bond: {}", e)),
                        )
                        .await
                        .ok();
                }
            }
        } else {
            component
                .edit_response(
                    &ctx.http,
                    EditInteractionResponse::new().content("No unit selected to equip."),
                )
                .await
                .ok();
        }
        return;
    }
    if let Some(host_part) = custom_id.strip_prefix("bond_unequip:")
        && let Ok(host_id) = host_part.parse::<i32>()
    {
        match database::units::unequip_equippable(&pool, user_id, host_id).await {
            Ok(true) => {
                app_state.invalidate_user_caches(user_id).await;
                component
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new().content("Unit unequipped."),
                    )
                    .await
                    .ok();
            }
            Ok(false) => {
                component
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new()
                            .content("No equipped unit found for that host."),
                    )
                    .await
                    .ok();
            }
            Err(e) => {
                warn!(error = %e, "unequip_failed");
                component
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new().content(format!("Error: {}", e)),
                    )
                    .await
                    .ok();
            }
        }
    }
}
