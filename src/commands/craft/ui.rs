//! Handles the UI creation for the `/craft` command.

use crate::commands::economy::core::item::Item;
use crate::database::models::{InventoryItem, Recipe, RecipeIngredient};
use serenity::builder::{
    CreateActionRow, CreateEmbed, CreateEmbedFooter, CreateSelectMenu, CreateSelectMenuKind,
    CreateSelectMenuOption,
};
use std::collections::HashMap;

/// A helper struct to hold all the data needed to render a recipe.
pub struct RecipeInfo<'a> {
    pub recipe: &'a Recipe,
    pub ingredients: &'a [RecipeIngredient],
    pub output_item: Item,
}

/// Creates the main embed and components for the crafting menu.
pub fn create_crafting_menu(
    recipes: &[RecipeInfo],
    inventory: &[InventoryItem],
) -> (CreateEmbed, Vec<CreateActionRow>) {
    let mut embed = CreateEmbed::new()
        .title("Crafting Workshop")
        .description("Combine materials to create powerful new items.")
        .color(0x964B00); // Brown

    if recipes.is_empty() {
        embed = embed.description("There are no crafting recipes available at the moment.");
        return (embed, vec![]);
    }

    // Use a HashMap for efficient inventory lookups.
    let inventory_map: HashMap<i32, i64> = inventory
        .iter()
        .filter_map(|inv_item| {
            if let Ok(item_enum) = inv_item.name.parse::<Item>() {
                Some((item_enum as i32, inv_item.quantity))
            } else {
                None
            }
        })
        .collect();

    let mut select_options = Vec::new();

    for recipe_info in recipes {
        let ingredients_str = recipe_info
            .ingredients
            .iter()
            .map(|ing| {
                let required_item = Item::from_i32(ing.item_id).unwrap();
                let owned_qty = inventory_map.get(&ing.item_id).copied().unwrap_or(0);
                format!(
                    "`{}/{}` {}",
                    owned_qty,
                    ing.quantity,
                    required_item.display_name()
                )
            })
            .collect::<Vec<_>>()
            .join(", ");

        let can_craft = recipe_info.ingredients.iter().all(|ing| {
            inventory_map.get(&ing.item_id).copied().unwrap_or(0) >= ing.quantity as i64
        });

        let craft_status_emoji = if can_craft { "✅" } else { "❌" };

        embed = embed.field(
            format!(
                "{} {} {}",
                craft_status_emoji,
                recipe_info.output_item.emoji(),
                recipe_info.output_item.display_name()
            ),
            format!("Requires: {}", ingredients_str),
            false,
        );

        // Only add recipes the player can afford to the dropdown menu.
        if can_craft {
            select_options.push(CreateSelectMenuOption::new(
                recipe_info.output_item.display_name(),
                recipe_info.recipe.recipe_id.to_string(),
            ));
        }
    }

    let mut components = Vec::new();
    if !select_options.is_empty() {
        let menu = CreateSelectMenu::new(
            "craft_select_recipe",
            CreateSelectMenuKind::String {
                options: select_options,
            },
        )
        .placeholder("Select an item to craft...");
        components.push(CreateActionRow::SelectMenu(menu));
    } else {
        // Show footer only when nothing can be crafted
        embed = embed.footer(CreateEmbedFooter::new(
            "You don't have the required materials to craft anything.",
        ));
    }

    (embed, components)
}
