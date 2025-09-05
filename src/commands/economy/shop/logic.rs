//! Contains the core logic for the `/shop` command.

use super::ui;
use crate::commands::economy::core::item::Item;
use crate::database;
use serenity::builder::CreateEmbed;
use serenity::model::user::User;
use sqlx::PgPool;

pub async fn buy_item(pool: &PgPool, user: &User, item: Item, quantity: i64) -> CreateEmbed {
    let properties = item.properties();
    let buy_price = match properties.buy_price {
        Some(price) => price,
        None => {
            return ui::create_error_embed(&format!(
                "'{}' cannot be bought from the shop.",
                properties.display_name
            ));
        }
    };

    if quantity <= 0 {
        return ui::create_error_embed("You must buy at least one item.");
    }

    let total_cost = buy_price * quantity;

    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return ui::create_error_embed("Could not start database transaction."),
    };

    // (✓) FIXED: Dereference the transaction `tx` to get an executor `&mut *tx`.
    // This passes the underlying connection to the function, which satisfies the trait bound.
    let profile = match database::economy::get_or_create_profile(pool, user.id).await {
        Ok(p) => p,
        Err(_) => {
            tx.rollback().await.ok();
            return ui::create_error_embed("Could not fetch your profile.");
        }
    };

    if profile.balance < total_cost {
        tx.rollback().await.ok();
        return ui::create_error_embed(&format!(
            "You cannot afford that! You need **💰{}**, but you only have **💰{}**.",
            total_cost, profile.balance
        ));
    }

    if database::economy::add_balance(&mut tx, user.id, -total_cost)
        .await
        .is_err()
    {
        tx.rollback().await.ok();
        return ui::create_error_embed("Failed to deduct coins from your balance.");
    }

    if database::economy::add_to_inventory(&mut tx, user.id, item, quantity)
        .await
        .is_err()
    {
        tx.rollback().await.ok();
        return ui::create_error_embed("Failed to add the item to your inventory.");
    }

    if tx.commit().await.is_err() {
        return ui::create_error_embed("Failed to commit the transaction to the database.");
    }

    ui::create_success_embed(properties.display_name, quantity, total_cost)
}
