//! Handles all UI and embed creation for the `/shop` command.

use super::state::ShopSession;
use crate::commands::economy::core::item::{Item, ItemCategory};
use serenity::builder::{
    CreateActionRow, CreateButton, CreateEmbed, CreateSelectMenu, CreateSelectMenuKind,
    CreateSelectMenuOption,
};
use serenity::model::application::ButtonStyle;
// (✓) REMOVED: Unused import of FromStr.

const ITEMS_PER_PAGE: usize = 5;

impl ShopSession {
    // ... render_shop function remains the same ...
    pub(super) fn render_shop(&self) -> (CreateEmbed, Vec<CreateActionRow>) {
        let items_to_display: Vec<Item> = Item::get_all_purchasable()
            .into_iter()
            .filter(|item| {
                item.properties().category == self.current_category
                    && item.properties().buy_price.is_some()
            })
            .collect();

        let start = self.current_page * ITEMS_PER_PAGE;
        let end = (start + ITEMS_PER_PAGE).min(items_to_display.len());
        let page_items = &items_to_display[start..end];

        let title = match self.current_category {
            ItemCategory::Resource => "🛒 Shop - Resources",
            ItemCategory::Special => "✨ Shop - Special Items",
            ItemCategory::Consumable => "🧪 Shop - Consumables",
        };

        let mut embed = CreateEmbed::new().title(title).color(0x5865F2);

        if page_items.is_empty() {
            embed = embed.description("There are no items in this category.");
        } else {
            let item_list = page_items
                .iter()
                .map(|item| {
                    let props = item.properties();
                    if let Some(price) = props.buy_price {
                        format!(
                            "**{} {}** - **💰{}**\n*{}*",
                            props.emoji, props.display_name, price, props.description
                        )
                    } else {
                        format!(
                            "**{} {}**\n*{}*",
                            props.emoji, props.display_name, props.description
                        )
                    }
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            embed = embed.description(item_list);
        }

        let mut components = Vec::<CreateActionRow>::new();

        let category_buttons = CreateActionRow::Buttons(vec![
            CreateButton::new("shop_cat_resources")
                .label("Resources")
                .style(if self.current_category == ItemCategory::Resource {
                    ButtonStyle::Primary
                } else {
                    ButtonStyle::Secondary
                }),
            CreateButton::new("shop_cat_special")
                .label("Special")
                .style(if self.current_category == ItemCategory::Special {
                    ButtonStyle::Primary
                } else {
                    ButtonStyle::Secondary
                }),
            CreateButton::new("shop_cat_consumables")
                .label("Consumables")
                .style(if self.current_category == ItemCategory::Consumable {
                    ButtonStyle::Primary
                } else {
                    ButtonStyle::Secondary
                }),
        ]);
        components.push(category_buttons);

        let mut options = Vec::new();
        for item in page_items {
            let props = item.properties();
            let value = item.to_string();
            let mut option = CreateSelectMenuOption::new(props.display_name, value);
            if let Some(emoji) = props.emoji.chars().next() {
                option = option.emoji(emoji);
            }
            options.push(option);
        }

        if !options.is_empty() {
            let menu = CreateSelectMenu::new("shop_buy", CreateSelectMenuKind::String { options })
                .placeholder("Select an item to purchase...");
            components.push(CreateActionRow::SelectMenu(menu));
        }

        let page_buttons = CreateActionRow::Buttons(vec![
            CreateButton::new("shop_prev_page")
                .label("Previous")
                .style(ButtonStyle::Secondary)
                .disabled(self.current_page == 0),
            CreateButton::new("shop_next_page")
                .label("Next")
                .style(ButtonStyle::Secondary)
                .disabled(end >= items_to_display.len()),
        ]);
        components.push(page_buttons);

        (embed, components)
    }
}

// (✓) ADDED: The missing helper functions required by logic.rs.
pub fn create_error_embed(error_message: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title("Shop Error")
        .description(error_message)
        .color(0xFF0000) // Red
}

pub fn create_success_embed(item_name: &str, quantity: i64, total_cost: i64) -> CreateEmbed {
    CreateEmbed::new()
        .title("Purchase Successful!")
        .description(format!(
            "You bought **`{}` {}** for a total of **💰`{}`** coins.",
            quantity, item_name, total_cost
        ))
        .color(0x00FF00) // Green
}
