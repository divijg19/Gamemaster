//! Contains the core logic for the `/give` command.

use crate::commands::economy::core::item::Item;
use crate::database;
use serenity::builder::CreateEmbed;
use serenity::model::user::User;
use sqlx::PgPool;

pub async fn give_item(
    pool: &PgPool,
    giver: &User,
    receiver: &User,
    item: Item,
    quantity: i64,
) -> CreateEmbed {
    if giver.id == receiver.id {
        return CreateEmbed::new()
            .title("Error")
            .description("You cannot give items to yourself.")
            .color(0xFF0000);
    }
    if receiver.bot {
        return CreateEmbed::new()
            .title("Error")
            .description("You cannot give items to bots.")
            .color(0xFF0000);
    }
    if quantity <= 0 {
        return CreateEmbed::new()
            .title("Error")
            .description("You must give at least one item.")
            .color(0xFF0000);
    }

    let properties = item.properties();
    // (✓) ALIVE: The `is_tradeable` flag is now being used for game logic.
    if !properties.is_tradeable {
        let err_msg = format!("The item '{}' cannot be traded.", properties.display_name);
        return CreateEmbed::new()
            .title("Trade Error")
            .description(err_msg)
            .color(0xFF0000);
    }

    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => {
            return CreateEmbed::new()
                .title("Error")
                .description("Could not start database transaction.")
                .color(0xFF0000);
        }
    };

    // Check if the giver has enough items to give.
    match database::profile::get_inventory_item(&mut tx, giver.id, item).await {
        Ok(Some(item_in_inv)) if item_in_inv.quantity >= quantity => (),
        _ => {
            tx.rollback().await.ok();
            let err_msg = format!(
                "You do not have enough **{}** to give. You need `{}` but only have the required amount.",
                properties.display_name, quantity
            );
            return CreateEmbed::new()
                .title("Not Enough Items")
                .description(err_msg)
                .color(0xFF0000);
        }
    };

    // Perform the transaction.
    if database::profile::add_to_inventory(&mut tx, giver.id, item, -quantity)
        .await
        .is_err()
        || database::profile::add_to_inventory(&mut tx, receiver.id, item, quantity)
            .await
            .is_err()
    {
        tx.rollback().await.ok();
        return CreateEmbed::new()
            .title("Error")
            .description("Failed to update inventories.")
            .color(0xFF0000);
    }

    if tx.commit().await.is_err() {
        return CreateEmbed::new()
            .title("Error")
            .description("Failed to commit the transaction.")
            .color(0xFF0000);
    }

    CreateEmbed::new()
        .title("Trade Successful!")
        .description(format!(
            "You gave **`{}` {}** to **{}**.",
            quantity, properties.display_name, receiver.name
        ))
        .color(0x00FF00)
}
