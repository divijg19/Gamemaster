//! Handles the UI creation for the `/party` command.

use crate::database::models::PlayerPet;
use serenity::builder::{
    CreateActionRow, CreateEmbed, CreateEmbedFooter, CreateSelectMenu, CreateSelectMenuKind,
    CreateSelectMenuOption,
};

/// Creates the main embed and components for the party and army management view.
pub fn create_party_view(pets: &[PlayerPet]) -> (CreateEmbed, Vec<CreateActionRow>) {
    let mut embed = CreateEmbed::new()
        .title("Party & Army Management")
        .description(
            "Your **Party** is your active combat team. Your **Army** is all units you own.",
        )
        .footer(CreateEmbedFooter::new(format!(
            "Total Army Size: {}/10",
            pets.len()
        )))
        .color(0x3498DB);

    if pets.is_empty() {
        embed = embed.description(
            "Your army is empty! Visit the Tavern in the `/saga` menu to hire your first units.",
        );
        return (embed, vec![]);
    }

    let party: Vec<_> = pets.iter().filter(|p| p.is_in_party).collect();
    let reserves: Vec<_> = pets.iter().filter(|p| !p.is_in_party).collect();

    let party_list = if party.is_empty() {
        "Your active party is empty. Add members from your reserves!".to_string()
    } else {
        party
            .iter()
            .map(|p| format_pet_line(p))
            .collect::<Vec<_>>()
            .join("\n")
    };
    embed = embed.field(
        format!("⚔️ Active Party ({}/5)", party.len()),
        party_list,
        false,
    );

    if !reserves.is_empty() {
        let reserve_list = reserves
            .iter()
            .map(|p| format_pet_line(p))
            .collect::<Vec<_>>()
            .join("\n");
        embed = embed.field("🛡️ Reserves", reserve_list, false);
    }

    let mut components = Vec::new();

    let add_options: Vec<_> = reserves
        .iter()
        .map(|p| {
            let pet_name = p.nickname.as_deref().unwrap_or(&p.name);
            CreateSelectMenuOption::new(pet_name, p.player_pet_id.to_string())
        })
        .collect();

    if !add_options.is_empty() && party.len() < 5 {
        let menu = CreateSelectMenu::new(
            "party_add",
            CreateSelectMenuKind::String {
                options: add_options,
            },
        )
        .placeholder("Add a unit to your party...");
        components.push(CreateActionRow::SelectMenu(menu));
    }

    let remove_options: Vec<_> = party
        .iter()
        .map(|p| {
            let pet_name = p.nickname.as_deref().unwrap_or(&p.name);
            CreateSelectMenuOption::new(pet_name, p.player_pet_id.to_string())
        })
        .collect();

    if !remove_options.is_empty() {
        let menu = CreateSelectMenu::new(
            "party_remove",
            CreateSelectMenuKind::String {
                options: remove_options,
            },
        )
        .placeholder("Remove a unit from your party...");
        components.push(CreateActionRow::SelectMenu(menu));
    }

    (embed, components)
}

/// Helper function to format a single line for a pet's display.
fn format_pet_line(pet: &PlayerPet) -> String {
    let training_status = if pet.is_training {
        let ends_at = pet.training_ends_at.unwrap();
        let timestamp = format!("<t:{}:R>", ends_at.timestamp());
        format!("(💪 Training ends {})", timestamp)
    } else {
        "".to_string()
    };

    let pet_name = pet.nickname.as_deref().unwrap_or(&pet.name);

    // (✓) ALIVE: The pet's current_xp is now displayed, making the data model whole.
    format!(
        "**{}** | Lvl {} (`{}` XP) | Atk: {} | Def: {} | HP: {} {}",
        pet_name,
        pet.current_level,
        pet.current_xp,
        pet.current_attack,
        pet.current_defense,
        pet.current_health,
        training_status
    )
    .trim()
    .to_string()
}
