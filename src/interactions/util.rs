//! Shared interaction utility helpers (single defer + safe edit wrapper).
use crate::ui::ContextBag;
use serenity::builder::EditInteractionResponse;
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;

/// Acknowledge a component interaction (non-ephemeral) ignoring duplicate/late errors.
pub async fn defer_component(ctx: &Context, c: &ComponentInteraction) {
    if let Err(e) = c.defer(&ctx.http).await {
        tracing::debug!(target="ui.defer", cid=%c.data.custom_id, error=?e, "defer failed (already acknowledged?)");
    }
}

/// Edit original interaction response; logs failure with a tag for observability.
pub async fn edit_component(
    ctx: &Context,
    c: &ComponentInteraction,
    tag: &str,
    builder: EditInteractionResponse,
) {
    if let Err(e) = c.edit_response(&ctx.http, builder).await {
        tracing::error!(target="ui.edit", cid=%c.data.custom_id, tag=%tag, error=?e, "edit_response failed");
    }
}

/// Handle a nav_* button globally (saga/party/train). Returns true if handled.
pub async fn handle_global_nav(
    ctx: &Context,
    c: &ComponentInteraction,
    app_state: &std::sync::Arc<crate::AppState>,
    _current: &str,
) -> bool {
    let cid = c.data.custom_id.as_str();
    match cid {
        "nav_saga" => {
            // Use unified SagaView root rendering (no stack push since Root variant ignored in push).
            match crate::saga::view::push_and_render(
                crate::saga::view::SagaView::Root,
                app_state,
                c.user.id,
                0,
            )
            .await
            {
                Ok((embed, components)) => {
                    edit_component(
                        ctx,
                        c,
                        "nav_saga",
                        EditInteractionResponse::new()
                            .embed(embed)
                            .components(components),
                    )
                    .await;
                }
                Err(e) => {
                    edit_component(
                        ctx,
                        c,
                        "nav_saga.err",
                        EditInteractionResponse::new()
                            .content(format!("Failed to load saga root: {e}")),
                    )
                    .await;
                }
            }
            true
        }
        "nav_party" => {
            let (embed, components) =
                crate::commands::party::ui::create_party_view_with_bonds(app_state, c.user.id)
                    .await;
            edit_component(
                ctx,
                c,
                "nav_party",
                EditInteractionResponse::new()
                    .embed(embed)
                    .components(components),
            )
            .await;
            true
        }
        "nav_train" => {
            if let (Ok(units), Some(profile)) = (
                crate::database::units::get_player_units(&app_state.db, c.user.id).await,
                crate::services::saga::get_saga_profile(app_state, c.user.id, false).await,
            ) {
                let (embed, components) =
                    crate::commands::train::ui::create_training_menu(&units, &profile);
                edit_component(
                    ctx,
                    c,
                    "nav_train",
                    EditInteractionResponse::new()
                        .embed(embed)
                        .components(components),
                )
                .await;
            }
            true
        }
        _ => false,
    }
}

/// Centralized saga back/refresh handling. Returns true if handled.
pub async fn handle_saga_back_refresh(
    ctx: &Context,
    c: &ComponentInteraction,
    app_state: &crate::AppState,
) -> bool {
    use crate::commands::saga::ui::back_refresh_row;
    use serenity::builder::EditInteractionResponse as EIR;
    if c.data.custom_id == "saga_back" {
        {
            let mut stacks = app_state.nav_stacks.write().await;
            if let Some(s) = stacks.get_mut(&c.user.id.get())
                && let Some(old) = s.pop()
            {
                tracing::debug!(target="nav", user_id=%c.user.id.get(), state=old.id(), action="pop", depth=s.stack.len());
            }
        }
        let has_party = crate::database::units::get_user_party(&app_state.db, c.user.id)
            .await
            .map(|p| !p.is_empty())
            .unwrap_or(false);
        let saga_profile =
            crate::services::saga::get_saga_profile(app_state, c.user.id, false).await;
        if let Some(nav_box) = app_state
            .nav_stacks
            .write()
            .await
            .get_mut(&c.user.id.get())
            .and_then(|s| s.stack.last_mut())
        {
            // Attempt stale refresh for saga views.
            if let Some(saga_nav) = nav_box
                .as_any_mut()
                .downcast_mut::<crate::saga::view::SagaNavState>()
            {
                saga_nav.refresh_if_stale(app_state, c.user.id).await;
            }
            let ctxbag = ContextBag::new(app_state.db.clone(), c.user.id);
            // Touch dynamic type & context fields to keep trait surface and struct fields alive.
            let _ = nav_box.as_any().type_id();
            let _ = (&ctxbag.db, ctxbag.user_id);
            let (embed, components) = nav_box.render(&ctxbag).await;
            edit_component(
                ctx,
                c,
                "back.stack",
                EIR::new().embed(embed).components(components),
            )
            .await;
        } else if let Some(profile) = saga_profile {
            let (embed, components) =
                crate::commands::saga::ui::create_saga_menu(&profile, has_party);
            edit_component(
                ctx,
                c,
                "back.root",
                EIR::new().embed(embed).components(components),
            )
            .await;
        }
        return true;
    }
    if c.data.custom_id == "saga_refresh" {
        let _ = crate::services::saga::get_saga_profile(app_state, c.user.id, true).await;
        if let Some(nav_box) = app_state
            .nav_stacks
            .write()
            .await
            .get_mut(&c.user.id.get())
            .and_then(|s| s.stack.last_mut())
        {
            if let Some(saga_nav) = nav_box
                .as_any_mut()
                .downcast_mut::<crate::saga::view::SagaNavState>()
            {
                saga_nav.refresh_if_stale(app_state, c.user.id).await;
            }
            let ctxbag = ContextBag::new(app_state.db.clone(), c.user.id);
            let _ = nav_box.as_any().type_id();
            let _ = (&ctxbag.db, ctxbag.user_id);
            let (embed, mut components) = nav_box.render(&ctxbag).await;
            let depth = app_state
                .nav_stacks
                .read()
                .await
                .get(&c.user.id.get())
                .map(|s| s.stack.len())
                .unwrap_or(1);
            if let Some(row) = back_refresh_row(depth) {
                components.push(row);
            }
            edit_component(
                ctx,
                c,
                "refresh",
                EIR::new().embed(embed).components(components),
            )
            .await;
        }
        return true;
    }
    false
}
