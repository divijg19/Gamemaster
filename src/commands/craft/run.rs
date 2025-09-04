//! Implements the run logic for the `/craft` command.

use super::ui::{RecipeInfo, create_crafting_menu};
use crate::commands::economy::core::item::Item;
use crate::{AppState, database};
use serenity::builder::{
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage,
    EditInteractionResponse,
};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::*;
use std::collections::HashMap;

pub fn register() -> CreateCommand {
    CreateCommand::new("craft").description("Craft new items from materials in your inventory.")
}

pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
        .ok();

    let Some(app_state) = AppState::from_ctx(ctx).await else {
        return;
    };
    let pool = app_state.db.clone();

    // 1. Fetch all recipes and ingredients from the database.
    let all_recipes = database::crafting::get_all_recipes(&pool)
        .await
        .unwrap_or_default();
    let mut all_ingredients = HashMap::new();
    for recipe in &all_recipes {
        if let Ok(ingredients) =
            database::crafting::get_ingredients_for_recipe(&pool, recipe.recipe_id).await
        {
            all_ingredients.insert(recipe.recipe_id, ingredients);
        }
    }

    // 2. Fetch the player's inventory.
    let inventory = database::economy::get_inventory(&pool, interaction.user.id)
        .await
        .unwrap_or_default();

    // 3. Combine the data into the format the UI expects.
    let recipe_infos: Vec<_> = all_recipes
        .iter()
        .filter_map(|recipe| {
            if let Some(ingredients) = all_ingredients.get(&recipe.recipe_id)
                && let Some(output_item) = Item::from_i32(recipe.output_item_id)
            {
                return Some(RecipeInfo {
                    recipe,
                    ingredients,
                    output_item,
                });
            }
            None
        })
        .collect();

    // 4. Generate and send the UI.
    let (embed, components) = create_crafting_menu(&recipe_infos, &inventory);
    let builder = EditInteractionResponse::new()
        .embed(embed)
        .components(components);

    interaction.edit_response(&ctx.http, builder).await.ok();
}

pub async fn run_prefix(ctx: &Context, msg: &Message, _args: Vec<&str>) {
    let Some(app_state) = AppState::from_ctx(ctx).await else {
        return;
    };
    let pool = app_state.db.clone();

    let all_recipes = database::crafting::get_all_recipes(&pool)
        .await
        .unwrap_or_default();
    let mut all_ingredients = HashMap::new();
    for recipe in &all_recipes {
        if let Ok(ingredients) =
            database::crafting::get_ingredients_for_recipe(&pool, recipe.recipe_id).await
        {
            all_ingredients.insert(recipe.recipe_id, ingredients);
        }
    }

    let inventory = database::economy::get_inventory(&pool, msg.author.id)
        .await
        .unwrap_or_default();

    let recipe_infos: Vec<_> = all_recipes
        .iter()
        .filter_map(|recipe| {
            if let Some(ingredients) = all_ingredients.get(&recipe.recipe_id)
                && let Some(output_item) = Item::from_i32(recipe.output_item_id)
            {
                return Some(RecipeInfo {
                    recipe,
                    ingredients,
                    output_item,
                });
            }
            None
        })
        .collect();

    let (embed, components) = create_crafting_menu(&recipe_infos, &inventory);
    let builder = CreateMessage::new()
        .embed(embed)
        .components(components)
        .reference_message(msg);
    msg.channel_id.send_message(&ctx.http, builder).await.ok();
}
