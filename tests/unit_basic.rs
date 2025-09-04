use gamemaster_bot::commands::economy::core::item::Item;
use gamemaster_bot::database::human::defeats_required_for;
use gamemaster_bot::database::models::UnitRarity;

#[test]
fn test_defeats_required_progression() {
    assert_eq!(defeats_required_for(UnitRarity::Common), 2);
    assert_eq!(defeats_required_for(UnitRarity::Rare), 3);
    assert_eq!(defeats_required_for(UnitRarity::Epic), 5);
    assert_eq!(defeats_required_for(UnitRarity::Legendary), 7);
}

#[test]
fn test_research_item_mapping() {
    assert_eq!(
        Item::research_item_for_unit("Slime"),
        Some(Item::SlimeResearchData)
    );
    assert_eq!(
        Item::research_item_for_unit("Wolf"),
        Some(Item::WolfResearchData)
    );
    assert_eq!(
        Item::research_item_for_unit("Alpha Wolf"),
        Some(Item::WolfResearchData)
    );
    assert_eq!(
        Item::research_item_for_unit("Bear"),
        Some(Item::BearResearchData)
    );
    assert_eq!(
        Item::research_item_for_unit("Giant Spider"),
        Some(Item::SpiderResearchData)
    );
    assert_eq!(Item::research_item_for_unit("Unknown"), None);
}
