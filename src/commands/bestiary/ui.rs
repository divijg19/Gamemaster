use crate::database::models::Unit;
use crate::ui::style::pad_label;
use serenity::builder::{CreateActionRow, CreateButton, CreateEmbed};
use serenity::model::application::ButtonStyle;

pub struct BestiaryEntry {
    pub unit: Unit,
    pub defeated: i64,
    pub research_owned: i64,
}

pub fn create_bestiary_embed(entries: &[BestiaryEntry]) -> (CreateEmbed, Vec<CreateActionRow>) {
    let mut embed = CreateEmbed::new()
        .title("📚 Bestiary")
        .description(
            "Catalog of creatures you've encountered. Defeat and gather research to learn more.",
        )
        .color(0x556B2F);
    if entries.is_empty() {
        embed = embed
            .description("You haven't encountered any creatures yet. Explore the world to begin.");
    }
    for e in entries.iter().take(25) {
        // cap to keep embed size safe
        let rarity = format!("{:?}", e.unit.rarity);
        let progress = format!("Defeated: {} | Research: {}", e.defeated, e.research_owned);
        embed = embed.field(format!("{} ({})", e.unit.name, rarity), progress, false);
    }
    (
        embed,
        vec![
            crate::commands::saga::ui::play_button_row(&crate::ui::style::pad_label(
                "Play / Menu",
                14,
            )),
            CreateActionRow::Buttons(vec![
                CreateButton::new("bestiary_refresh")
                    .label(pad_label("🔄 Refresh", 14))
                    .style(ButtonStyle::Secondary),
                CreateButton::new("contracts_refresh")
                    .label(pad_label("📜 Contracts", 16))
                    .style(ButtonStyle::Secondary),
                CreateButton::new("research_refresh")
                    .label(pad_label("🔬 Research", 16))
                    .style(ButtonStyle::Secondary),
            ]),
        ],
    )
}
