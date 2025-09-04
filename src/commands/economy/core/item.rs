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
    HealthPotion = 11,
    WolfResearchData = 12,
    BoarResearchData = 13,
    ForestContractParchment = 14,
    FrontierContractParchment = 15,
    ScholarResearchNotes = 16,
    GreaterHealthPotion = 17,
    StaminaDraft = 18,
    FocusTonic = 19,
    BearResearchData = 20,
    SpiderResearchData = 21,
}

impl Item {
    // (✓) NEW: Add the id() method to resolve compiler errors.
    pub fn id(&self) -> i32 {
        *self as i32
    }

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
                description: "A lure used to attract and pacify wild or proud units for bonding.",
                emoji: "🐾",
                category: ItemCategory::Consumable,
                rarity: Rarity::Uncommon,
                is_sellable: true,
                is_tradeable: true,
                buy_price: Some(250),
                sell_price: Some(125),
            },
            Item::HealthPotion => ItemProperties {
                display_name: "Health Potion",
                description: "A basic potion that restores a small amount of health.",
                emoji: "🧪",
                category: ItemCategory::Consumable,
                rarity: Rarity::Uncommon,
                is_sellable: true,
                is_tradeable: true,
                buy_price: None,
                sell_price: Some(50),
            },
            Item::WolfResearchData => ItemProperties {
                display_name: "Wolf Research Data",
                description: "Observations on wolf behavior, useful for taming.",
                emoji: "📓",
                category: ItemCategory::Special,
                rarity: Rarity::Uncommon,
                is_sellable: false,
                is_tradeable: false,
                buy_price: None,
                sell_price: None,
            },
            Item::BoarResearchData => ItemProperties {
                display_name: "Boar Research Data",
                description: "Notes on boar aggression and patterns.",
                emoji: "📕",
                category: ItemCategory::Special,
                rarity: Rarity::Uncommon,
                is_sellable: false,
                is_tradeable: false,
                buy_price: None,
                sell_price: None,
            },
            Item::ForestContractParchment => ItemProperties {
                display_name: "Forest Contract Parchment",
                description: "A blank contract ready to draft a local human recruit.",
                emoji: "📜",
                category: ItemCategory::Special,
                rarity: Rarity::Rare,
                is_sellable: true,
                is_tradeable: true,
                buy_price: None,
                sell_price: Some(300),
            },
            Item::FrontierContractParchment => ItemProperties {
                display_name: "Frontier Contract Parchment",
                description: "Higher grade contract for seasoned humans.",
                emoji: "📜",
                category: ItemCategory::Special,
                rarity: Rarity::Rare,
                is_sellable: true,
                is_tradeable: true,
                buy_price: None,
                sell_price: Some(500),
            },
            Item::ScholarResearchNotes => ItemProperties {
                display_name: "Scholar Research Notes",
                description: "Dense annotations that accelerate future discoveries.",
                emoji: "📘",
                category: ItemCategory::Special,
                rarity: Rarity::Rare,
                is_sellable: false,
                is_tradeable: false,
                buy_price: None,
                sell_price: None,
            },
            Item::GreaterHealthPotion => ItemProperties {
                display_name: "Greater Health Potion",
                description: "Restores a large amount of health.",
                emoji: "🧪",
                category: ItemCategory::Consumable,
                rarity: Rarity::Rare,
                is_sellable: true,
                is_tradeable: true,
                buy_price: None,
                sell_price: Some(150),
            },
            Item::StaminaDraft => ItemProperties {
                display_name: "Stamina Draft",
                description: "Restores action stamina in the saga.",
                emoji: "🥤",
                category: ItemCategory::Consumable,
                rarity: Rarity::Rare,
                is_sellable: true,
                is_tradeable: true,
                buy_price: None,
                sell_price: Some(120),
            },
            Item::FocusTonic => ItemProperties {
                display_name: "Focus Tonic",
                description: "Slightly increases research drop rate for a short time.",
                emoji: "🧴",
                category: ItemCategory::Consumable,
                rarity: Rarity::Rare,
                is_sellable: true,
                is_tradeable: true,
                buy_price: None,
                sell_price: Some(140),
            },
            Item::BearResearchData => ItemProperties {
                display_name: "Bear Research Data",
                description: "Heavy scrawlings on bear movement and power.",
                emoji: "📙",
                category: ItemCategory::Special,
                rarity: Rarity::Rare,
                is_sellable: false,
                is_tradeable: false,
                buy_price: None,
                sell_price: None,
            },
            Item::SpiderResearchData => ItemProperties {
                display_name: "Spider Research Data",
                description: "Sketched web patterns and venom potency notes.",
                emoji: "🕷️",
                category: ItemCategory::Special,
                rarity: Rarity::Rare,
                is_sellable: false,
                is_tradeable: false,
                buy_price: None,
                sell_price: None,
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
            11 => Some(Item::HealthPotion),
            12 => Some(Item::WolfResearchData),
            13 => Some(Item::BoarResearchData),
            14 => Some(Item::ForestContractParchment),
            15 => Some(Item::FrontierContractParchment),
            16 => Some(Item::ScholarResearchNotes),
            17 => Some(Item::GreaterHealthPotion),
            18 => Some(Item::StaminaDraft),
            19 => Some(Item::FocusTonic),
            20 => Some(Item::BearResearchData),
            21 => Some(Item::SpiderResearchData),
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

    /// Maps a unit (pet/creature) name to its corresponding Research Data item, if any.
    /// Includes aliasing for evolved or alpha forms sharing the same research.
    pub fn research_item_for_unit(unit_name: &str) -> Option<Item> {
        match unit_name {
            // Slime family
            "Slime" => Some(Item::SlimeResearchData),
            // Wolves (Alpha Wolf shares Wolf research data)
            "Wolf" | "Alpha Wolf" => Some(Item::WolfResearchData),
            // Boars
            "Boar" => Some(Item::BoarResearchData),
            // Bear
            "Bear" => Some(Item::BearResearchData),
            // Giant Spider maps to generic Spider research
            "Giant Spider" => Some(Item::SpiderResearchData),
            _ => None,
        }
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
            "taminglure" | "lure" | "contract" => Ok(Item::TamingLure),
            "healthpotion" | "potion" => Ok(Item::HealthPotion),
            "wolfresearchdata" | "wolfdata" => Ok(Item::WolfResearchData),
            "boarresearchdata" | "boardata" => Ok(Item::BoarResearchData),
            "forestcontract" | "forestparchment" => Ok(Item::ForestContractParchment),
            "frontiercontract" | "frontierparchment" => Ok(Item::FrontierContractParchment),
            "scholarnotes" | "researchnotes" => Ok(Item::ScholarResearchNotes),
            "greaterpotion" | "greaterhealthpotion" => Ok(Item::GreaterHealthPotion),
            "staminadraft" | "draft" => Ok(Item::StaminaDraft),
            "focustonic" | "tonic" => Ok(Item::FocusTonic),
            "beardata" | "bearresearchdata" => Ok(Item::BearResearchData),
            "spiderdata" | "spiderresearchdata" => Ok(Item::SpiderResearchData),
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
                Item::HealthPotion => "healthpotion",
                Item::WolfResearchData => "wolfdata",
                Item::BoarResearchData => "boardata",
                Item::ForestContractParchment => "forestcontract",
                Item::FrontierContractParchment => "frontiercontract",
                Item::ScholarResearchNotes => "scholarnotes",
                Item::GreaterHealthPotion => "greaterhealthpotion",
                Item::StaminaDraft => "staminadraft",
                Item::FocusTonic => "focustonic",
                Item::BearResearchData => "beardata",
                Item::SpiderResearchData => "spiderdata",
            }
        )
    }
}
