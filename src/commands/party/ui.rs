//! Handles the UI creation for the `/party` command.

use crate::constants::rarity_icon;
use crate::ui::style::pad_label;
use crate::constants::{BOND_MAP_CACHE_TTL_SECS, EQUIP_BONUS_CACHE_TTL_SECS};
use crate::database::models::{PlayerUnit, UnitRarity};
use crate::model::AppState;
use crate::model::{BondedEquippablesMap, EquipmentBonusMap};
use crate::services::cache as cache_service;
use serenity::builder::{
    CreateActionRow, CreateEmbed, CreateEmbedFooter, CreateSelectMenu, CreateSelectMenuKind,
    CreateSelectMenuOption,
};
use serenity::model::id::UserId;
use sqlx::PgPool;
// HashMap imported via type aliases in crate::model
use std::time::Duration;

/// Creates the main embed and components for the party and army management view.
pub fn create_party_view(units: &[PlayerUnit]) -> (CreateEmbed, Vec<CreateActionRow>) {
    let mut embed = CreateEmbed::new()
        .title("Party & Army Management")
        .description(
            "Your **Party** is your active combat team. Your **Army** is all units you own.",
        )
        .footer(CreateEmbedFooter::new(format!(
            "Total Army Size: {}/10",
            units.len()
        )))
        .color(0x3498DB);

    if units.is_empty() {
        embed = embed.description(
            "Your army is empty! Visit the Tavern in the `/saga` menu to hire your first units.",
        );
        return (
            embed,
            vec![crate::commands::saga::ui::play_button_row(&pad_label("Play / Menu", 14))],
        );
    }

    let party: Vec<_> = units.iter().filter(|p| p.is_in_party).collect();
    let reserves: Vec<_> = units.iter().filter(|p| !p.is_in_party).collect();

    let party_list = if party.is_empty() {
        "Your active party is empty. Add members from your reserves!".to_string()
    } else {
        party
            .iter()
            .map(|p| format_pet_line(p))
            .collect::<Vec<_>>()
            .join("\n")
    };
    embed = embed.field(
        format!("⚔️ Active Party ({}/5)", party.len()),
        party_list,
        false,
    );

    if !reserves.is_empty() {
        let reserve_list = reserves
            .iter()
            .map(|p| format_pet_line(p))
            .collect::<Vec<_>>()
            .join("\n");
        embed = embed.field("🛡️ Reserves", reserve_list, false);
    }

    embed = embed.field("🔗 Bonding Legend", "Bond bonuses are applied automatically in battles. Use the Bond Management button to equip or unequip special units.", false);

    let mut components = Vec::new();

    let add_options: Vec<_> = reserves
        .iter()
        .map(|p| {
            let pet_name = p.nickname.as_deref().unwrap_or(&p.name);
            CreateSelectMenuOption::new(pet_name, p.player_unit_id.to_string())
        })
        .collect();

    if !add_options.is_empty() && party.len() < 5 {
        let menu = CreateSelectMenu::new(
            "party_add",
            CreateSelectMenuKind::String {
                options: add_options,
            },
        )
        .placeholder("Add a unit to your party...");
        components.push(CreateActionRow::SelectMenu(menu));
    }

    let remove_options: Vec<_> = party
        .iter()
        .map(|p| {
            let pet_name = p.nickname.as_deref().unwrap_or(&p.name);
            CreateSelectMenuOption::new(pet_name, p.player_unit_id.to_string())
        })
        .collect();

    if !remove_options.is_empty() {
        let menu = CreateSelectMenu::new(
            "party_remove",
            CreateSelectMenuKind::String {
                options: remove_options,
            },
        )
        .placeholder("Remove a unit from your party...");
        components.push(CreateActionRow::SelectMenu(menu));
    }

    // Dropdown for dismissing units (party members and reserves).
    if !units.is_empty() {
        let dismiss_options: Vec<_> = units
            .iter()
            .map(|p| {
                let pet_name = p.nickname.as_deref().unwrap_or(&p.name);
                CreateSelectMenuOption::new(pet_name, p.player_unit_id.to_string())
            })
            .collect();

        let menu = CreateSelectMenu::new(
            "party_dismiss",
            CreateSelectMenuKind::String {
                options: dismiss_options,
            },
        )
        .placeholder("Dismiss a unit from your army...");
        components.push(CreateActionRow::SelectMenu(menu));
        // Add a bond management button row (links to /bond command UI via interaction custom id route)
        components.push(CreateActionRow::Buttons(vec![
            serenity::builder::CreateButton::new("bond_open")
                .label(pad_label("🔗 Manage Bonds", 20))
                .style(serenity::model::application::ButtonStyle::Secondary),
        ]));
    }

    // Prepend Play row
    let mut rows = vec![crate::commands::saga::ui::play_button_row(&pad_label("Play / Menu", 14))];
    // Global navigation row (active = party)
    crate::commands::saga::ui::add_nav(&mut rows, "party");
    rows.extend(components);
    (embed, rows)
}

/// Helper function to format a single line for a unit's display.
fn format_pet_line(unit: &PlayerUnit) -> String {
    let training_status = if unit.is_training {
        if let Some(ends_at) = unit.training_ends_at {
            let timestamp = format!("<t:{}:R>", ends_at.timestamp());
            let stat = unit.training_stat.as_deref().unwrap_or("stat");
            format!("(💪 {} ends {})", stat, timestamp)
        } else {
            "(💪 Training)".to_string()
        }
    } else {
        "".to_string()
    };

    let unit_name = unit.nickname.as_deref().unwrap_or(&unit.name);

    let rarity_icon = rarity_icon(unit.rarity);

    format!(
        "{} **{}** | Lvl {} (`{}` XP) | Atk: {} | Def: {} | HP: {} {}",
        rarity_icon,
        unit_name,
        unit.current_level,
        unit.current_xp,
        unit.current_attack,
        unit.current_defense,
        unit.current_health,
        training_status
    )
    .trim()
    .to_string()
}

// Extended async helper (not used directly here yet) to build bonded mapping for future caching.
pub async fn fetch_bonded_mapping(
    pool: &PgPool,
    user_id: UserId,
) -> sqlx::Result<BondedEquippablesMap> {
    use std::collections::HashMap;
    let rows = sqlx::query!(r#"SELECT b.host_player_unit_id, pu.player_unit_id, COALESCE(pu.nickname, u.name) as equipped_name, pu.rarity as "rarity: UnitRarity" FROM equippable_unit_bonds b JOIN player_units pu ON pu.player_unit_id = b.equipped_player_unit_id JOIN units u ON u.unit_id = pu.unit_id WHERE pu.user_id = $1 AND b.is_equipped = TRUE"#, user_id.get() as i64).fetch_all(pool).await?;
    let mut map: BondedEquippablesMap = HashMap::new();
    for r in rows {
        map.entry(r.host_player_unit_id).or_default().push((
            r.player_unit_id,
            r.equipped_name.unwrap_or_else(|| "(Unnamed)".into()),
            r.rarity,
        ));
    }
    Ok(map)
}

/// Async variant that enriches lines with bonded equippables underneath each host.
pub async fn create_party_view_with_bonds(
    app_state: &AppState,
    user_id: UserId,
) -> (CreateEmbed, Vec<CreateActionRow>) {
    let pool: &PgPool = &app_state.db;
    let units = sqlx::query_as!(
        crate::database::models::PlayerUnit,
        r#"SELECT
        pu.player_unit_id, pu.user_id, pu.unit_id, pu.nickname, pu.current_level, pu.current_xp,
        pu.current_attack, pu.current_defense, pu.current_health, pu.is_in_party, pu.is_training,
        pu.training_stat, pu.training_ends_at, u.name, pu.rarity as "rarity: UnitRarity"
        FROM player_units pu JOIN units u ON pu.unit_id = u.unit_id
        WHERE pu.user_id = $1 ORDER BY pu.is_in_party DESC, pu.current_level DESC"#,
        user_id.get() as i64
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // Start from base party view (gives us menus & basic embed)
    let (mut embed, mut components) = create_party_view(&units);

    // Early exit if empty army already handled by base view
    if units.is_empty() {
        components.push(crate::commands::saga::ui::global_nav_row("party"));
        return (embed, components);
    }

    // Fetch bond & equipment bonus maps in parallel (each may hit cache or DB)
    let uid = user_id.get();
    let (bond_map, bonuses_map): (BondedEquippablesMap, EquipmentBonusMap) = tokio::join!(
        async {
            if let Some(m) = cache_service::get_with_ttl(
                &app_state.bond_cache,
                &uid,
                Duration::from_secs(BOND_MAP_CACHE_TTL_SECS),
            )
            .await
            {
                m
            } else {
                let fresh = fetch_bonded_mapping(pool, user_id)
                    .await
                    .unwrap_or_default();
                cache_service::insert(&app_state.bond_cache, uid, fresh.clone()).await;
                fresh
            }
        },
        async {
            if let Some(m) = cache_service::get_with_ttl(
                &app_state.bonus_cache,
                &uid,
                Duration::from_secs(EQUIP_BONUS_CACHE_TTL_SECS),
            )
            .await
            {
                m
            } else {
                let fresh = crate::database::units::get_equipment_bonuses(pool, user_id)
                    .await
                    .unwrap_or_default();
                cache_service::insert(&app_state.bonus_cache, uid, fresh.clone()).await;
                fresh
            }
        }
    );

    // Build enhanced party lines (replace existing Active Party field)
    let mut party_lines: Vec<String> = Vec::new();
    for p in units.iter().filter(|u| u.is_in_party) {
        let mut line = format_pet_line(p);
        if let Some(b) = bonuses_map.get(&p.player_unit_id)
            && (b.0 > 0 || b.1 > 0 || b.2 > 0)
        {
            line.push_str(&format!(" (+{} Atk / +{} Def / +{} HP)", b.0, b.1, b.2));
        }
        if let Some(eqs) = bond_map.get(&p.player_unit_id) {
            // Compact representation: group by rarity, show counts & first names
            if !eqs.is_empty() {
                // Map rarity -> Vec<name>
                use std::collections::BTreeMap;
                let mut grouped: BTreeMap<UnitRarity, Vec<&str>> = BTreeMap::new();
                for (_, name, rarity) in eqs {
                    grouped.entry(*rarity).or_default().push(name);
                }
                for (rarity, names) in grouped {
                    if names.len() == 1 {
                        line.push_str(&format!(
                            "\n   └─ {} Bonded: {}",
                            rarity_icon(rarity),
                            names[0]
                        ));
                    } else {
                        let preview = names.iter().take(2).cloned().collect::<Vec<_>>().join(", ");
                        let more = if names.len() > 2 {
                            format!(" +{} more", names.len() - 2)
                        } else {
                            String::new()
                        };
                        line.push_str(&format!(
                            "\n   └─ {} {} bonded ({}{})",
                            rarity_icon(rarity),
                            names.len(),
                            preview,
                            more
                        ));
                    }
                }
            }
        }
        party_lines.push(line);
    }
    if !party_lines.is_empty() {
        // Rebuild embed (simpler than editing in place)
        let reserves: Vec<_> = units.iter().filter(|p| !p.is_in_party).collect();
        // Aggregate bonus summary
        let total_bonus = bonuses_map
            .values()
            .fold((0, 0, 0), |acc, b| (acc.0 + b.0, acc.1 + b.1, acc.2 + b.2));
        let bonus_line = if total_bonus != (0, 0, 0) {
            format!(
                "**Total Bond Bonuses:** +{} Atk / +{} Def / +{} HP\n\n",
                total_bonus.0, total_bonus.1, total_bonus.2
            )
        } else {
            String::new()
        };
        embed = CreateEmbed::new()
            .title("Party & Army Management")
            .description(format!(
                "{}Your **Party** is your active combat team. Your **Army** is all units you own.",
                bonus_line
            ))
            .footer(CreateEmbedFooter::new(format!(
                "Total Army Size: {}/10",
                units.len()
            )))
            .color(0x3498DB)
            .field(
                format!("⚔️ Active Party ({}/5)", party_lines.len()),
                party_lines.join("\n"),
                false,
            );
        if !reserves.is_empty() {
            let reserve_list = reserves
                .iter()
                .map(|p| format_pet_line(p))
                .collect::<Vec<_>>()
                .join("\n");
            embed = embed.field("🛡️ Reserves", reserve_list, false);
        }
        embed = embed.field("🔗 Bonding Legend", "Bond bonuses are applied automatically in battles. Use the Bond Management button to equip or unequip special units.", false);
    }
    (embed, components)
}
