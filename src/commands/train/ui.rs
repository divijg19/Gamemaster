//! Handles the UI creation for the `/train` command.

use crate::database::models::{PlayerPet, SagaProfile};
use serenity::builder::{
    CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter, CreateSelectMenu,
    CreateSelectMenuKind, CreateSelectMenuOption,
};
use serenity::model::application::ButtonStyle;

/// Creates the main training menu, showing a list of pets.
pub fn create_training_menu(
    pets: &[PlayerPet],
    saga_profile: &SagaProfile,
) -> (CreateEmbed, Vec<CreateActionRow>) {
    let mut embed = CreateEmbed::new()
        .title("Pet Training Grounds")
        .description("Select a pet from the dropdown menu to begin training. Training costs **1 TP** and takes **2 hours**.")
        .field("⚡ Your Training Points", format!("`{}/{}`", saga_profile.current_tp, saga_profile.max_tp), false)
        .color(0xDAA520); // Goldenrod

    if pets.is_empty() {
        embed = embed.description(
            "You don't have any pets to train yet! Visit the Tavern in the `/saga` menu to hire some.",
        );
        return (embed, vec![]);
    }

    let mut select_menu_options = Vec::new();
    let mut pet_list_lines = Vec::new();

    for pet in pets {
        let pet_name = pet.nickname.as_deref().unwrap_or(&pet.name);
        if pet.is_training {
            let ends_at = pet.training_ends_at.unwrap();
            let timestamp = format!("<t:{}:R>", ends_at.timestamp());
            pet_list_lines.push(format!(
                "💪 **{}** is training **{}** (finishes {})",
                pet_name,
                pet.training_stat.as_deref().unwrap_or("a stat"),
                timestamp
            ));
        } else {
            pet_list_lines.push(format!("✅ **{}** is idle and ready to train.", pet_name));
            select_menu_options.push(CreateSelectMenuOption::new(
                pet_name,
                pet.player_pet_id.to_string(),
            ));
        }
    }

    embed = embed.field("Your Army", pet_list_lines.join("\n"), false);

    let mut components = Vec::new();
    if !select_menu_options.is_empty() && saga_profile.current_tp > 0 {
        let menu = CreateSelectMenu::new(
            "train_select_pet",
            CreateSelectMenuKind::String {
                options: select_menu_options,
            },
        )
        .placeholder("Select an idle pet to train...");
        components.push(CreateActionRow::SelectMenu(menu));
    } else if saga_profile.current_tp == 0 {
        // (✓) FIXED: The footer now creates the struct directly, which is the correct syntax.
        embed = embed.footer(CreateEmbedFooter::new(
            "You are out of Training Points. They recharge over time.",
        ));
    }

    (embed, components)
}

/// Creates the stat selection menu after a pet has been chosen.
pub fn create_stat_selection_menu(player_pet_id: i32) -> (CreateEmbed, Vec<CreateActionRow>) {
    let embed = CreateEmbed::new()
        .title("Choose a Stat to Train")
        .description("Which stat would you like to improve? This will cost **1 TP** and complete in 2 hours.")
        .color(0xDAA520);

    let components = vec![CreateActionRow::Buttons(vec![
        CreateButton::new(format!("train_stat_attack_{}", player_pet_id))
            .label("Attack")
            .style(ButtonStyle::Danger)
            // (✓) FIXED: Use a single `char` for the emoji as required by the builder.
            .emoji('⚔'),
        CreateButton::new(format!("train_stat_defense_{}", player_pet_id))
            .label("Defense")
            .style(ButtonStyle::Primary)
            // (✓) FIXED: Use a single `char` for the emoji as required by the builder.
            .emoji('🛡'),
    ])];

    (embed, components)
}
