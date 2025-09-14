use crate::database::models::Unit;
use crate::ui::buttons::Btn;
use serenity::builder::{CreateActionRow, CreateEmbed};

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
            crate::commands::saga::ui::global_nav_row("saga"),
            CreateActionRow::Buttons(vec![
                Btn::secondary("bestiary_refresh", "🔄 Refresh"),
                Btn::secondary("contracts_refresh", "📜 Contracts"),
                Btn::secondary("research_refresh", "🔬 Research"),
            ]),
        ],
    )
}
