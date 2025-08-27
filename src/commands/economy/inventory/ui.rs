//! Handles all UI and embed creation for the `/inventory` command.

use crate::commands::economy::core::item::{Item, Rarity};
use crate::database;
use serenity::builder::CreateEmbed;
use serenity::model::user::User;
use std::str::FromStr;

pub fn create_inventory_embed(
    user: &User,
    inventory_result: Result<Vec<database::profile::InventoryItem>, sqlx::Error>,
) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title(format!("{}'s Inventory", user.name))
        .thumbnail(user.face());

    match inventory_result {
        Ok(inventory) => {
            if inventory.is_empty() {
                embed = embed
                    .description("Nothing to see here! Use `/work` to get some!")
                    .color(0x5865F2);
            } else {
                // (✓) ALIVE: The Rarity::color() method is now used here.
                // Find the highest rarity item in the inventory to set the embed color.
                let highest_rarity = inventory
                    .iter()
                    .filter_map(|db_item| Item::from_str(&db_item.name).ok())
                    .map(|item_enum| item_enum.properties().rarity)
                    .max()
                    .unwrap_or(Rarity::Common);

                let items_list = inventory
                    .iter()
                    .map(|db_item| {
                        if let Ok(item_enum) = Item::from_str(&db_item.name) {
                            let props = item_enum.properties();
                            format!(
                                "{} **{}** `x{}`\n*{} Rarity*",
                                props.emoji,
                                db_item.name,
                                db_item.quantity,
                                props.rarity.as_str()
                            )
                        } else {
                            format!(
                                "❔ **{}** `x{}`\n*Unknown Rarity*",
                                db_item.name, db_item.quantity
                            )
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n");

                embed = embed.description(items_list).color(highest_rarity.color());
            }
        }
        Err(_) => {
            embed = embed
                .color(0xFF0000)
                .description("Could not retrieve inventory data.");
        }
    }

    embed
}
