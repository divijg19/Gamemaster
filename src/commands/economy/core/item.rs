//! Defines all items, their properties, and master lists for the economy.

use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Legendary,
}

impl Rarity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Rarity::Common => "Common",
            Rarity::Uncommon => "Uncommon",
            Rarity::Rare => "Rare",
            Rarity::Legendary => "Legendary",
        }
    }

    pub fn color(&self) -> u32 {
        match self {
            Rarity::Common => 0x95A5A6,
            Rarity::Uncommon => 0x2ECC71,
            Rarity::Rare => 0x3498DB,
            Rarity::Legendary => 0x9B59B6,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemCategory {
    Resource,
    Special,
    Consumable,
}

pub struct ItemProperties {
    pub display_name: &'static str,
    pub description: &'static str,
    pub emoji: &'static str,
    pub category: ItemCategory,
    pub rarity: Rarity,
    pub is_sellable: bool,
    pub is_tradeable: bool,
    pub buy_price: Option<i64>,
    pub sell_price: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Item {
    Fish = 1,
    Ore = 2,
    Gem = 3,
    GoldenFish = 4,
    LargeGeode = 5,
    AncientRelic = 6,
    XpBooster = 7,
    SlimeGel = 8,
    SlimeResearchData = 9,
    TamingLure = 10,
}

impl Item {
    pub fn properties(&self) -> ItemProperties {
        match self {
            Item::Fish => ItemProperties {
                display_name: "Fish",
                description: "A common fish. Good for selling in bulk.",
                emoji: "🐟",
                category: ItemCategory::Resource,
                rarity: Rarity::Common,
                is_sellable: true,
                is_tradeable: true,
                buy_price: Some(20),
                sell_price: Some(10),
            },
            Item::Ore => ItemProperties {
                display_name: "Ore",
                description: "A chunk of raw, unprocessed ore.",
                emoji: "⛏️",
                category: ItemCategory::Resource,
                rarity: Rarity::Common,
                is_sellable: true,
                is_tradeable: true,
                buy_price: Some(100),
                sell_price: Some(50),
            },
            Item::Gem => ItemProperties {
                display_name: "Gem",
                description: "A polished, valuable gemstone.",
                emoji: "💎",
                category: ItemCategory::Resource,
                rarity: Rarity::Uncommon,
                is_sellable: true,
                is_tradeable: true,
                buy_price: Some(500),
                sell_price: Some(250),
            },
            Item::GoldenFish => ItemProperties {
                display_name: "Golden Fish",
                description: "An incredibly rare and valuable fish. A true prize!",
                emoji: "🐠",
                category: ItemCategory::Special,
                rarity: Rarity::Rare,
                is_sellable: true,
                is_tradeable: true,
                buy_price: None,
                sell_price: Some(1000),
            },
            Item::LargeGeode => ItemProperties {
                display_name: "Large Geode",
                description: "A heavy, unassuming rock. Perhaps something valuable is inside?",
                emoji: "🪨",
                category: ItemCategory::Special,
                rarity: Rarity::Rare,
                is_sellable: false,
                is_tradeable: true,
                buy_price: None,
                sell_price: None,
            },
            Item::AncientRelic => ItemProperties {
                display_name: "Ancient Relic",
                description: "A mysterious artifact from a forgotten era. Its value is immense.",
                emoji: "🏺",
                category: ItemCategory::Special,
                rarity: Rarity::Legendary,
                is_sellable: true,
                is_tradeable: true,
                buy_price: None,
                sell_price: Some(10000),
            },
            Item::XpBooster => ItemProperties {
                display_name: "XP Booster",
                description: "Doubles XP gain from working for one hour.",
                emoji: "🚀",
                category: ItemCategory::Consumable,
                rarity: Rarity::Rare,
                is_sellable: false,
                is_tradeable: true,
                buy_price: Some(2000),
                sell_price: None,
            },
            Item::SlimeGel => ItemProperties {
                display_name: "Slime Gel",
                description: "A sticky, gelatinous substance. Surprisingly useful in crafting.",
                emoji: "🟢",
                category: ItemCategory::Resource,
                rarity: Rarity::Common,
                is_sellable: true,
                is_tradeable: true,
                buy_price: None,
                sell_price: Some(5),
            },
            Item::SlimeResearchData => ItemProperties {
                display_name: "Slime Research Data",
                description: "Combat notes that could be used to tame a slime.",
                emoji: "🔬",
                category: ItemCategory::Special,
                rarity: Rarity::Uncommon,
                is_sellable: false,
                is_tradeable: false,
                buy_price: None,
                sell_price: None,
            },
            Item::TamingLure => ItemProperties {
                display_name: "Taming Lure",
                description: "A lure used to attract and pacify wild creatures.",
                emoji: "🐾",
                category: ItemCategory::Consumable,
                rarity: Rarity::Uncommon,
                is_sellable: true,
                is_tradeable: true,
                buy_price: Some(250),
                sell_price: Some(125),
            },
        }
    }

    pub fn get_all_purchasable() -> Vec<Item> {
        vec![
            Item::Fish,
            Item::Ore,
            Item::Gem,
            Item::XpBooster,
            Item::TamingLure,
        ]
    }

    pub fn from_i32(id: i32) -> Option<Self> {
        match id {
            1 => Some(Item::Fish),
            2 => Some(Item::Ore),
            3 => Some(Item::Gem),
            4 => Some(Item::GoldenFish),
            5 => Some(Item::LargeGeode),
            6 => Some(Item::AncientRelic),
            7 => Some(Item::XpBooster),
            8 => Some(Item::SlimeGel),
            9 => Some(Item::SlimeResearchData),
            10 => Some(Item::TamingLure),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        self.properties().display_name
    }
    pub fn emoji(&self) -> &'static str {
        self.properties().emoji
    }
    pub fn sell_price(&self) -> Option<i64> {
        self.properties().sell_price
    }
}

impl FromStr for Item {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "fish" => Ok(Item::Fish),
            "ore" => Ok(Item::Ore),
            "gem" => Ok(Item::Gem),
            "goldenfish" | "golden" => Ok(Item::GoldenFish),
            "largegeode" | "geode" => Ok(Item::LargeGeode),
            "ancientrelic" | "relic" => Ok(Item::AncientRelic),
            "xpbooster" | "booster" => Ok(Item::XpBooster),
            "slimegel" | "gel" => Ok(Item::SlimeGel),
            "slimedata" | "data" => Ok(Item::SlimeResearchData),
            "taminglure" | "lure" => Ok(Item::TamingLure),
            _ => Err(()),
        }
    }
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Item::Fish => "fish",
                Item::Ore => "ore",
                Item::Gem => "gem",
                Item::GoldenFish => "goldenfish",
                Item::LargeGeode => "largegeode",
                Item::AncientRelic => "ancientrelic",
                Item::XpBooster => "xpbooster",
                Item::SlimeGel => "slimegel",
                Item::SlimeResearchData => "slimedata",
                Item::TamingLure => "taminglure",
            }
        )
    }
}
