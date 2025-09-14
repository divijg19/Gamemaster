//! Handles all component interactions for the `saga` command family.

use crate::commands::games::Game;
use crate::saga::battle::game::BattleGame;
// (✓) FIXED: Import the specific structs needed, removing the unused `BattlePhase`.
use super::util::{defer_component, edit_component, handle_global_nav, handle_saga_back_refresh};
use crate::constants::EQUIP_BONUS_CACHE_TTL_SECS;
use crate::saga::battle::state::{BattleSession, BattleUnit};
use crate::saga::view::{SagaView, push_and_render};
use crate::services::cache as cache_service;
use crate::ui::style::error_embed;
// NavState no longer needed directly after SagaView migration
use crate::{AppState, database};
use chrono::Datelike;
use serenity::builder::EditInteractionResponse;
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;
use std::sync::Arc;
use std::time::Duration;
use tracing::instrument;

// (Removed local edit helper; using util::edit_component for consistency.)
// Small helper: render the Tavern Games main menu consistently.
#[instrument(name = "ui.tavern.games", skip_all)]
async fn render_tavern_games_view(
    ctx: &Context,
    component: &mut ComponentInteraction,
    _app_state: &Arc<AppState>,
) {
    use serenity::builder::{CreateActionRow, CreateEmbed};
    let embed = CreateEmbed::new()
        .title("Tavern Games")
        .description("Challenge the house or friends. Card games here are friendly — no ante.")
        .field("Blackjack", "Play vs the dealer.", true)
        .field("Poker", "Five Card Draw.", true)
        .field(
            "Arm Wrestling",
            "Pick a party member. Tests Strength.",
            true,
        )
        .field("Darts", "Pick a party member. Tests Dexterity.", true)
        .color(crate::ui::style::COLOR_SAGA_TAVERN);
    let mut rows = Vec::with_capacity(4);
    // Row 1: Card games
    rows.push(CreateActionRow::Buttons(vec![
        crate::ui::buttons::Btn::primary(
            crate::interactions::ids::SAGA_TAVERN_GAMES_BLACKJACK,
            "🃏 Blackjack",
        ),
        crate::ui::buttons::Btn::primary(
            crate::interactions::ids::SAGA_TAVERN_GAMES_POKER,
            "🂡 Poker",
        ),
    ]));
    // Row 2: Stat games
    rows.push(CreateActionRow::Buttons(vec![
        crate::ui::buttons::Btn::secondary(
            crate::interactions::ids::SAGA_TAVERN_GAMES_ARM,
            "💪 Arm Wrestling",
        ),
        crate::ui::buttons::Btn::secondary(
            crate::interactions::ids::SAGA_TAVERN_GAMES_DARTS,
            "🎯 Darts",
        ),
    ]));
    // Row 3: Home/Recruitment
    let buttons = vec![
        crate::ui::buttons::Btn::secondary(crate::interactions::ids::SAGA_TAVERN_HOME, "🏰 Tavern"),
        crate::ui::buttons::Btn::primary(crate::interactions::ids::SAGA_RECRUIT, "🧭 Recruitment"),
    ];
    rows.push(CreateActionRow::Buttons(buttons));
    // Row 4: Global nav (↩ Saga + Refresh)
    rows.push(crate::commands::saga::ui::tavern_saga_row());
    edit_component(
        ctx,
        component,
        "tavern.games",
        EditInteractionResponse::new().embed(embed).components(rows),
    )
    .await;
}

// Local cache helpers removed (centralized in services::saga).

// Centralized Tavern pricing for Goods and Small Arms menus.
fn tavern_price(item: crate::commands::economy::core::item::Item) -> Option<i64> {
    use crate::commands::economy::core::item::Item as I;
    match item {
        I::HealthPotion => Some(50),
        I::FocusTonic => Some(125),
        I::StaminaDraft => Some(125),
        I::TamingLure => Some(200),
        I::GreaterHealthPotion => Some(150),
        I::XpBooster => item.properties().buy_price.or(Some(2000)),
        I::ForestContractParchment => Some(300),
        I::FrontierContractParchment => Some(500),
        _ => None,
    }
}

// Small helper: render the Tavern Goods view consistently, with optional notice line.
async fn render_tavern_goods_view(
    ctx: &Context,
    component: &mut ComponentInteraction,
    app_state: &Arc<AppState>,
    notice: Option<String>,
) {
    use crate::commands::economy::core::item::Item;
    use serenity::builder::{CreateActionRow, CreateEmbed};
    let db = &app_state.db;
    let catalog: Vec<Item> = vec![
        Item::HealthPotion,
        Item::FocusTonic,
        Item::StaminaDraft,
        Item::TamingLure,
    ];
    // Focus status for banner/disable
    let focus_state = crate::services::cache::get_with_ttl_and_remaining(
        &app_state.focus_buff_cache,
        &component.user.id.get(),
        std::time::Duration::from_secs(crate::constants::FOCUS_TONIC_TTL_SECS),
    )
    .await;
    // Profile + discount
    let profile = crate::database::economy::get_or_create_profile(db, component.user.id)
        .await
        .unwrap_or(crate::database::models::Profile {
            balance: 0,
            last_work: None,
            work_streak: 0,
            fishing_xp: 0,
            fishing_level: 1,
            mining_xp: 0,
            mining_level: 1,
            coding_xp: 0,
            coding_level: 1,
        });
    let (_, meta_tmp) =
        crate::commands::saga::tavern::build_tavern_state_cached(app_state, component.user.id)
            .await
            .unwrap_or((
                Vec::new(),
                crate::commands::saga::tavern::TavernUiMeta {
                    balance: profile.balance,
                    fame: 0,
                    fame_tier: 0,
                    fame_progress: 0.0,
                    daily_rerolls_used: 0,
                    max_daily_rerolls: crate::commands::saga::tavern::TAVERN_MAX_DAILY_REROLLS,
                    reroll_cost: crate::commands::saga::tavern::TAVERN_REROLL_COST,
                    can_reroll: false,
                },
            ));
    let (shop_disc, _, _) = crate::commands::saga::tavern::fame_perks(meta_tmp.fame_tier);
    let mut embed = CreateEmbed::new()
        .title("Tavern Goods")
        .description({
            let mut s = format!(
                "Buy consumables and aids.{}",
                if shop_disc > 0.0 {
                    format!(" Fame discount: -{}%", (shop_disc * 100.0) as i32)
                } else {
                    String::new()
                }
            );
            if let Some((active, remaining)) = &focus_state
                && *active
            {
                let mins = remaining.as_secs() / 60;
                let secs = remaining.as_secs() % 60;
                s.push_str(&format!(
                    "\n🧠 Focus active: {:02}:{:02} remaining.",
                    mins, secs
                ));
            }
            if let Some(msg) = &notice {
                s.push_str(&format!("\nℹ️ {}", msg));
            }
            s.push_str("\nDisabled buttons indicate you don’t meet the requirement (e.g., insufficient coins or an effect already active).");
            s
        })
        .field(
            "Balance",
            format!("{} {}", crate::ui::style::EMOJI_COIN, profile.balance),
            true,
        )
        .color(crate::ui::style::COLOR_SAGA_TAVERN);
    let mut buy_buttons = Vec::new();
    for item in &catalog {
        let base = tavern_price(*item).unwrap_or(1_000_000);
        let cost = crate::commands::saga::tavern::apply_shop_discount(base, shop_disc);
        let label = format!("{} {}", item.emoji(), item.display_name());
        let desc = item.properties().description;
        embed = embed.field(
            label,
            format!("{}\nCost: {} {}", desc, crate::ui::style::EMOJI_COIN, cost),
            true,
        );
        buy_buttons.push(
            crate::ui::buttons::Btn::success(
                &format!(
                    "{}{}",
                    crate::interactions::ids::SAGA_TAVERN_BUY_PREFIX,
                    item.id()
                ),
                &format!("Buy {} {}", item.emoji(), item.display_name()),
            )
            .disabled(profile.balance < cost),
        );
    }
    let mut rows = vec![CreateActionRow::Buttons(buy_buttons)];
    // Use row
    use crate::database::economy::get_inventory_item_simple;
    let mut use_buttons: Vec<serenity::builder::CreateButton> = Vec::new();
    for use_item in [Item::FocusTonic, Item::StaminaDraft] {
        let qty = get_inventory_item_simple(db, component.user.id, use_item)
            .await
            .ok()
            .flatten()
            .map(|i| i.quantity)
            .unwrap_or(0);
        let label = match use_item {
            Item::FocusTonic => {
                if let Some((active, remaining)) = &focus_state {
                    if *active {
                        let mins = remaining.as_secs() / 60;
                        let secs = remaining.as_secs() % 60;
                        format!(
                            "{} Focus active ({:02}:{:02})",
                            use_item.emoji(),
                            mins,
                            secs
                        )
                    } else {
                        format!(
                            "Use {} {} ({} in bag)",
                            use_item.emoji(),
                            use_item.display_name(),
                            qty
                        )
                    }
                } else {
                    format!(
                        "Use {} {} ({} in bag)",
                        use_item.emoji(),
                        use_item.display_name(),
                        qty
                    )
                }
            }
            Item::StaminaDraft => format!(
                "Use {} {} ({} in bag)",
                use_item.emoji(),
                use_item.display_name(),
                qty
            ),
            _ => String::new(),
        };
        use_buttons.push(
            crate::ui::buttons::Btn::primary(
                &format!(
                    "{}{}",
                    crate::interactions::ids::SAGA_TAVERN_USE_PREFIX,
                    use_item.id()
                ),
                &label,
            )
            .disabled(match use_item {
                Item::FocusTonic => {
                    qty <= 0 || focus_state.as_ref().map(|(a, _)| *a).unwrap_or(false)
                }
                _ => qty <= 0,
            }),
        );
    }
    rows.push(CreateActionRow::Buttons(use_buttons));
    // Home/Recruitment
    rows.push(CreateActionRow::Buttons(vec![
        crate::ui::buttons::Btn::secondary(crate::interactions::ids::SAGA_TAVERN_HOME, "🏰 Tavern"),
        crate::ui::buttons::Btn::primary(crate::interactions::ids::SAGA_RECRUIT, "🧭 Recruitment"),
    ]));
    // Tavern-specific nav row: ↩ Saga + Refresh (Saga not disabled)
    rows.push(crate::commands::saga::ui::tavern_saga_row());
    crate::interactions::util::edit_component(
        ctx,
        component,
        "tavern.goods",
        serenity::builder::EditInteractionResponse::new()
            .embed(embed)
            .components(rows),
    )
    .await;
}

// Small helper: render the Tavern Small Arms shop consistently, with optional notice line.
async fn render_tavern_shop_view(
    ctx: &Context,
    component: &mut ComponentInteraction,
    app_state: &Arc<AppState>,
    notice: Option<String>,
) {
    use crate::commands::economy::core::item::Item;
    use serenity::builder::{CreateActionRow, CreateEmbed};
    let db = &app_state.db;
    let catalog: Vec<Item> = crate::commands::saga::tavern::get_daily_shop_items(component.user.id);
    let profile = crate::database::economy::get_or_create_profile(db, component.user.id)
        .await
        .unwrap_or(crate::database::models::Profile {
            balance: 0,
            last_work: None,
            work_streak: 0,
            fishing_xp: 0,
            fishing_level: 1,
            mining_xp: 0,
            mining_level: 1,
            coding_xp: 0,
            coding_level: 1,
        });
    let (_, meta_tmp) =
        crate::commands::saga::tavern::build_tavern_state_cached(app_state, component.user.id)
            .await
            .unwrap_or((
                Vec::new(),
                crate::commands::saga::tavern::TavernUiMeta {
                    balance: profile.balance,
                    fame: 0,
                    fame_tier: 0,
                    fame_progress: 0.0,
                    daily_rerolls_used: 0,
                    max_daily_rerolls: crate::commands::saga::tavern::TAVERN_MAX_DAILY_REROLLS,
                    reroll_cost: crate::commands::saga::tavern::TAVERN_REROLL_COST,
                    can_reroll: false,
                },
            ));
    let (shop_disc, _, _) = crate::commands::saga::tavern::fame_perks(meta_tmp.fame_tier);
    let mut embed = CreateEmbed::new()
        .title("Tavern Shop — Small Arms")
        .description({
            let mut s = format!(
                "Basic gear and parchments for newcomers. Rotates daily. Resets in {}.",
                crate::commands::saga::tavern::time_until_reset_str()
            );
            if shop_disc > 0.0 {
                s.push_str(&format!(" Fame discount: -{}%", (shop_disc * 100.0) as i32));
            }
            if let Some(msg) = &notice {
                s.push_str(&format!("\nℹ️ {}", msg));
            }
            s
        })
        .field(
            "Balance",
            format!("{} {}", crate::ui::style::EMOJI_COIN, profile.balance),
            true,
        )
        .color(crate::ui::style::COLOR_SAGA_TAVERN);
    let mut buy_buttons = Vec::new();
    for item in &catalog {
        let base = tavern_price(*item).unwrap_or(1_000_000);
        let cost = crate::commands::saga::tavern::apply_shop_discount(base, shop_disc);
        let label = format!("{} {}", item.emoji(), item.display_name());
        let desc = item.properties().description;
        embed = embed.field(
            label,
            format!("{}\nCost: {} {}", desc, crate::ui::style::EMOJI_COIN, cost),
            true,
        );
        buy_buttons.push(
            crate::ui::buttons::Btn::success(
                &format!(
                    "{}{}",
                    crate::interactions::ids::SAGA_TAVERN_SHOP_BUY_PREFIX,
                    *item as i32
                ),
                &format!("Buy {} {}", item.emoji(), item.display_name()),
            )
            .disabled(profile.balance < cost),
        );
    }
    let mut rows = vec![CreateActionRow::Buttons(buy_buttons)];
    // Home + Recruitment row
    rows.push(CreateActionRow::Buttons(vec![
        crate::ui::buttons::Btn::secondary(crate::interactions::ids::SAGA_TAVERN_HOME, "🏰 Tavern"),
        crate::ui::buttons::Btn::primary(crate::interactions::ids::SAGA_RECRUIT, "🧭 Recruitment"),
    ]));
    rows.push(crate::commands::saga::ui::tavern_saga_row());
    crate::interactions::util::edit_component(
        ctx,
        component,
        "tavern.shop",
        serenity::builder::EditInteractionResponse::new()
            .embed(embed)
            .components(rows),
    )
    .await;
}

// Central Tavern Home renderer (Common Room menu)
async fn render_tavern_home_view(
    ctx: &Context,
    component: &mut ComponentInteraction,
    _app_state: &Arc<AppState>,
) {
    use serenity::builder::{CreateActionRow, CreateEmbed};
    let embed = CreateEmbed::new()
        .title("Tavern — Common Room")
        .description("Your home away from home. Choose a section below.")
        .color(crate::ui::style::COLOR_SAGA_TAVERN)
        .field(
            "1) Beers, Liquor, Food, Bait",
            "Consumables, buffs, and taming aids",
            false,
        )
        .field("2) Tavern Games", "Blackjack, Poker, and more", false)
        .field(
            "3) Quests",
            "Meet NPCs, guilds, and mysterious patrons",
            false,
        )
        .field(
            "4) Recruitment",
            "Hire mercenaries; Fame shown in UI",
            false,
        )
        .field(
            "5) Small Arms & Petty Equipment",
            "Basic gear until you find better shops",
            false,
        );
    let buttons = vec![
        crate::ui::buttons::Btn::secondary(crate::interactions::ids::SAGA_TAVERN_GOODS, "🍺 Goods"),
        crate::ui::buttons::Btn::secondary(crate::interactions::ids::SAGA_TAVERN_GAMES, "🎲 Games"),
        crate::ui::buttons::Btn::primary(crate::interactions::ids::SAGA_TAVERN_QUESTS, "🗺 Quests"),
        crate::ui::buttons::Btn::primary(crate::interactions::ids::SAGA_RECRUIT, "🧭 Recruitment"),
        crate::ui::buttons::Btn::secondary(crate::interactions::ids::SAGA_TAVERN_SHOP, "🗡 Shop"),
    ];
    let mut rows = vec![CreateActionRow::Buttons(buttons)];
    rows.push(crate::commands::saga::ui::tavern_saga_row());
    edit_component(
        ctx,
        component,
        "tavern.home",
        EditInteractionResponse::new().embed(embed).components(rows),
    )
    .await;
}

#[instrument(level="debug", skip(ctx, component, app_state), fields(user_id = component.user.id.get(), cid = %component.data.custom_id))]
pub async fn handle(ctx: &Context, component: &mut ComponentInteraction, app_state: Arc<AppState>) {
    let db = &app_state.db;
    let raw_id = component.data.custom_id.as_str();
    let custom_id_parts: Vec<&str> = raw_id.split('_').collect();
    tracing::debug!(target="saga.interaction", user_id=%component.user.id, cid=%raw_id, "Saga component received");

    // Standard unified defer + global nav handling
    defer_component(ctx, component).await;
    if handle_global_nav(ctx, component, &app_state, "saga").await {
        return;
    }

    const MAX_NAV_DEPTH: usize = 15;

    // Centralized back / refresh.
    if handle_saga_back_refresh(ctx, component, &app_state).await {
        return;
    }

    match custom_id_parts.get(1) {
        Some(&"preview") if crate::interactions::ids::is_saga_preview(raw_id) => {
            // Preview a node's enemies & rewards without spending AP.
            let node_id = match raw_id
                .trim_start_matches(crate::interactions::ids::SAGA_PREVIEW_PREFIX)
                .parse::<i32>()
            {
                Ok(v) => v,
                Err(_) => {
                    edit_component(
                        ctx,
                        component,
                        "preview.bad_id",
                        EditInteractionResponse::new().content("Invalid node id for preview."),
                    )
                    .await;
                    return;
                }
            };
            match database::world::get_full_node_bundle(db, node_id).await {
                Ok((node, enemies, rewards)) => {
                    use serenity::builder::CreateEmbed;
                    let mut embed = CreateEmbed::new()
                        .title(format!("Node Preview: {}", node.name))
                        .description(
                            node.description
                                .clone()
                                .unwrap_or_else(|| "No description.".into()),
                        )
                        .field(
                            "Story Progress Req",
                            format!("`{}`", node.story_progress_required),
                            true,
                        )
                        .field(
                            "Base Rewards",
                            format!("💰 {} | XP {}", node.reward_coins, node.reward_unit_xp),
                            true,
                        )
                        .color(crate::ui::style::COLOR_SAGA_MAP);
                    // Fetch profile once for difficulty + AP checks
                    let profile_opt = crate::services::saga::get_saga_profile(
                        &app_state,
                        component.user.id,
                        false,
                    )
                    .await;
                    // Show a compact difficulty tag (E, =, M, H) similar to map UI
                    if let Some(profile) = &profile_opt {
                        let tag = if node.story_progress_required > profile.story_progress + 2 {
                            "HARD"
                        } else if node.story_progress_required > profile.story_progress {
                            "MOD"
                        } else if node.story_progress_required + 2 < profile.story_progress {
                            "EASY"
                        } else {
                            "EVEN"
                        };
                        let sym = match tag {
                            "EASY" => "E",
                            "EVEN" => "=",
                            "MOD" => "M",
                            "HARD" => "H",
                            _ => "?",
                        };
                        embed = embed.field("Difficulty", format!("{} ({})", sym, tag), true);
                    }
                    if !enemies.is_empty() {
                        let shown = 10usize;
                        let enemy_lines = enemies
                            .iter()
                            .map(|e| format!("- {} ({:?})", e.name, e.rarity))
                            .take(shown)
                            .collect::<Vec<_>>()
                            .join("\n");
                        let extra = enemies.len().saturating_sub(shown);
                        let val = if extra > 0 {
                            format!("{}\n… and {} more", enemy_lines, extra)
                        } else {
                            enemy_lines
                        };
                        embed = embed.field("Enemies", val, false);
                    }
                    if !rewards.is_empty() {
                        let shown = 10usize;
                        let reward_lines = rewards
                            .iter()
                            .take(shown)
                            .map(|r| {
                                format!(
                                    "• Item {} x{} ({}%)",
                                    r.item_id,
                                    r.quantity,
                                    (r.drop_chance * 100.0) as i32
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        let extra = rewards.len().saturating_sub(shown);
                        let val = if extra > 0 {
                            format!("{}\n… and {} more", reward_lines, extra)
                        } else {
                            reward_lines
                        };
                        embed = embed.field("Possible Drops", val, false);
                    }
                    let mut components = Vec::new();
                    // Provide a Start Battle button (spends AP) reflecting AP availability.
                    let ap_ok = profile_opt
                        .as_ref()
                        .map(|p| p.current_ap > 0)
                        .unwrap_or(false);
                    let start_label = if ap_ok {
                        "⚔ Start Battle (1 AP)"
                    } else {
                        "⚔ Start Battle (No AP)"
                    };
                    if !ap_ok {
                        embed = embed.field(
                            "Action Points",
                            "You need 1 AP to start this battle.",
                            false,
                        );
                    }
                    components.push(serenity::builder::CreateActionRow::Buttons(vec![
                        crate::ui::buttons::Btn::primary(
                            &format!(
                                "{}{}",
                                crate::interactions::ids::SAGA_NODE_PREFIX,
                                node.node_id
                            ),
                            start_label,
                        )
                        .disabled(!ap_ok),
                    ]));
                    // Standard saga navigation controls: Back+Refresh (if depth>1) then global nav row.
                    let depth = app_state
                        .nav_stacks
                        .read()
                        .await
                        .get(&component.user.id.get())
                        .map(|s| s.stack.len())
                        .unwrap_or(1);
                    crate::commands::saga::ui::insert_back_before_nav(
                        &mut components,
                        depth,
                        "saga",
                    );
                    components.push(crate::commands::saga::ui::global_nav_row("saga"));
                    edit_component(
                        ctx,
                        component,
                        "preview.render",
                        EditInteractionResponse::new()
                            .embed(embed)
                            .components(components),
                    )
                    .await;
                }
                Err(_) => {
                    edit_component(
                        ctx,
                        component,
                        "preview.load_err",
                        EditInteractionResponse::new().content("Failed to load preview."),
                    )
                    .await;
                }
            }
            return;
        }
        Some(&"tavern") if raw_id.starts_with(crate::interactions::ids::SAGA_TAVERN_USE_PREFIX) => {
            use crate::commands::economy::core::item::Item;
            // Parse item id
            let id_str =
                raw_id.trim_start_matches(crate::interactions::ids::SAGA_TAVERN_USE_PREFIX);
            let Some(item_id) = id_str.parse::<i32>().ok() else {
                edit_component(
                    ctx,
                    component,
                    "tavern.use.bad_id",
                    EditInteractionResponse::new().content("Invalid item id."),
                )
                .await;
                return;
            };
            let Some(item) = Item::from_i32(item_id) else {
                edit_component(
                    ctx,
                    component,
                    "tavern.use.unknown_item",
                    EditInteractionResponse::new().content("Unknown item."),
                )
                .await;
                return;
            };
            match item {
                Item::FocusTonic => {
                    // Atomically decrement inventory; on success, set buff cache
                    let mut tx = match db.begin().await {
                        Ok(t) => t,
                        Err(e) => {
                            edit_component(
                                ctx,
                                component,
                                "tavern.use.tx_err",
                                EditInteractionResponse::new()
                                    .content(format!("Failed to use item: {}", e)),
                            )
                            .await;
                            return;
                        }
                    };
                    // Ensure the user has at least 1
                    match crate::database::economy::add_to_inventory(
                        &mut tx,
                        component.user.id,
                        Item::FocusTonic,
                        -1,
                    )
                    .await
                    {
                        Ok(()) => {
                            let _ = tx.commit().await;
                            // Set focus buff active in TTL cache
                            crate::services::cache::insert(
                                &app_state.focus_buff_cache,
                                component.user.id.get(),
                                true,
                            )
                            .await;
                            // Re-render goods with confirmation banner
                            component.data.custom_id =
                                crate::interactions::ids::SAGA_TAVERN_GOODS.into();
                            render_tavern_goods_view(
                                ctx,
                                component,
                                &app_state,
                                Some("Focus Tonic used. Buff active.".to_string()),
                            )
                            .await;
                        }
                        Err(_) => {
                            let _ = tx.rollback().await;
                            edit_component(
                                ctx,
                                component,
                                "tavern.use.focus.fail",
                                EditInteractionResponse::new()
                                    .content("You don't have a Focus Tonic."),
                            )
                            .await;
                        }
                    }
                }
                Item::StaminaDraft => {
                    // Attempt to restore AP if not full; else restore TP up to a small amount.
                    // Start a transaction to deduct the item, then update saga profile accordingly.
                    let mut tx = match db.begin().await {
                        Ok(t) => t,
                        Err(e) => {
                            edit_component(
                                ctx,
                                component,
                                "tavern.use.tx_err",
                                EditInteractionResponse::new()
                                    .content(format!("Failed to use item: {}", e)),
                            )
                            .await;
                            return;
                        }
                    };
                    // Check inventory quantity FOR UPDATE
                    match crate::database::economy::get_inventory_item(
                        &mut tx,
                        component.user.id,
                        Item::StaminaDraft,
                    )
                    .await
                    {
                        Ok(Some(inv)) if inv.quantity > 0 => {
                            // Deduct 1 now
                            if let Err(e) = crate::database::economy::add_to_inventory(
                                &mut tx,
                                component.user.id,
                                Item::StaminaDraft,
                                -1,
                            )
                            .await
                            {
                                let _ = tx.rollback().await;
                                edit_component(
                                    ctx,
                                    component,
                                    "tavern.use.stamina.fail",
                                    EditInteractionResponse::new()
                                        .content(format!("Could not consume item: {}", e)),
                                )
                                .await;
                                return;
                            }
                            // Fetch current saga profile inside tx and decide restoration
                            let uid = component.user.id.get() as i64;
                            // Lock profile row
                            let current = sqlx::query!("SELECT current_ap, max_ap, current_tp, max_tp FROM player_saga_profile WHERE user_id = $1 FOR UPDATE", uid).fetch_one(&mut *tx).await;
                            if let Ok(p) = current {
                                let mut new_ap = p.current_ap;
                                let mut new_tp = p.current_tp;
                                let restore_msg = if p.current_ap < p.max_ap {
                                    new_ap = (p.current_ap + 1).min(p.max_ap);
                                    format!("Restored 1 AP (now {}/{}).", new_ap, p.max_ap)
                                } else if p.current_tp < p.max_tp {
                                    let add = 5i32.min(p.max_tp - p.current_tp);
                                    new_tp = p.current_tp + add;
                                    format!("Restored {} TP (now {}/{}).", add, new_tp, p.max_tp)
                                } else {
                                    let _ = tx.rollback().await;
                                    edit_component(
                                        ctx,
                                        component,
                                        "tavern.use.stamina.full",
                                        EditInteractionResponse::new()
                                            .content("Your AP and TP are already full."),
                                    )
                                    .await;
                                    return;
                                };
                                // Apply update
                                if let Err(e) = sqlx::query!("UPDATE player_saga_profile SET current_ap = $1, current_tp = $2 WHERE user_id = $3", new_ap, new_tp, uid).execute(&mut *tx).await {
                                    let _ = tx.rollback().await;
                                    edit_component(ctx, component, "tavern.use.stamina.fail2", EditInteractionResponse::new().content(format!("Could not update profile: {}", e))).await;
                                    return;
                                }
                                if tx.commit().await.is_ok() {
                                    // Invalidate saga profile cache so fresh values render
                                    app_state.invalidate_user_caches(component.user.id).await;
                                    // Re-render goods after success with a short confirmation notice
                                    component.data.custom_id =
                                        crate::interactions::ids::SAGA_TAVERN_GOODS.into();
                                    render_tavern_goods_view(
                                        ctx,
                                        component,
                                        &app_state,
                                        Some(restore_msg),
                                    )
                                    .await;
                                } else {
                                    edit_component(
                                        ctx,
                                        component,
                                        "tavern.use.stamina.commit",
                                        EditInteractionResponse::new()
                                            .content("Failed to commit stamina use."),
                                    )
                                    .await;
                                }
                            } else {
                                let _ = tx.rollback().await;
                                edit_component(
                                    ctx,
                                    component,
                                    "tavern.use.stamina.no_profile",
                                    EditInteractionResponse::new()
                                        .content("No saga profile found."),
                                )
                                .await;
                            }
                        }
                        _ => {
                            let _ = tx.rollback().await;
                            edit_component(
                                ctx,
                                component,
                                "tavern.use.stamina.none",
                                EditInteractionResponse::new()
                                    .content("You don't have a Stamina Draft."),
                            )
                            .await;
                        }
                    }
                }
                _ => {
                    edit_component(
                        ctx,
                        component,
                        "tavern.use.unsupported",
                        EditInteractionResponse::new().content("This item can't be used here."),
                    )
                    .await;
                }
            }
        }
        // Map view activation
        Some(&"map") => {
            // Guard: need party + 1 AP
            let saga_profile =
                match database::saga::update_and_get_saga_profile(db, component.user.id).await {
                    Ok(p) => p,
                    Err(e) => {
                        edit_component(
                            ctx,
                            component,
                            "map.profile_err",
                            EditInteractionResponse::new()
                                .content(format!("Failed to load profile: {e}")),
                        )
                        .await;
                        return;
                    }
                };
            let has_party = database::units::get_user_party(db, component.user.id)
                .await
                .map(|p| !p.is_empty())
                .unwrap_or(false);
            if saga_profile.current_ap < 1 || !has_party {
                edit_component(
                    ctx,
                    component,
                    "map.blocked",
                    EditInteractionResponse::new()
                        .content("You can't open the World Map right now (need party and 1+ AP)."),
                )
                .await;
                return;
            }
            if let Ok((embed, mut components)) = crate::saga::view::push_and_render(
                crate::saga::view::SagaView::Map,
                &app_state,
                component.user.id,
                15,
            )
            .await
            {
                let depth = app_state
                    .nav_stacks
                    .read()
                    .await
                    .get(&component.user.id.get())
                    .map(|s| s.stack.len())
                    .unwrap_or(1);
                crate::commands::saga::ui::insert_back_before_nav(&mut components, depth, "saga");
                edit_component(
                    ctx,
                    component,
                    "map.render",
                    EditInteractionResponse::new()
                        .embed(embed)
                        .components(components),
                )
                .await;
            } else {
                edit_component(
                    ctx,
                    component,
                    "map.render_err",
                    EditInteractionResponse::new().content("Failed to render map."),
                )
                .await;
            }
        }
        Some(&"tavern") if raw_id == crate::interactions::ids::SAGA_TAVERN_CANCEL => {
            // Session pagination removed; simply re-render current tavern state.
            let (recruits, meta) = match crate::commands::saga::tavern::build_tavern_state_cached(
                &app_state,
                component.user.id,
            )
            .await
            {
                Ok(v) => v,
                Err(_) => (
                    Vec::new(),
                    crate::commands::saga::tavern::TavernUiMeta {
                        balance: 0,
                        fame: 0,
                        fame_tier: 0,
                        fame_progress: 0.0,
                        daily_rerolls_used: 0,
                        max_daily_rerolls: crate::commands::saga::tavern::TAVERN_MAX_DAILY_REROLLS,
                        reroll_cost: crate::commands::saga::tavern::TAVERN_REROLL_COST,
                        can_reroll: false,
                    },
                ),
            };
            let (embed, mut components) =
                crate::commands::saga::tavern::create_tavern_menu(&recruits, &meta);
            components.push(crate::commands::saga::ui::tavern_saga_row());
            edit_component(
                ctx,
                component,
                "tavern.cancel",
                EditInteractionResponse::new()
                    .embed(embed)
                    .components(components),
            )
            .await;
        }
        // Open Tavern home menu (choices) instead of jumping to recruitment
        Some(&"tavern") if raw_id == crate::interactions::ids::SAGA_TAVERN => {
            render_tavern_home_view(ctx, component, &app_state).await;
        }
        Some(&"tavern") if raw_id == crate::interactions::ids::SAGA_TAVERN_HOME => {
            render_tavern_home_view(ctx, component, &app_state).await;
        }
        Some(&"tavern") if raw_id == crate::interactions::ids::SAGA_TAVERN_GOODS => {
            // Unified goods renderer
            render_tavern_goods_view(ctx, component, &app_state, None).await;
        }
        // Recruitment entry: open the recruitment (rotation) view
        Some(&"recruit") if raw_id == crate::interactions::ids::SAGA_RECRUIT => {
            if let Ok((embed, mut components)) = push_and_render(
                SagaView::Tavern,
                &app_state,
                component.user.id,
                MAX_NAV_DEPTH,
            )
            .await
            {
                components.push(crate::commands::saga::ui::tavern_saga_row());
                edit_component(
                    ctx,
                    component,
                    "tavern.recruit",
                    EditInteractionResponse::new()
                        .embed(embed)
                        .components(components),
                )
                .await;
            }
        }
        // Tavern: Hire flow (confirm)
        Some(_) if raw_id.starts_with(crate::interactions::ids::SAGA_HIRE_PREFIX) => {
            use serenity::builder::CreateActionRow;
            let id_str = raw_id.trim_start_matches(crate::interactions::ids::SAGA_HIRE_PREFIX);
            let Some(unit_id) = id_str.parse::<i32>().ok() else {
                edit_component(
                    ctx,
                    component,
                    "tavern.hire.bad_id",
                    EditInteractionResponse::new().content("Invalid unit id."),
                )
                .await;
                return;
            };
            // Load unit and user balance
            let units = crate::database::units::get_units_by_ids(db, &[unit_id]).await;
            let Some(unit) = units.ok().and_then(|mut v| v.pop()) else {
                edit_component(
                    ctx,
                    component,
                    "tavern.hire.not_found",
                    EditInteractionResponse::new().content("Unit not found."),
                )
                .await;
                return;
            };
            if !unit.is_recruitable {
                edit_component(
                    ctx,
                    component,
                    "tavern.hire.unrecruitable",
                    EditInteractionResponse::new().content("This unit cannot be hired."),
                )
                .await;
                return;
            }
            let balance = crate::database::economy::get_or_create_profile(db, component.user.id)
                .await
                .map(|p| p.balance)
                .unwrap_or(0);
            // Reuse existing helper to build a consistent confirmation embed
            let embed = crate::commands::saga::tavern::create_hire_confirmation(&unit, balance);
            let can_afford =
                balance >= crate::commands::saga::tavern::hire_cost_for_rarity(unit.rarity);
            let buttons = vec![
                crate::ui::buttons::Btn::success(
                    &format!(
                        "{}{}",
                        crate::interactions::ids::SAGA_HIRE_CONFIRM_PREFIX,
                        unit.unit_id
                    ),
                    "✅ Confirm",
                )
                .disabled(!can_afford),
                crate::ui::buttons::Btn::secondary(
                    crate::interactions::ids::SAGA_HIRE_CANCEL,
                    "❌ Cancel",
                ),
            ];
            let mut rows = vec![CreateActionRow::Buttons(buttons)];
            // Home + Recruitment row
            rows.push(CreateActionRow::Buttons(vec![
                crate::ui::buttons::Btn::secondary(
                    crate::interactions::ids::SAGA_TAVERN_HOME,
                    "🏰 Tavern",
                ),
                crate::ui::buttons::Btn::primary(
                    crate::interactions::ids::SAGA_RECRUIT,
                    "🧭 Recruitment",
                ),
            ]));
            rows.push(crate::commands::saga::ui::tavern_saga_row());
            edit_component(
                ctx,
                component,
                "tavern.hire.confirm",
                EditInteractionResponse::new().embed(embed).components(rows),
            )
            .await;
        }
        Some(_) if raw_id == crate::interactions::ids::SAGA_HIRE_CANCEL => {
            // Re-render the recruitment menu via build_tavern_state_cached
            let (recruits, meta) = crate::commands::saga::tavern::build_tavern_state_cached(
                &app_state,
                component.user.id,
            )
            .await
            .unwrap_or((
                Vec::new(),
                crate::commands::saga::tavern::TavernUiMeta {
                    balance: 0,
                    fame: 0,
                    fame_tier: 0,
                    fame_progress: 0.0,
                    daily_rerolls_used: 0,
                    max_daily_rerolls: crate::commands::saga::tavern::TAVERN_MAX_DAILY_REROLLS,
                    reroll_cost: crate::commands::saga::tavern::TAVERN_REROLL_COST,
                    can_reroll: false,
                },
            ));
            let (embed, mut components) =
                crate::commands::saga::tavern::create_tavern_menu(&recruits, &meta);
            components.push(crate::commands::saga::ui::tavern_saga_row());
            edit_component(
                ctx,
                component,
                "tavern.hire.cancel",
                EditInteractionResponse::new()
                    .embed(embed)
                    .components(components),
            )
            .await;
        }
        Some(_) if raw_id.starts_with(crate::interactions::ids::SAGA_HIRE_CONFIRM_PREFIX) => {
            // Commit hire: charge coins atomically and add unit + fame
            let id_str =
                raw_id.trim_start_matches(crate::interactions::ids::SAGA_HIRE_CONFIRM_PREFIX);
            let Some(unit_id) = id_str.parse::<i32>().ok() else {
                edit_component(
                    ctx,
                    component,
                    "tavern.hireC.bad_id",
                    EditInteractionResponse::new().content("Invalid unit id."),
                )
                .await;
                return;
            };
            // Need unit rarity to compute cost
            let units = crate::database::units::get_units_by_ids(db, &[unit_id]).await;
            let Some(unit) = units.ok().and_then(|mut v| v.pop()) else {
                edit_component(
                    ctx,
                    component,
                    "tavern.hireC.not_found",
                    EditInteractionResponse::new().content("Unit not found."),
                )
                .await;
                return;
            };
            if !unit.is_recruitable {
                edit_component(
                    ctx,
                    component,
                    "tavern.hireC.unrecruitable",
                    EditInteractionResponse::new().content("This unit cannot be hired."),
                )
                .await;
                return;
            }
            let cost = crate::commands::saga::tavern::hire_cost_for_rarity(unit.rarity);
            match crate::database::units::hire_unit(db, component.user.id, unit_id, cost).await {
                Ok(name) => {
                    // Award small fame for hiring
                    let _ = crate::database::tavern::add_fame(
                        db,
                        component.user.id,
                        crate::commands::saga::tavern::FAME_PER_HIRE,
                    )
                    .await;
                    // Re-render with fresh state
                    let (recruits, meta) =
                        crate::commands::saga::tavern::build_tavern_state_cached(
                            &app_state,
                            component.user.id,
                        )
                        .await
                        .unwrap_or((
                            Vec::new(),
                            crate::commands::saga::tavern::TavernUiMeta {
                                balance: 0,
                                fame: 0,
                                fame_tier: 0,
                                fame_progress: 0.0,
                                daily_rerolls_used: 0,
                                max_daily_rerolls:
                                    crate::commands::saga::tavern::TAVERN_MAX_DAILY_REROLLS,
                                reroll_cost: crate::commands::saga::tavern::TAVERN_REROLL_COST,
                                can_reroll: false,
                            },
                        ));
                    let notice = format!(
                        "Hired {}! Gained {} Fame.",
                        name,
                        crate::commands::saga::tavern::FAME_PER_HIRE
                    );
                    let (mut embed, mut components) =
                        crate::commands::saga::tavern::create_tavern_menu(&recruits, &meta);
                    embed = embed.field("Notice", notice, false);
                    components.push(crate::commands::saga::ui::tavern_saga_row());
                    edit_component(
                        ctx,
                        component,
                        "tavern.hire.success",
                        EditInteractionResponse::new()
                            .embed(embed)
                            .components(components),
                    )
                    .await;
                }
                Err(e) => {
                    edit_component(
                        ctx,
                        component,
                        "tavern.hire.fail",
                        EditInteractionResponse::new()
                            .content(format!("Could not hire unit: {}", e)),
                    )
                    .await;
                }
            }
        }
        Some(&"tavern") if raw_id.starts_with(crate::interactions::ids::SAGA_TAVERN_BUY_PREFIX) => {
            use crate::commands::economy::core::item::Item;
            use serenity::builder::{CreateActionRow, CreateEmbed};
            let id_str =
                raw_id.trim_start_matches(crate::interactions::ids::SAGA_TAVERN_BUY_PREFIX);
            let Some(item_id) = id_str.parse::<i32>().ok() else {
                edit_component(
                    ctx,
                    component,
                    "tavern.buy.bad_id",
                    EditInteractionResponse::new().content("Invalid item id."),
                )
                .await;
                return;
            };
            let Some(item) = Item::from_i32(item_id) else {
                edit_component(
                    ctx,
                    component,
                    "tavern.buy.unknown_item",
                    EditInteractionResponse::new().content("Unknown item."),
                )
                .await;
                return;
            };
            // Compute balance first (needed for fallback meta), then apply fame-based discount
            let balance = crate::database::economy::get_or_create_profile(db, component.user.id)
                .await
                .map(|p| p.balance)
                .unwrap_or(0);
            let (_, meta_tmp) = crate::commands::saga::tavern::build_tavern_state_cached(
                &app_state,
                component.user.id,
            )
            .await
            .unwrap_or((
                Vec::new(),
                crate::commands::saga::tavern::TavernUiMeta {
                    balance,
                    fame: 0,
                    fame_tier: 0,
                    fame_progress: 0.0,
                    daily_rerolls_used: 0,
                    max_daily_rerolls: crate::commands::saga::tavern::TAVERN_MAX_DAILY_REROLLS,
                    reroll_cost: crate::commands::saga::tavern::TAVERN_REROLL_COST,
                    can_reroll: false,
                },
            ));
            let (shop_disc, _, _) = crate::commands::saga::tavern::fame_perks(meta_tmp.fame_tier);
            let base = tavern_price(item).unwrap_or(1_000_000);
            let cost: i64 = crate::commands::saga::tavern::apply_shop_discount(base, shop_disc);
            let mut embed = CreateEmbed::new()
                .title("Confirm Purchase")
                .description(format!(
                    "Buy {} {} for {} {}?\n{}",
                    item.emoji(),
                    item.display_name(),
                    crate::ui::style::EMOJI_COIN,
                    cost,
                    item.properties().description
                ))
                .field(
                    "Your Balance",
                    format!("{} {}", crate::ui::style::EMOJI_COIN, balance),
                    true,
                )
                .color(crate::ui::style::COLOR_SAGA_TAVERN);
            if balance < cost {
                embed = embed.field("Note", "You cannot afford this.", false);
            }
            let buttons = vec![
                crate::ui::buttons::Btn::success(
                    &format!(
                        "{}{}",
                        crate::interactions::ids::SAGA_TAVERN_BUY_CONFIRM_PREFIX,
                        item_id
                    ),
                    "✅ Confirm",
                )
                .disabled(balance < cost),
                crate::ui::buttons::Btn::secondary(
                    crate::interactions::ids::SAGA_TAVERN_BUY_CANCEL,
                    "❌ Cancel",
                ),
            ];
            let mut rows = vec![CreateActionRow::Buttons(buttons)];
            // Home + Recruitment
            rows.push(CreateActionRow::Buttons(vec![
                crate::ui::buttons::Btn::secondary(
                    crate::interactions::ids::SAGA_TAVERN_HOME,
                    "🏰 Tavern",
                ),
                crate::ui::buttons::Btn::primary(
                    crate::interactions::ids::SAGA_RECRUIT,
                    "🧭 Recruitment",
                ),
            ]));
            rows.push(crate::commands::saga::ui::tavern_saga_row());
            edit_component(
                ctx,
                component,
                "tavern.buy.confirm",
                EditInteractionResponse::new().embed(embed).components(rows),
            )
            .await;
        }
        Some(&"tavern") if raw_id == crate::interactions::ids::SAGA_TAVERN_BUY_CANCEL => {
            // Re-render goods via unified renderer
            component.data.custom_id = crate::interactions::ids::SAGA_TAVERN_GOODS.into();
            render_tavern_goods_view(ctx, component, &app_state, None).await;
        }
        Some(&"tavern")
            if raw_id.starts_with(crate::interactions::ids::SAGA_TAVERN_BUY_CONFIRM_PREFIX) =>
        {
            use crate::commands::economy::core::item::Item;
            let id_str =
                raw_id.trim_start_matches(crate::interactions::ids::SAGA_TAVERN_BUY_CONFIRM_PREFIX);
            let Some(item_id) = id_str.parse::<i32>().ok() else {
                edit_component(
                    ctx,
                    component,
                    "tavern.buyC.bad_id",
                    EditInteractionResponse::new().content("Invalid item id."),
                )
                .await;
                return;
            };
            let Some(item) = Item::from_i32(item_id) else {
                edit_component(
                    ctx,
                    component,
                    "tavern.buyC.unknown_item",
                    EditInteractionResponse::new().content("Unknown item."),
                )
                .await;
                return;
            };
            // Apply fame-based discount for Goods purchases
            let profile_balance =
                crate::database::economy::get_or_create_profile(db, component.user.id)
                    .await
                    .map(|p| p.balance)
                    .unwrap_or(0);
            let (_, meta_tmp) = crate::commands::saga::tavern::build_tavern_state_cached(
                &app_state,
                component.user.id,
            )
            .await
            .unwrap_or((
                Vec::new(),
                crate::commands::saga::tavern::TavernUiMeta {
                    balance: profile_balance,
                    fame: 0,
                    fame_tier: 0,
                    fame_progress: 0.0,
                    daily_rerolls_used: 0,
                    max_daily_rerolls: crate::commands::saga::tavern::TAVERN_MAX_DAILY_REROLLS,
                    reroll_cost: crate::commands::saga::tavern::TAVERN_REROLL_COST,
                    can_reroll: false,
                },
            ));
            let (shop_disc, _, _) = crate::commands::saga::tavern::fame_perks(meta_tmp.fame_tier);
            let base = tavern_price(item).unwrap_or(1_000_000);
            let cost: i64 = crate::commands::saga::tavern::apply_shop_discount(base, shop_disc);
            // Atomic purchase
            let mut tx = match db.begin().await {
                Ok(t) => t,
                Err(e) => {
                    edit_component(
                        ctx,
                        component,
                        "tavern.buyC.tx_err",
                        EditInteractionResponse::new()
                            .content(format!("Failed to start purchase: {}", e)),
                    )
                    .await;
                    return;
                }
            };
            let res = {
                match crate::database::economy::add_balance(&mut tx, component.user.id, -cost).await
                {
                    Ok(()) => {
                        crate::database::economy::add_to_inventory(
                            &mut tx,
                            component.user.id,
                            item,
                            1,
                        )
                        .await
                    }
                    Err(e) => Err(e),
                }
            };
            match res {
                Ok(()) => {
                    if tx.commit().await.is_err() {
                        edit_component(
                            ctx,
                            component,
                            "tavern.buyC.commit_err",
                            EditInteractionResponse::new().content("Purchase failed to commit."),
                        )
                        .await;
                        return;
                    }
                    // Re-render goods with a succinct purchase notice
                    component.data.custom_id = crate::interactions::ids::SAGA_TAVERN_GOODS.into();
                    let notice = Some(format!(
                        "You bought {} for {} {}.",
                        item.display_name(),
                        crate::ui::style::EMOJI_COIN,
                        cost
                    ));
                    render_tavern_goods_view(ctx, component, &app_state, notice).await;
                }
                Err(_) => {
                    // Likely insufficient funds or race; rollback auto on drop
                    let _ = tx.rollback().await;
                    edit_component(
                        ctx,
                        component,
                        "tavern.buyC.fail",
                        EditInteractionResponse::new()
                            .content("Could not complete purchase (balance changed?)."),
                    )
                    .await;
                }
            }
        }
        Some(&"tavern") if raw_id == crate::interactions::ids::SAGA_TAVERN_GAMES => {
            render_tavern_games_view(ctx, component, &app_state).await;
        }
        Some(&"tavern") if raw_id == crate::interactions::ids::SAGA_TAVERN_GAMES_BLACKJACK => {
            // Start a friendly (no-ante) Blackjack table immediately
            use crate::commands::blackjack::state::BlackjackGame;
            let game = BlackjackGame::new(Arc::new(component.user.clone()), 0);
            let mut gm = app_state.game_manager.write().await;
            let (content, embed, components) = game.render();
            let builder = EditInteractionResponse::new()
                .content(content)
                .embed(embed)
                .components(components);
            if let Ok(msg) = component.edit_response(&ctx.http, builder).await {
                gm.start_game(msg.id, Box::new(game));
            }
        }
        Some(&"tavern") if raw_id == crate::interactions::ids::SAGA_TAVERN_GAMES_POKER => {
            // Start a friendly (no-ante) Poker table immediately
            use crate::commands::poker::state::PokerGame;
            let game = PokerGame::new(Arc::new(component.user.clone()), 0);
            let mut gm = app_state.game_manager.write().await;
            let (content, embed, components) = game.render();
            let builder = EditInteractionResponse::new()
                .content(content)
                .embed(embed)
                .components(components);
            if let Ok(msg) = component.edit_response(&ctx.http, builder).await {
                gm.start_game(msg.id, Box::new(game));
            }
        }
        Some(&"tavern")
            if raw_id == crate::interactions::ids::SAGA_TAVERN_GAMES_ARM
                || raw_id == crate::interactions::ids::SAGA_TAVERN_GAMES_DARTS =>
        {
            // Party selection prompt for stat-based game
            let units = database::units::get_user_party(db, component.user.id)
                .await
                .unwrap_or_default();
            if units.is_empty() {
                edit_component(
                    ctx,
                    component,
                    "tavern.games.no_party",
                    EditInteractionResponse::new().content("You need a party member to play."),
                )
                .await;
                return;
            }
            let embed = serenity::builder::CreateEmbed::new()
                .title("Choose a contestant")
                .description("Pick a party member to represent you.")
                .color(crate::ui::style::COLOR_SAGA_TAVERN);
            let mut btns = Vec::new();
            for u in units.iter().take(5) {
                btns.push(crate::ui::buttons::Btn::secondary(
                    &format!(
                        "{}{}_{}",
                        crate::interactions::ids::SAGA_TAVERN_GAMES_PLAY_PREFIX,
                        if raw_id == crate::interactions::ids::SAGA_TAVERN_GAMES_ARM {
                            "arm"
                        } else {
                            "darts"
                        },
                        u.unit_id
                    ),
                    &u.name,
                ));
            }
            let rows = vec![
                serenity::builder::CreateActionRow::Buttons(btns),
                crate::commands::saga::ui::tavern_saga_row(),
            ];
            edit_component(
                ctx,
                component,
                "tavern.games.pick",
                EditInteractionResponse::new().embed(embed).components(rows),
            )
            .await;
        }
        Some(&"tavern")
            if raw_id.starts_with(crate::interactions::ids::SAGA_TAVERN_GAMES_PLAY_PREFIX) =>
        {
            // Resolve the mini-game with a simple stat check
            let parts: Vec<&str> = raw_id.split('_').collect();
            let game = parts.get(4).copied().unwrap_or("arm");
            let unit_id = parts.get(5).and_then(|s| s.parse::<i32>().ok());
            let Some(unit_id) = unit_id else {
                edit_component(
                    ctx,
                    component,
                    "tavern.games.bad_id",
                    EditInteractionResponse::new().content("Invalid unit id."),
                )
                .await;
                return;
            };
            let units = database::units::get_user_party(db, component.user.id)
                .await
                .unwrap_or_default();
            let Some(unit) = units.into_iter().find(|u| u.unit_id == unit_id) else {
                edit_component(
                    ctx,
                    component,
                    "tavern.games.no_unit",
                    EditInteractionResponse::new().content("Unit not in your party."),
                )
                .await;
                return;
            };
            // Stat check: stat bonus + d20 vs. DC (seeded for fairness)
            let (stat, dc, label, stat_name) = if game == "arm" {
                (unit.current_attack, 13, "Arm Wrestling", "Strength")
            } else {
                (unit.current_defense, 13, "Darts", "Dexterity")
            };
            let today = chrono::Utc::now().date_naive();
            let seed = (component.user.id.get())
                ^ (((today.year() as u64) << 32) ^ (today.ordinal() as u64))
                ^ (unit_id as u64);
            let roll = (crate::commands::saga::tavern::splitmix64(seed) % 20 + 1) as i32; // d20
            let score = stat / 10 + roll; // stat bonus scaled
            let success = score >= dc;
            let mut embed = serenity::builder::CreateEmbed::new()
                .title(format!("{} — Result", label))
                .description(format!(
                    "{} used {} ({}), rolled d20 = {} ➜ total {} vs DC {} → {}",
                    unit.name,
                    stat_name,
                    stat,
                    roll,
                    score,
                    dc,
                    if success { "Win" } else { "Loss" }
                ))
                .color(crate::ui::style::COLOR_SAGA_TAVERN);
            if success {
                embed = embed.field(
                    "Winnings",
                    format!("{} {}", crate::ui::style::EMOJI_COIN, 25),
                    true,
                );
                if let Ok(mut tx) = db.begin().await {
                    let _ =
                        crate::database::economy::add_balance(&mut tx, component.user.id, 25).await;
                    let _ = tx.commit().await;
                }
            }
            // Provide quick return to Tavern
            let rows = vec![
                serenity::builder::CreateActionRow::Buttons(vec![
                    crate::ui::buttons::Btn::secondary(
                        crate::interactions::ids::SAGA_TAVERN_HOME,
                        "🏰 Tavern",
                    ),
                ]),
                crate::commands::saga::ui::tavern_saga_row(),
            ];
            edit_component(
                ctx,
                component,
                "tavern.games.result",
                EditInteractionResponse::new().embed(embed).components(rows),
            )
            .await;
        }
        Some(&"tavern") if raw_id == crate::interactions::ids::SAGA_TAVERN_QUESTS => {
            // Wire to real quest board UI
            match crate::database::quests::get_or_refresh_quest_board(db, component.user.id).await {
                Ok(board) => {
                    let (mut embed, mut rows) =
                        crate::commands::quests::ui::create_quest_board_embed(&board);
                    embed = embed.title("Tavern — Quest Offers");
                    // Prepend Home + Recruitment row
                    rows.insert(
                        0,
                        serenity::builder::CreateActionRow::Buttons(vec![
                            crate::ui::buttons::Btn::secondary(
                                crate::interactions::ids::SAGA_TAVERN_HOME,
                                "🏰 Tavern",
                            ),
                            crate::ui::buttons::Btn::primary(
                                crate::interactions::ids::SAGA_RECRUIT,
                                "🧭 Recruitment",
                            ),
                        ]),
                    );
                    // Insert Tavern-specific Saga row after the header row
                    rows.insert(1, crate::commands::saga::ui::tavern_saga_row());
                    edit_component(
                        ctx,
                        component,
                        "tavern.quests",
                        EditInteractionResponse::new().embed(embed).components(rows),
                    )
                    .await;
                }
                Err(e) => {
                    edit_component(
                        ctx,
                        component,
                        "tavern.quests.err",
                        EditInteractionResponse::new()
                            .content(format!("Failed to load quest board: {}", e)),
                    )
                    .await;
                }
            }
        }
        Some(&"tavern") if raw_id == crate::interactions::ids::SAGA_TAVERN_SHOP => {
            render_tavern_shop_view(ctx, component, &app_state, None).await;
        }
        Some(&"tavern")
            if raw_id.starts_with(crate::interactions::ids::SAGA_TAVERN_SHOP_BUY_PREFIX) =>
        {
            use crate::commands::economy::core::item::Item;
            use serenity::builder::{CreateActionRow, CreateEmbed};
            let id_str =
                raw_id.trim_start_matches(crate::interactions::ids::SAGA_TAVERN_SHOP_BUY_PREFIX);
            let Some(item_id) = id_str.parse::<i32>().ok() else {
                edit_component(
                    ctx,
                    component,
                    "tavern.shop.bad_id",
                    EditInteractionResponse::new().content("Invalid item id."),
                )
                .await;
                return;
            };
            let Some(item) = Item::from_i32(item_id) else {
                edit_component(
                    ctx,
                    component,
                    "tavern.shop.unknown_item",
                    EditInteractionResponse::new().content("Unknown item."),
                )
                .await;
                return;
            };
            // Price via centralized helper with fame-based discount (to match listing and charge)
            let balance = crate::database::economy::get_or_create_profile(db, component.user.id)
                .await
                .map(|p| p.balance)
                .unwrap_or(0);
            let (_, meta_tmp) = crate::commands::saga::tavern::build_tavern_state_cached(
                &app_state,
                component.user.id,
            )
            .await
            .unwrap_or((
                Vec::new(),
                crate::commands::saga::tavern::TavernUiMeta {
                    balance,
                    fame: 0,
                    fame_tier: 0,
                    fame_progress: 0.0,
                    daily_rerolls_used: 0,
                    max_daily_rerolls: crate::commands::saga::tavern::TAVERN_MAX_DAILY_REROLLS,
                    reroll_cost: crate::commands::saga::tavern::TAVERN_REROLL_COST,
                    can_reroll: false,
                },
            ));
            let (shop_disc, _, _) = crate::commands::saga::tavern::fame_perks(meta_tmp.fame_tier);
            let base = tavern_price(item).unwrap_or(1_000_000);
            let cost: i64 = crate::commands::saga::tavern::apply_shop_discount(base, shop_disc);
            let mut embed = CreateEmbed::new()
                .title("Confirm Purchase — Small Arms")
                .description(format!(
                    "Buy {} {} for {} {}?\n{}",
                    item.emoji(),
                    item.display_name(),
                    crate::ui::style::EMOJI_COIN,
                    cost,
                    item.properties().description
                ))
                .field(
                    "Your Balance",
                    format!("{} {}", crate::ui::style::EMOJI_COIN, balance),
                    true,
                )
                .color(crate::ui::style::COLOR_SAGA_TAVERN);
            if balance < cost {
                embed = embed.field("Note", "You cannot afford this.", false);
            }
            let buttons = vec![
                crate::ui::buttons::Btn::success(
                    &format!(
                        "{}{}",
                        crate::interactions::ids::SAGA_TAVERN_SHOP_BUY_CONFIRM_PREFIX,
                        item_id
                    ),
                    "✅ Confirm",
                )
                .disabled(balance < cost),
                crate::ui::buttons::Btn::secondary(
                    crate::interactions::ids::SAGA_TAVERN_SHOP_BUY_CANCEL,
                    "❌ Cancel",
                ),
            ];
            let mut rows = vec![CreateActionRow::Buttons(buttons)];
            rows.push(CreateActionRow::Buttons(vec![
                crate::ui::buttons::Btn::secondary(
                    crate::interactions::ids::SAGA_TAVERN_HOME,
                    "🏰 Tavern",
                ),
                crate::ui::buttons::Btn::primary(
                    crate::interactions::ids::SAGA_RECRUIT,
                    "🧭 Recruitment",
                ),
            ]));
            rows.push(crate::commands::saga::ui::tavern_saga_row());
            edit_component(
                ctx,
                component,
                "tavern.shop.confirm",
                EditInteractionResponse::new().embed(embed).components(rows),
            )
            .await;
        }
        Some(&"tavern") if raw_id == crate::interactions::ids::SAGA_TAVERN_SHOP_BUY_CANCEL => {
            component.data.custom_id = crate::interactions::ids::SAGA_TAVERN_SHOP.into();
            render_tavern_shop_view(ctx, component, &app_state, None).await;
        }
        Some(&"tavern")
            if raw_id
                .starts_with(crate::interactions::ids::SAGA_TAVERN_SHOP_BUY_CONFIRM_PREFIX) =>
        {
            use crate::commands::economy::core::item::Item;
            let id_str = raw_id
                .trim_start_matches(crate::interactions::ids::SAGA_TAVERN_SHOP_BUY_CONFIRM_PREFIX);
            let Some(item_id) = id_str.parse::<i32>().ok() else {
                edit_component(
                    ctx,
                    component,
                    "tavern.shopC.bad_id",
                    EditInteractionResponse::new().content("Invalid item id."),
                )
                .await;
                return;
            };
            let Some(item) = Item::from_i32(item_id) else {
                edit_component(
                    ctx,
                    component,
                    "tavern.shopC.unknown_item",
                    EditInteractionResponse::new().content("Unknown item."),
                )
                .await;
                return;
            };
            // Centralized price + fame discount
            let profile_balance =
                crate::database::economy::get_or_create_profile(db, component.user.id)
                    .await
                    .map(|p| p.balance)
                    .unwrap_or(0);
            let (_, meta_tmp) = crate::commands::saga::tavern::build_tavern_state_cached(
                &app_state,
                component.user.id,
            )
            .await
            .unwrap_or((
                Vec::new(),
                crate::commands::saga::tavern::TavernUiMeta {
                    balance: profile_balance,
                    fame: 0,
                    fame_tier: 0,
                    fame_progress: 0.0,
                    daily_rerolls_used: 0,
                    max_daily_rerolls: crate::commands::saga::tavern::TAVERN_MAX_DAILY_REROLLS,
                    reroll_cost: crate::commands::saga::tavern::TAVERN_REROLL_COST,
                    can_reroll: false,
                },
            ));
            let (shop_disc, _, _) = crate::commands::saga::tavern::fame_perks(meta_tmp.fame_tier);
            let base = tavern_price(item).unwrap_or(1_000_000);
            let cost: i64 = crate::commands::saga::tavern::apply_shop_discount(base, shop_disc);
            let mut tx = match db.begin().await {
                Ok(t) => t,
                Err(e) => {
                    edit_component(
                        ctx,
                        component,
                        "tavern.shopC.tx_err",
                        EditInteractionResponse::new()
                            .content(format!("Failed to start purchase: {}", e)),
                    )
                    .await;
                    return;
                }
            };
            let res = {
                match crate::database::economy::add_balance(&mut tx, component.user.id, -cost).await
                {
                    Ok(()) => {
                        crate::database::economy::add_to_inventory(
                            &mut tx,
                            component.user.id,
                            item,
                            1,
                        )
                        .await
                    }
                    Err(e) => Err(e),
                }
            };
            match res {
                Ok(()) => {
                    if tx.commit().await.is_err() {
                        edit_component(
                            ctx,
                            component,
                            "tavern.shopC.commit_err",
                            EditInteractionResponse::new().content("Purchase failed to commit."),
                        )
                        .await;
                        return;
                    }
                    // Re-render shop with success message via unified renderer
                    component.data.custom_id = crate::interactions::ids::SAGA_TAVERN_SHOP.into();
                    let notice = Some(format!(
                        "You bought {} {} for {} {}.",
                        item.emoji(),
                        item.display_name(),
                        crate::ui::style::EMOJI_COIN,
                        cost
                    ));
                    render_tavern_shop_view(ctx, component, &app_state, notice).await;
                }
                Err(_) => {
                    let _ = tx.rollback().await;
                    edit_component(
                        ctx,
                        component,
                        "tavern.shopC.fail",
                        EditInteractionResponse::new()
                            .content("Could not complete purchase (balance changed?)."),
                    )
                    .await;
                }
            }
        }
        // Removed pagination / filter handling branch
        Some(&"tavern") if raw_id == crate::interactions::ids::SAGA_TAVERN_REROLL => {
            use crate::commands::saga::tavern::{TAVERN_MAX_DAILY_REROLLS, TAVERN_REROLL_COST};
            let profile = crate::database::economy::get_or_create_profile(db, component.user.id)
                .await
                .ok();
            let balance = profile.as_ref().map(|p| p.balance).unwrap_or(0);
            let can_reroll_now = crate::database::tavern::can_reroll(
                db,
                component.user.id,
                TAVERN_MAX_DAILY_REROLLS,
            )
            .await
            .unwrap_or(false);
            let (_, meta) = crate::commands::saga::tavern::build_tavern_state_cached(
                &app_state,
                component.user.id,
            )
            .await
            .unwrap_or((
                Vec::new(),
                crate::commands::saga::tavern::TavernUiMeta {
                    balance,
                    fame: 0,
                    fame_tier: 0,
                    fame_progress: 0.0,
                    daily_rerolls_used: 0,
                    max_daily_rerolls: TAVERN_MAX_DAILY_REROLLS,
                    reroll_cost: TAVERN_REROLL_COST,
                    can_reroll: false,
                },
            ));
            let left = (meta.max_daily_rerolls - meta.daily_rerolls_used).max(0);
            let embed = serenity::builder::CreateEmbed::new()
                .title("Confirm Reroll")
                .description(format!(
                    "Spend {} {} to reshuffle your personal rotation. {} rerolls left today. Resets in {}.",
                    meta.reroll_cost,
                    crate::ui::style::EMOJI_COIN,
                    left,
                    crate::commands::saga::tavern::time_until_reset_str()
                ))
                .color(crate::ui::style::COLOR_SAGA_TAVERN);
            let embed = if !can_reroll_now || balance < meta.reroll_cost || left == 0 {
                embed.field("Note", "You cannot reroll right now.", false)
            } else {
                embed
            };
            let buttons = vec![
                crate::ui::buttons::Btn::danger(
                    crate::interactions::ids::SAGA_TAVERN_REROLL_CONFIRM,
                    "Confirm Reroll",
                )
                .disabled(!can_reroll_now || balance < meta.reroll_cost || left == 0),
                crate::ui::buttons::Btn::secondary(
                    crate::interactions::ids::SAGA_TAVERN_REROLL_CANCEL,
                    "Cancel",
                ),
            ];
            edit_component(
                ctx,
                component,
                "tavern.reroll.confirm",
                EditInteractionResponse::new().embed(embed).components(vec![
                    serenity::builder::CreateActionRow::Buttons(buttons),
                    crate::commands::saga::ui::tavern_saga_row(),
                ]),
            )
            .await;
        }
        Some(&"tavern") if raw_id == crate::interactions::ids::SAGA_TAVERN_REROLL_CANCEL => {
            // Restore tavern view using session state
            let (recruits, meta) = crate::commands::saga::tavern::build_tavern_state_cached(
                &app_state,
                component.user.id,
            )
            .await
            .unwrap_or((
                Vec::new(),
                crate::commands::saga::tavern::TavernUiMeta {
                    balance: 0,
                    fame: 0,
                    fame_tier: 0,
                    fame_progress: 0.0,
                    daily_rerolls_used: 0,
                    max_daily_rerolls: crate::commands::saga::tavern::TAVERN_MAX_DAILY_REROLLS,
                    reroll_cost: crate::commands::saga::tavern::TAVERN_REROLL_COST,
                    can_reroll: false,
                },
            ));
            let (embed, mut components) =
                crate::commands::saga::tavern::create_tavern_menu(&recruits, &meta);
            components.push(crate::commands::saga::ui::tavern_saga_row());
            edit_component(
                ctx,
                component,
                "tavern.reroll.cancel",
                EditInteractionResponse::new()
                    .embed(embed)
                    .components(components),
            )
            .await;
        }
        Some(&"tavern") if raw_id == crate::interactions::ids::SAGA_TAVERN_REROLL_CONFIRM => {
            use crate::commands::saga::tavern::TAVERN_MAX_DAILY_REROLLS;
            if let Ok(profile) =
                crate::database::economy::get_or_create_profile(db, component.user.id).await
            {
                // Use discounted reroll cost based on fame
                let (_, meta_now) = crate::commands::saga::tavern::build_tavern_state_cached(
                    &app_state,
                    component.user.id,
                )
                .await
                .unwrap_or((
                    Vec::new(),
                    crate::commands::saga::tavern::TavernUiMeta {
                        balance: profile.balance,
                        fame: 0,
                        fame_tier: 0,
                        fame_progress: 0.0,
                        daily_rerolls_used: 0,
                        max_daily_rerolls: TAVERN_MAX_DAILY_REROLLS,
                        reroll_cost: crate::commands::saga::tavern::TAVERN_REROLL_COST,
                        can_reroll: false,
                    },
                ));
                let reroll_cost_now = meta_now.reroll_cost;
                if profile.balance < reroll_cost_now {
                    edit_component(
                        ctx,
                        component,
                        "tavern.reroll.no_funds",
                        EditInteractionResponse::new().content("Not enough coins to reroll."),
                    )
                    .await;
                    return;
                }
                // Capture old rotation (for diff highlighting) before overwriting.
                let (old_units, _old_meta) =
                    crate::commands::saga::tavern::build_tavern_state_cached(
                        &app_state,
                        component.user.id,
                    )
                    .await
                    .unwrap_or((
                        Vec::new(),
                        crate::commands::saga::tavern::TavernUiMeta {
                            balance: profile.balance,
                            fame: 0,
                            fame_tier: 0,
                            fame_progress: 0.0,
                            daily_rerolls_used: 0,
                            max_daily_rerolls:
                                crate::commands::saga::tavern::TAVERN_MAX_DAILY_REROLLS,
                            reroll_cost: crate::commands::saga::tavern::TAVERN_REROLL_COST,
                            can_reroll: false,
                        },
                    ));
                let old_ids: std::collections::HashSet<i32> =
                    old_units.iter().map(|u| u.unit_id).collect();
                let global = crate::commands::saga::tavern::get_daily_recruits(db).await;
                let rotation: Vec<i32> = global.iter().map(|u| u.unit_id).collect();
                // Deterministic per-user-per-day shuffle using splitmix64 key ordering
                let today = chrono::Utc::now().date_naive();
                let seed = component.user.id.get()
                    ^ (((today.year() as u64) << 32) ^ (today.ordinal() as u64));
                let mut rotation = rotation.clone();
                rotation.sort_by(|a, b| {
                    let ka = crate::commands::saga::tavern::splitmix64(
                        seed ^ (*a as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15),
                    );
                    let kb = crate::commands::saga::tavern::splitmix64(
                        seed ^ (*b as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15),
                    );
                    ka.cmp(&kb).then_with(|| a.cmp(b))
                });
                // Perform atomic reroll (deduct, overwrite, increment counters)
                match crate::database::tavern::transactional_reroll(
                    db,
                    component.user.id,
                    &rotation,
                    reroll_cost_now,
                    TAVERN_MAX_DAILY_REROLLS,
                )
                .await
                {
                    Ok(()) => {}
                    Err(_) => {
                        edit_component(
                            ctx,
                            component,
                            "tavern.reroll.failed",
                            EditInteractionResponse::new().content(
                                "Cannot reroll right now (limit reached or balance changed).",
                            ),
                        )
                        .await;
                        return;
                    }
                }
                let (resolved_units, meta) =
                    crate::commands::saga::tavern::build_tavern_state_cached(
                        &app_state,
                        component.user.id,
                    )
                    .await
                    .unwrap_or((
                        Vec::new(),
                        crate::commands::saga::tavern::TavernUiMeta {
                            balance: profile.balance.saturating_sub(reroll_cost_now),
                            fame: 0,
                            fame_tier: 0,
                            fame_progress: 0.0,
                            daily_rerolls_used: 0,
                            max_daily_rerolls:
                                crate::commands::saga::tavern::TAVERN_MAX_DAILY_REROLLS,
                            reroll_cost: reroll_cost_now,
                            can_reroll: false,
                        },
                    ));
                let (mut embed, components) =
                    crate::commands::saga::tavern::create_tavern_menu(&resolved_units, &meta);
                // Diff highlighting: show new and removed units (based on full rotation, not filtered), truncated for brevity.
                let new_ids: std::collections::HashSet<i32> =
                    resolved_units.iter().map(|u| u.unit_id).collect();
                let added: Vec<_> = new_ids.difference(&old_ids).cloned().collect();
                let removed: Vec<_> = old_ids.difference(&new_ids).cloned().collect();
                if (!added.is_empty()) || (!removed.is_empty()) {
                    let mut added_list: Vec<String> = resolved_units
                        .iter()
                        .filter(|u| added.contains(&u.unit_id))
                        .map(|u| u.name.to_string())
                        .take(5)
                        .collect();
                    if added.len() > added_list.len() {
                        added_list.push("…".into());
                    }
                    let mut removed_list: Vec<String> = old_units
                        .iter()
                        .filter(|u| removed.contains(&u.unit_id))
                        .map(|u| u.name.to_string())
                        .take(5)
                        .collect();
                    if removed.len() > removed_list.len() {
                        removed_list.push("…".into());
                    }
                    let mut diff_lines = Vec::new();
                    if !added.is_empty() {
                        diff_lines.push(format!("➕ {} new", added.len()));
                    }
                    if !removed.is_empty() {
                        diff_lines.push(format!("➖ {} removed", removed.len()));
                    }
                    let summary = diff_lines.join(" • ");
                    let detail = format!(
                        "Added: {}\nRemoved: {}",
                        if added_list.is_empty() {
                            "-".into()
                        } else {
                            added_list.join(", ")
                        },
                        if removed_list.is_empty() {
                            "-".into()
                        } else {
                            removed_list.join(", ")
                        }
                    );
                    embed = embed.field(
                        "Rotation Changes",
                        format!("{}\n{}", summary, detail),
                        false,
                    );
                }
                edit_component(
                    ctx,
                    component,
                    "tavern.reroll.ok",
                    EditInteractionResponse::new()
                        .embed(embed)
                        .components(components),
                )
                .await;
            }
        }
        Some(_) if crate::interactions::ids::is_saga_area(raw_id) => {
            // Area switch: push MapArea view onto stack (persistent) and render
            let area_id = match raw_id
                .trim_start_matches(crate::interactions::ids::SAGA_AREA_PREFIX)
                .parse::<i32>()
            {
                Ok(v) => v,
                Err(_) => {
                    edit_component(
                        ctx,
                        component,
                        "area.bad_id",
                        EditInteractionResponse::new().content("Invalid area id"),
                    )
                    .await;
                    return;
                }
            };
            if let Ok((embed, mut comps)) = crate::saga::view::push_and_render(
                crate::saga::view::SagaView::MapArea(area_id),
                &app_state,
                component.user.id,
                MAX_NAV_DEPTH,
            )
            .await
            {
                let depth = app_state
                    .nav_stacks
                    .read()
                    .await
                    .get(&component.user.id.get())
                    .map(|s| s.stack.len())
                    .unwrap_or(1);
                crate::commands::saga::ui::insert_back_before_nav(&mut comps, depth, "saga");
                edit_component(
                    ctx,
                    component,
                    "area.switch",
                    EditInteractionResponse::new()
                        .embed(embed)
                        .components(comps),
                )
                .await;
            }
        }
        Some(&suffix) if crate::interactions::ids::is_saga_node(raw_id) || suffix == "node" => {
            // Support both new full custom_id (saga_node_<id>) and legacy split form (saga_node_<id>) already parsed.
            let node_id = if crate::interactions::ids::is_saga_node(raw_id) {
                match raw_id
                    .trim_start_matches(crate::interactions::ids::SAGA_NODE_PREFIX)
                    .parse::<i32>()
                {
                    Ok(v) => v,
                    Err(_) => {
                        edit_component(
                            ctx,
                            component,
                            "node.bad_id",
                            EditInteractionResponse::new()
                                .content("Error: Invalid battle node ID format."),
                        )
                        .await;
                        return;
                    }
                }
            } else if let Some(id_str) = custom_id_parts.get(2) {
                if let Ok(id) = id_str.parse::<i32>() {
                    id
                } else {
                    edit_component(
                        ctx,
                        component,
                        "node.bad_id",
                        EditInteractionResponse::new()
                            .content("Error: Invalid battle node ID format."),
                    )
                    .await;
                    return;
                }
            } else {
                edit_component(
                    ctx,
                    component,
                    "node.missing_id",
                    EditInteractionResponse::new().content("Error: Missing battle node ID."),
                )
                .await;
                return;
            };

            // Ensure player has a valid party before spending AP
            let player_party_units =
                match database::units::get_user_party(db, component.user.id).await {
                    Ok(units) if !units.is_empty() => units,
                    _ => {
                        edit_component(
                            ctx,
                            component,
                            "node.no_party",
                            EditInteractionResponse::new()
                                .content("You cannot start a battle without an active party!"),
                        )
                        .await;
                        return;
                    }
                };

            // Spend AP last so failures above don't consume it
            if let Ok(true) = database::saga::spend_action_points(db, component.user.id, 1).await {
                let (node_data, enemies, _rewards) =
                    match database::world::get_full_node_bundle(db, node_id).await {
                        Ok(bundle) => bundle,
                        Err(_) => {
                            edit_component(
                                ctx,
                                component,
                                "node.bundle_err",
                                EditInteractionResponse::new()
                                    .content("Error: Could not load node data."),
                            )
                            .await;
                            return;
                        }
                    };
                // Cached equipment bonuses via generic TTL helper
                let bonuses = if let Some(map) = cache_service::get_with_ttl(
                    &app_state.bonus_cache,
                    &component.user.id.get(),
                    Duration::from_secs(EQUIP_BONUS_CACHE_TTL_SECS),
                )
                .await
                {
                    map
                } else {
                    let fresh = database::units::get_equipment_bonuses(db, component.user.id)
                        .await
                        .unwrap_or_default();
                    cache_service::insert(
                        &app_state.bonus_cache,
                        component.user.id.get(),
                        fresh.clone(),
                    )
                    .await;
                    fresh
                };
                let mut synergy_log: Vec<String> = Vec::new();
                let player_units: Vec<BattleUnit> = player_party_units
                    .iter()
                    .map(|u| {
                        if let Some(b) = bonuses.get(&u.player_unit_id) {
                            if b.0 > 0 || b.1 > 0 || b.2 > 0 {
                                synergy_log.push(format!(
                                    "🔗 {} gains +{} Atk / +{} Def / +{} HP from bonded unit(s).",
                                    u.nickname.as_deref().unwrap_or(&u.name),
                                    b.0,
                                    b.1,
                                    b.2
                                ));
                            }
                            BattleUnit::from_player_unit_with_bonus(u, *b)
                        } else {
                            BattleUnit::from_player_unit(u)
                        }
                    })
                    .collect();
                // Dynamic enemy scaling: if player's story progress greatly exceeds node requirement, slightly buff enemies
                let player_story = crate::database::saga::get_story_progress(db, component.user.id)
                    .await
                    .unwrap_or(0);
                let diff = player_story.saturating_sub(node_data.story_progress_required);
                let (atk_scale, def_scale, hp_scale) = if diff >= 6 {
                    (1.25, 1.25, 1.35)
                } else if diff >= 3 {
                    (1.15, 1.10, 1.20)
                } else if diff <= -3 {
                    // Player under-leveled: small nerf to enemies
                    (0.90, 0.95, 0.90)
                } else {
                    (1.0, 1.0, 1.0)
                };
                let enemy_units: Vec<BattleUnit> = enemies
                    .iter()
                    .map(|u| {
                        let mut b = BattleUnit::from_unit(u);
                        b.attack = ((b.attack as f32) * atk_scale).round() as i32;
                        b.defense = ((b.defense as f32) * def_scale).round() as i32;
                        b.max_hp = ((b.max_hp as f32) * hp_scale).round() as i32;
                        b.current_hp = b.max_hp;
                        b
                    })
                    .collect();
                let mut session = BattleSession::new(player_units, enemy_units);
                session.log.extend(synergy_log);
                let can_afford_recruit = database::units::can_afford_recruit(db, component.user.id)
                    .await
                    .unwrap_or(false);
                let battle_game = BattleGame {
                    session,
                    party_members: player_party_units,
                    node_id,
                    node_name: node_data.name,
                    can_afford_recruit,
                    player_quest_id: None,
                    claimed: false,
                };
                let (content, embed, components) = battle_game.render();
                let builder = EditInteractionResponse::new()
                    .content(content)
                    .embed(embed)
                    .components(components);
                if let Ok(msg) = component.edit_response(&ctx.http, builder).await {
                    app_state
                        .game_manager
                        .write()
                        .await
                        .start_game(msg.id, Box::new(battle_game));
                }
            } else {
                edit_component(
                    ctx,
                    component,
                    "node.no_ap",
                    EditInteractionResponse::new().embed(error_embed(
                        "Not Enough Action Points",
                        "You need more AP to start this battle. Come back after they recharge.",
                    )),
                )
                .await;
            }
        }
        Some(&"main") => {
            match push_and_render(SagaView::Root, &app_state, component.user.id, MAX_NAV_DEPTH)
                .await
            {
                Ok((embed, components)) => {
                    edit_component(
                        ctx,
                        component,
                        "main.render",
                        EditInteractionResponse::new()
                            .embed(embed)
                            .components(components),
                    )
                    .await;
                }
                Err(e) => {
                    edit_component(
                        ctx,
                        component,
                        "main.err",
                        EditInteractionResponse::new()
                            .content(format!("Error: Could not refresh saga root ({e})")),
                    )
                    .await;
                }
            }
        }
        Some(&"tutorial") => {
            match custom_id_parts.get(2) {
                Some(&"hire") => {
                    // Give a free starter unit (unit_id 1 assumed) if player has none
                    let has_any = database::units::get_player_units(db, component.user.id)
                        .await
                        .map(|v| !v.is_empty())
                        .unwrap_or(false);
                    if has_any {
                        edit_component(
                            ctx,
                            component,
                            "tutorial.hire_skip",
                            EditInteractionResponse::new()
                                .content("You already have a unit. Tutorial reward skipped."),
                        )
                        .await;
                        return;
                    }
                    // Insert starter without cost
                    let user_id_i64 = component.user.id.get() as i64;
                    let starter_id = *app_state.starter_unit_id.read().await;
                    if let Err(e) = sqlx::query!("INSERT INTO player_units (user_id, unit_id, nickname, current_level, current_xp, current_attack, current_defense, current_health, is_in_party, rarity) SELECT $1, u.unit_id, u.name, 1, 0, u.base_attack, u.base_defense, u.base_health, TRUE, u.rarity FROM units u WHERE u.unit_id = $2 ON CONFLICT DO NOTHING", user_id_i64, starter_id).execute(db).await {
                        edit_component(ctx, component, "tutorial.hire_err", EditInteractionResponse::new().content(format!("Failed to grant starter unit: {}", e))).await;
                        return;
                    }
                    // Refresh saga profile and show main menu
                    let (embed, components) = match push_and_render(
                        SagaView::Root,
                        &app_state,
                        component.user.id,
                        MAX_NAV_DEPTH,
                    )
                    .await
                    {
                        Ok(ec) => ec,
                        Err(e) => {
                            edit_component(
                                ctx,
                                component,
                                "tutorial.refresh_err",
                                EditInteractionResponse::new()
                                    .content(format!("Failed to refresh saga root: {}", e)),
                            )
                            .await;
                            return;
                        }
                    };
                    edit_component(
                        ctx,
                        component,
                        "tutorial.hire_ok",
                        EditInteractionResponse::new()
                            .content("Starter unit recruited and added to your party!")
                            .embed(embed)
                            .components(components),
                    )
                    .await;
                }
                Some(&"skip") => {
                    // Just show main menu (will still be disabled map until a recruit happens)
                    let (embed, components) = match push_and_render(
                        SagaView::Root,
                        &app_state,
                        component.user.id,
                        MAX_NAV_DEPTH,
                    )
                    .await
                    {
                        Ok(ec) => ec,
                        Err(e) => {
                            edit_component(
                                ctx,
                                component,
                                "tutorial.skip_refresh_err",
                                EditInteractionResponse::new()
                                    .content(format!("Failed to refresh saga root: {}", e)),
                            )
                            .await;
                            return;
                        }
                    };
                    edit_component(
                        ctx,
                        component,
                        "tutorial.skip_ok",
                        EditInteractionResponse::new()
                            .content("Tutorial skipped.")
                            .embed(embed)
                            .components(components),
                    )
                    .await;
                }
                _ => {
                    edit_component(
                        ctx,
                        component,
                        "tutorial.unknown",
                        EditInteractionResponse::new().content("Unknown tutorial action."),
                    )
                    .await;
                }
            }
        }
        _ => {
            edit_component(
                ctx,
                component,
                "unknown",
                EditInteractionResponse::new().content("Unknown saga interaction."),
            )
            .await;
        }
    }
}
