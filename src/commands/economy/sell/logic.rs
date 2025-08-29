//! Contains the core logic for the `sell` command.

use super::ui;
use crate::commands::economy::core::item::Item;
use crate::database;
use serenity::builder::CreateEmbed;
use serenity::model::user::User;
use sqlx::PgPool;

pub async fn sell_items(
    pool: &PgPool,
    user: &User,
    item: Item,
    quantity: Option<i64>,
) -> CreateEmbed {
    let properties = item.properties();
    if !properties.is_sellable {
        return ui::create_error_embed(&format!(
            "The item '{}' cannot be sold.",
            properties.display_name
        ));
    }
    // (✓) MODIFIED: Use the convenience wrapper method.
    let sell_price = item.sell_price().unwrap_or(0);

    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return ui::create_error_embed("Could not start database transaction."),
    };

    let inventory_item = match database::profile::get_inventory_item(&mut tx, user.id, item).await {
        Ok(Some(item)) => item,
        _ => {
            tx.rollback().await.ok();
            return ui::create_error_embed(&format!(
                "You do not have any {} to sell.",
                properties.display_name
            ));
        }
    };

    let amount_to_sell = quantity.unwrap_or(inventory_item.quantity).max(1);

    if inventory_item.quantity < amount_to_sell {
        tx.rollback().await.ok();
        return ui::create_error_embed(&format!(
            "You only have `{}` {} to sell.",
            inventory_item.quantity, properties.display_name
        ));
    }

    let total_sale_price = sell_price * amount_to_sell;

    if database::profile::add_to_inventory(&mut tx, user.id, item, -amount_to_sell)
        .await
        .is_err()
    {
        tx.rollback().await.ok();
        return ui::create_error_embed("Failed to remove items from your inventory.");
    }
    if database::profile::add_balance(&mut tx, user.id, total_sale_price)
        .await
        .is_err()
    {
        tx.rollback().await.ok();
        return ui::create_error_embed("Failed to add coins to your balance.");
    }

    if tx.commit().await.is_err() {
        return ui::create_error_embed("Failed to commit the transaction.");
    }

    ui::create_success_embed(properties.display_name, amount_to_sell, total_sale_price)
}
