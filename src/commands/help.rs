//! This module implements a state-of-the-art, interactive help command.
//!
//! Features:
//! - A categorized main menu for easy browsing.
//! - An interactive dropdown menu for both slash and prefix commands.
//! - Dynamic slash command option registration.
//! - A detailed view for specific commands.

use crate::AppState;
use serenity::all::ComponentInteractionDataKind;
use serenity::builder::{
    CreateActionRow, CreateCommand, CreateCommandOption, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponseMessage, CreateMessage, CreateSelectMenu, CreateSelectMenuKind,
    CreateSelectMenuOption, EditInteractionResponse,
};
use serenity::model::application::{CommandInteraction, CommandOptionType, ComponentInteraction};
use serenity::model::channel::Message;
use serenity::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum CommandCategory {
    General,
    Economy,
    Saga,
    Games, // (✓) IMPROVED: Re-added for clarity.
    Admin,
}

impl CommandCategory {
    fn name(&self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Economy => "Economy & Items",
            Self::Saga => "Gamemaster Saga",
            Self::Games => "Mini-Games",
            Self::Admin => "Admin",
        }
    }
    fn emoji(&self) -> &'static str {
        match self {
            Self::General => "🔧",
            Self::Economy => "💰",
            Self::Saga => "📜",
            Self::Games => "🎮",
            Self::Admin => "🛡️",
        }
    }
}

struct CommandInfo {
    name: &'static str,
    description: &'static str,
    usage: &'static [&'static str],
    details: &'static str,
    category: CommandCategory,
}

const COMMANDS: &[CommandInfo] = &[
    // General Commands
    CommandInfo {
        name: "ping",
        description: "Checks the bot's latency.",
        usage: &["ping"],
        details: "Pings the Discord gateway to check the bot's heartbeat latency.",
        category: CommandCategory::General,
    },
    CommandInfo {
        name: "help",
        description: "Shows this help menu.",
        usage: &["help", "h", "help <command>"],
        details: "Displays a list of all available commands or detailed information about a specific command.",
        category: CommandCategory::General,
    },
    CommandInfo {
        name: "leaderboard",
        description: "View the server-wide leaderboards.",
        usage: &["leaderboard", "lb"],
        details: "Displays the top players across several categories.",
        category: CommandCategory::General,
    },
    CommandInfo {
        name: "tasks",
        description: "View your daily and weekly tasks.",
        usage: &["tasks", "t"],
        details: "Shows your current daily and weekly tasks. Completed tasks can be claimed for rewards from this menu.",
        category: CommandCategory::General,
    },
    // Economy Commands
    CommandInfo {
        name: "profile",
        description: "Displays your or another user's profile.",
        usage: &["profile", "p", "profile @user"],
        details: "Shows your complete profile, including coin balance, game stats (AP/TP), job levels, and inventory.",
        category: CommandCategory::Economy,
    },
    CommandInfo {
        name: "work",
        description: "Work a job to earn coins and resources.",
        usage: &["work <job>", "w <job>"],
        details: "Perform a job to earn rewards and XP. **Jobs:** `fishing`, `mining`, `coding`.",
        category: CommandCategory::Economy,
    },
    CommandInfo {
        name: "inventory",
        description: "Check your item inventory.",
        usage: &["inventory", "inv", "i"],
        details: "Displays a list of all the items you currently own.",
        category: CommandCategory::Economy,
    },
    CommandInfo {
        name: "sell",
        description: "Sell items from your inventory.",
        usage: &["sell <item> [quantity]"],
        details: "Sell items you've collected to earn coins. Sells the whole stack if quantity is omitted.",
        category: CommandCategory::Economy,
    },
    CommandInfo {
        name: "shop",
        description: "Buy items from the bot.",
        usage: &["shop"],
        details: "Opens an interactive shop menu to purchase items.",
        category: CommandCategory::Economy,
    },
    CommandInfo {
        name: "give",
        description: "Give an item to another user.",
        usage: &["give @user <item> [quantity]"],
        details: "Transfer an item from your inventory to another user.",
        category: CommandCategory::Economy,
    },
    CommandInfo {
        name: "craft",
        description: "Craft new items from materials.",
        usage: &["craft", "c"],
        details: "Opens the crafting menu to create new items from resources.",
        category: CommandCategory::Economy,
    },
    // Saga Commands
    CommandInfo {
        name: "saga",
        description: "Opens the main menu for the Gamemaster Saga.",
        usage: &["saga", "play"],
        details: "The central hub for the main game. From here you can view the world map, hire mercenaries, and manage your party.",
        category: CommandCategory::Saga,
    },
    CommandInfo {
        name: "quests",
        description: "View the Guild Quest Board.",
        usage: &["quests", "q"],
        details: "Displays the quest board, showing available quests to accept. Accepting a battle quest starts the fight immediately.",
        category: CommandCategory::Saga,
    },
    CommandInfo {
        name: "questlog",
        description: "View your active and completed quests.",
        usage: &["questlog", "ql"],
        details: "Opens your personal quest log so you can review active objectives and completed quest history.",
        category: CommandCategory::Saga,
    },
    CommandInfo {
        name: "contracts",
        description: "Manage human encounter contracts.",
        usage: &["contracts"],
        details: "Shows human encounter progress, lets you draft contracts for ready humans and accept drafted contracts to recruit them.",
        category: CommandCategory::Saga,
    },
    CommandInfo {
        name: "research",
        description: "View or advance unit research bonuses.",
        usage: &["research"],
        details: "Opens the research interface to see passive bonuses unlocked by collecting research data drops from battles.",
        category: CommandCategory::Saga,
    },
    CommandInfo {
        name: "bestiary",
        description: "Browse discovered units and lore.",
        usage: &["bestiary", "be"],
        details: "Displays units you have encountered with basic stats and rarity; expand future lore entries here.",
        category: CommandCategory::Saga,
    },
    CommandInfo {
        name: "progress",
        description: "Shows your overall saga progression milestones.",
        usage: &["progress"],
        details: "Summarizes story progress, unlocked systems, and upcoming goals.",
        category: CommandCategory::Saga,
    },
    CommandInfo {
        name: "open",
        description: "Open loot or reward crates (if available).",
        usage: &["open <crate>"],
        details: "Opens a crate or reward container from your inventory and applies its contents.",
        category: CommandCategory::Saga,
    },
    // (✓) NEW: Added Quest Log to the help menu.
    CommandInfo {
        name: "party",
        description: "Manage your active party and army.",
        usage: &["party", "army"],
        details: "View all the units you own and set your active 5-member combat party.",
        category: CommandCategory::Saga,
    },
    CommandInfo {
        name: "bond",
        description: "Bond (equip) one unit onto another for stat bonuses.",
        usage: &["bond"],
        details: "Opens the bonding menu. Select a host (higher rarity) and an equippable (equal or lower rarity). Provides augmentation bonuses based on rarity and level. Only one equipped unit per host. Unequip preserves history.",
        category: CommandCategory::Saga,
    },
    CommandInfo {
        name: "train",
        description: "Train your units to improve their stats.",
        usage: &["train", "tr"],
        details: "Opens the training menu to spend Training Points (TP) on offline training sessions for your units.",
        category: CommandCategory::Saga,
    },
    // Games Commands
    CommandInfo {
        name: "rps",
        description: "Challenge a user to Rock, Paper, Scissors.",
        usage: &["rps @user"],
        details: "Starts a game of Rock, Paper, Scissors against another user.",
        category: CommandCategory::Games,
    },
    CommandInfo {
        name: "blackjack",
        description: "Play a game of Blackjack.",
        usage: &["blackjack <bet>", "bj <bet>"],
        details: "Starts a game of Blackjack against the house. Try to get as close to 21 as possible without going over.",
        category: CommandCategory::Games,
    },
    CommandInfo {
        name: "poker",
        description: "Play Five Card Draw poker.",
        usage: &["poker <bet>", "pk <bet>"],
        details: "Starts a game of Five Card Draw poker against the dealer.",
        category: CommandCategory::Games,
    },
    // Admin Commands
    CommandInfo {
        name: "prefix",
        description: "Views or (admin only) sets the prefix.",
        usage: &["prefix", "prefix set <new_prefix>"],
        details: "Displays the current command prefix. Administrators can use the `set` subcommand to change it.",
        category: CommandCategory::Admin,
    },
    CommandInfo {
        name: "adminutil",
        description: "Owner maintenance utilities (cache stats, bonding tests, marking units).",
        usage: &[
            "adminutil cachestats",
            "adminutil markhuman <unit_id>",
            "adminutil bondhost <id> bondequip <id>",
        ],
        details: "Provides maintenance helpers: view global cache hit/miss counters, mark units as Human, perform a raw bond test pair, and inspect research progress.",
        category: CommandCategory::Admin,
    },
    CommandInfo {
        name: "config",
        description: "Bot runtime configuration (admin only).",
        usage: &["config starter <unit_id>"],
        details: "Adjusts live bot configuration values such as the starter unit id used in the saga tutorial.",
        category: CommandCategory::Admin,
    },
];

/// Public helper returning all registered primary help command names.
/// Exposed for integration tests to ensure help coverage. Marked allow(dead_code)
/// because it's only referenced externally by tests.
#[allow(dead_code)]
pub fn all_command_names() -> Vec<&'static str> {
    COMMANDS.iter().map(|c| c.name).collect()
}

pub fn register() -> CreateCommand {
    // NOTE: Previously we enumerated every command name as a String choice ( >25 ),
    // which violated Discord's max 25 choices per option and caused a 400 INVALID FORM BODY.
    // We now expose a free-form string (no predefined choices) and rely on the
    // interactive select menu already implemented for discovery. This fixes the
    // BASE_TYPE_MAX_LENGTH / choices validation error while preserving UX.
    // Bump description with a lightweight version tag when schema changes to force
    // Discord to invalidate any stale cached command definition.
    CreateCommand::new("help")
        .description("Shows information about commands (v2)")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "command",
                "The specific command you want help with (free text)",
            )
            .required(false),
        )
}

fn create_command_select_menu() -> CreateActionRow {
    let options = COMMANDS
        .iter()
        .map(|cmd| {
            let mut opt =
                CreateSelectMenuOption::new(cmd.name, cmd.name).description(cmd.description);
            if let Some(em) = cmd.category.emoji().chars().next() {
                opt = opt.emoji(em);
            }
            opt
        })
        .collect();
    let select_menu = CreateSelectMenu::new(
        "help_select_command",
        CreateSelectMenuKind::String { options },
    )
    .placeholder("Select a command for more details...");
    CreateActionRow::SelectMenu(select_menu)
}

async fn create_help_embed(ctx: &Context, command_name_opt: Option<&str>) -> CreateEmbed {
    let prefix = {
        ctx.data
            .read()
            .await
            .get::<AppState>()
            .expect("Expected AppState in TypeMap.")
            .prefix
            .read()
            .await
            .clone()
    };
    let footer_text = format!("Current Prefix: {}", prefix);
    let mut embed = CreateEmbed::new()
        .footer(CreateEmbedFooter::new(footer_text))
        .color(0x5865F2);
    if AppState::from_ctx(ctx).await.is_none() {
        return embed
            .title("Help (limited)")
            .description("Internal state unavailable.");
    }

    match command_name_opt {
        Some(name) => {
            if let Some(cmd) = COMMANDS.iter().find(|c| c.name == name) {
                let usage_string = cmd
                    .usage
                    .iter()
                    .map(|u| format!("`{}{}`", prefix, u))
                    .collect::<Vec<_>>()
                    .join("\n");
                embed = embed
                    .title(format!("{} Command: {}", cmd.category.emoji(), cmd.name))
                    .field("Description", cmd.description, false)
                    .field("Usage", usage_string, false)
                    .field("Details", cmd.details, false);
            } else {
                embed = embed
                    .title("Command Not Found")
                    .description(format!("Sorry, I don't know a command called `{}`.", name))
                    .color(0xFF0000);
            }
        }
        None => {
            embed = embed.title("Help Menu").description(format!("Here are my available commands. For more details, use `{}help <command>` or select an option from the dropdown below.", prefix));
            let categories = [
                CommandCategory::General,
                CommandCategory::Economy,
                CommandCategory::Saga,
                CommandCategory::Games,
                CommandCategory::Admin,
            ];
            for category in categories {
                let command_list = get_commands_in_category(category);
                if !command_list.is_empty() {
                    embed = embed.field(
                        format!("{} {}", category.emoji(), category.name()),
                        command_list,
                        false,
                    );
                }
            }
            embed = embed.field("Rarity Legend", "Common < Rare < Epic < Legendary < Unique < Mythical < Fabled", false)
                .field("Bonding", "Use `/bond` or `bond` to equip one unit onto another. Stat bonus scales with equipped unit rarity & level. One equipped unit per host.", false);
        }
    }
    embed
}

fn get_commands_in_category(category: CommandCategory) -> String {
    COMMANDS
        .iter()
        .filter(|c| c.category == category)
        .map(|c| format!("`{}`", c.name))
        .collect::<Vec<_>>()
        .join(" ")
}

pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    let command_name = interaction
        .data
        .options
        .first()
        .and_then(|opt| opt.value.as_str());
    let embed = create_help_embed(ctx, command_name).await;
    let mut builder = CreateInteractionResponseMessage::new().embed(embed);
    if command_name.is_none() {
        builder = builder.components(vec![create_command_select_menu()]);
    }
    let response = serenity::builder::CreateInteractionResponse::Message(builder);
    interaction.create_response(&ctx.http, response).await.ok();
}

pub async fn handle_interaction(ctx: &Context, interaction: &mut ComponentInteraction) {
    let selected_command =
        if let ComponentInteractionDataKind::StringSelect { values } = &interaction.data.kind {
            &values[0]
        } else {
            return;
        };
    let embed = create_help_embed(ctx, Some(selected_command)).await;
    interaction.defer(&ctx.http).await.ok();
    let builder = EditInteractionResponse::new()
        .embed(embed)
        .components(vec![]);
    interaction.edit_response(&ctx.http, builder).await.ok();
}

pub async fn run_prefix(ctx: &Context, msg: &Message, args: Vec<&str>) {
    let command_name = args.first().map(|s| s.as_ref());
    let embed = create_help_embed(ctx, command_name).await;
    let mut builder = CreateMessage::new().embed(embed).reference_message(msg);
    if command_name.is_none() {
        builder = builder.components(vec![create_command_select_menu()]);
    }
    msg.channel_id.send_message(&ctx.http, builder).await.ok();
}
