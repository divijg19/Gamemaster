//! UI helpers for the bonding system.

use crate::database::models::PlayerUnit;
use serenity::builder::{CreateActionRow, CreateEmbed, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption};

pub fn create_bond_select(hosts: &[PlayerUnit], candidates: &[PlayerUnit]) -> (CreateEmbed, Vec<CreateActionRow>) {
    let mut embed = CreateEmbed::new()
        .title("Unit Bonding")
        .description("Select a host unit (eligible rarity) and an equippable unit (equal or lower rarity) to bond. Only ONE equipped unit per host. Use future *Unequip* button to detach without destroying history.")
        .color(0x9B59B6);

    if hosts.is_empty() || candidates.is_empty() {
        embed = embed.description("You need at least one potential host and one candidate unit.");
        return (embed, vec![]);
    }

    let host_opts = hosts.iter().map(|u| {
        let name = u.nickname.as_deref().unwrap_or(&u.name);
        CreateSelectMenuOption::new(format!("{} (R{:?})", name, u.rarity), u.player_unit_id.to_string())
    }).collect();

    // Initial host select uses dynamic custom_id per selection via handler reinterpretation; keep static id with colon suffix for consistency.
    let host_menu = CreateSelectMenu::new("bond_host:", CreateSelectMenuKind::String { options: host_opts }).placeholder("Select host unit...");
    // Without a chosen host we don't yet know host_id; bonding handler will instruct user and re-render.

    (embed, vec![CreateActionRow::SelectMenu(host_menu)])
}
