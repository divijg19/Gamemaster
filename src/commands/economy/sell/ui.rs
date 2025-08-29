//! Contains the UI-related functions for the `/sell` command.

use serenity::all::Colour;
use serenity::builder::CreateEmbed;

/// Creates a standardized error embed for the sell command.
pub fn create_error_embed(error_message: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title("Sell Error")
        .description(error_message)
        .color(Colour::RED)
}

/// Creates a standardized success embed for a successful sale.
pub fn create_success_embed(item_name: &str, quantity_sold: i64, total_price: i64) -> CreateEmbed {
    CreateEmbed::new()
        .title("Sale Successful!")
        .description(format!(
            "You sold **`{}` {}** for a total of **💰`{}`** coins.",
            quantity_sold, item_name, total_price
        ))
        .color(Colour::GOLD)
}
