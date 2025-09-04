//! Contains all database functions related to the crafting system.

use super::economy::{add_to_inventory, get_inventory_item};
use super::models::{Recipe, RecipeIngredient};
use crate::commands::economy::core::item::Item;
use serenity::model::id::UserId;
use sqlx::PgPool;

/// Fetches all available crafting recipes from the database.
pub async fn get_all_recipes(pool: &PgPool) -> Result<Vec<Recipe>, sqlx::Error> {
    sqlx::query_as!(Recipe, "SELECT * FROM recipes ORDER BY recipe_id")
        .fetch_all(pool)
        .await
}

/// Fetches all ingredients required for a specific recipe.
pub async fn get_ingredients_for_recipe(
    pool: &PgPool,
    recipe_id: i32,
) -> Result<Vec<RecipeIngredient>, sqlx::Error> {
    sqlx::query_as!(
        RecipeIngredient,
        "SELECT item_id, quantity FROM recipe_ingredients WHERE recipe_id = $1",
        recipe_id
    )
    .fetch_all(pool)
    .await
}

/// A transaction to craft an item.
/// Checks for ingredients, consumes them, and adds the crafted item to the inventory.
/// Returns Ok(output_item) on success or Err(reason_string) on failure.
pub async fn craft_item(pool: &PgPool, user_id: UserId, recipe_id: i32) -> Result<Item, String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    // 1. Get the recipe details.
    let recipe = sqlx::query_as!(
        Recipe,
        "SELECT * FROM recipes WHERE recipe_id = $1",
        recipe_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| "That recipe does not exist.".to_string())?;

    let ingredients = get_ingredients_for_recipe(pool, recipe_id)
        .await
        .map_err(|_| "Could not fetch recipe ingredients.".to_string())?;

    // 2. Check if the player has all the required ingredients.
    for ingredient in &ingredients {
        let required_item =
            Item::from_i32(ingredient.item_id).ok_or("Invalid item ID in recipe.")?;
        let has_item = get_inventory_item(&mut tx, user_id, required_item)
            .await
            .map_err(|_| "Could not check your inventory.".to_string())?;

        if has_item.is_none_or(|i| i.quantity < ingredient.quantity as i64) {
            tx.rollback().await.ok();
            return Err(format!(
                "You don't have enough {}!",
                required_item.display_name()
            ));
        }
    }

    // 3. Atomically consume the ingredients.
    for ingredient in &ingredients {
        let required_item = match Item::from_i32(ingredient.item_id) {
            Some(it) => it,
            None => {
                tx.rollback().await.ok();
                return Err("Invalid ingredient item id in recipe.".into());
            }
        }; // Safer conversion.
        add_to_inventory(
            &mut tx,
            user_id,
            required_item,
            -(ingredient.quantity as i64),
        )
        .await
        .map_err(|_| "Failed to consume crafting materials.".to_string())?;
    }

    // 4. Add the crafted item to the player's inventory.
    let output_item =
        Item::from_i32(recipe.output_item_id).ok_or("Invalid output item ID in recipe.")?;
    add_to_inventory(&mut tx, user_id, output_item, recipe.output_quantity as i64)
        .await
        .map_err(|_| "Failed to add crafted item to your inventory.".to_string())?;

    tx.commit()
        .await
        .map_err(|_| "Failed to finalize the transaction.".to_string())?;

    Ok(output_item)
}
