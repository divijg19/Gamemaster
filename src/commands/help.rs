//! This module implements the `help` command in both prefix and slash formats.
//! It provides a well-designed, categorized overview of all commands or detailed info for a specific command.

use crate::AppState;
use serenity::builder::{
    CreateEmbed, CreateEmbedFooter, CreateInteractionResponseFollowup, CreateMessage,
};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum CommandCategory {
    General,
    Economy,
    Games,
    Admin,
}

/// A struct to hold all the descriptive information for a single command.
struct CommandInfo {
    name: &'static str,
    description: &'static str,
    usage: &'static [&'static str],
    details: &'static str,
    category: CommandCategory,
}

/// The single source of truth for all command information.
const COMMANDS: &[CommandInfo] = &[
    // General Commands
    CommandInfo {
        name: "ping",
        description: "Checks the bot's latency.",
        usage: &["ping"],
        details: "Pings the Discord gateway to check the bot's heartbeat latency. A quick way to see if the bot is responsive.",
        category: CommandCategory::General,
    },
    CommandInfo {
        name: "help",
        description: "Shows this help menu.",
        usage: &["help", "help <command>"],
        details: "Displays a list of all available commands or detailed information about a specific command.",
        category: CommandCategory::General,
    },
    // Economy Commands
    CommandInfo {
        name: "profile",
        description: "Displays your or another user's profile.",
        usage: &["profile", "profile @user"],
        details: "Shows your economic profile, including your coin balance and inventory of items gathered from working.",
        category: CommandCategory::Economy,
    },
    CommandInfo {
        name: "work",
        description: "Work a job to earn coins and resources.",
        usage: &["work <job_name>"],
        details: "Allows you to perform a job to earn rewards. Each job has a different cooldown and payout.\n**Available jobs:** `fishing`, `mining`, `coding`.",
        category: CommandCategory::Economy,
    },
    // Game Commands
    CommandInfo {
        name: "rps",
        description: "Challenge a user to Rock, Paper, Scissors.",
        usage: &["rps @user", "rps @user [-b N] [-r N]"],
        details: "Starts a game of Rock, Paper, Scissors. You can specify a format like `-b 3` (Best of 3) or `-r 5` (Race to 5).",
        category: CommandCategory::Games,
    },
    CommandInfo {
        name: "blackjack",
        description: "Play a game of Blackjack against the house.",
        usage: &["blackjack", "bj"],
        details: "Starts a game of single-player Blackjack. Try to get as close to 21 as possible without going over.",
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
];

/// The shared core logic that builds the appropriate help embed.
async fn create_help_embed(ctx: &Context, command_name_opt: Option<&str>) -> CreateEmbed {
    let prefix = {
        let data = ctx.data.read().await;
        let app_state = data
            .get::<AppState>()
            .expect("Expected AppState in TypeMap.");
        app_state.prefix.read().await.clone()
    };

    let footer_text = format!("Current Prefix: {} (Default is $)", prefix);
    let mut embed = CreateEmbed::new()
        .footer(CreateEmbedFooter::new(footer_text))
        .color(0x5865F2);

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
                    .title(format!(
                        "{} Command: {}",
                        get_category_emoji(cmd.category),
                        cmd.name
                    ))
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
            embed = embed.title("Help Menu")
                 .description(format!("Here are my available commands. For more details, use `{}help <command>` or `/help command:<command>`.", prefix));

            let general_cmds = get_commands_in_category(CommandCategory::General);
            let economy_cmds = get_commands_in_category(CommandCategory::Economy);
            let game_cmds = get_commands_in_category(CommandCategory::Games);
            let admin_cmds = get_commands_in_category(CommandCategory::Admin);

            if !general_cmds.is_empty() {
                embed = embed.field(
                    format!("{} General", get_category_emoji(CommandCategory::General)),
                    general_cmds,
                    false,
                );
            }
            if !economy_cmds.is_empty() {
                embed = embed.field(
                    format!("{} Economy", get_category_emoji(CommandCategory::Economy)),
                    economy_cmds,
                    false,
                );
            }
            if !game_cmds.is_empty() {
                embed = embed.field(
                    format!("{} Games", get_category_emoji(CommandCategory::Games)),
                    game_cmds,
                    false,
                );
            }
            if !admin_cmds.is_empty() {
                embed = embed.field(
                    format!("{} Admin", get_category_emoji(CommandCategory::Admin)),
                    admin_cmds,
                    false,
                );
            }
        }
    }
    embed
}

// (✓) CORRECTED: The full function body is now provided.
/// Helper function to format a list of command names for a category.
fn get_commands_in_category(category: CommandCategory) -> String {
    COMMANDS
        .iter()
        .filter(|c| c.category == category)
        .map(|c| format!("`{}`", c.name))
        .collect::<Vec<_>>()
        .join(" ")
}

// (✓) CORRECTED: The full function body is now provided.
/// Helper to get an emoji for a category.
fn get_category_emoji(category: CommandCategory) -> &'static str {
    match category {
        CommandCategory::General => "🔧",
        CommandCategory::Economy => "💰",
        CommandCategory::Games => "🎮",
        CommandCategory::Admin => "🛡️",
    }
}

/// Entry point for the `/help` slash command.
pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    if let Err(e) = interaction.defer_ephemeral(&ctx.http).await {
        println!("[HELP CMD] Failed to defer slash interaction: {:?}", e);
    }

    let command_name = interaction
        .data
        .options
        .iter()
        .find(|opt| opt.name == "command")
        .and_then(|opt| opt.value.as_str());

    let embed = create_help_embed(ctx, command_name).await;
    let builder = CreateInteractionResponseFollowup::new().embed(embed);
    if let Err(e) = interaction.create_followup(&ctx.http, builder).await {
        println!("[HELP CMD] Failed to send slash followup: {:?}", e);
    }
}

/// Entry point for the `!help` prefix command.
pub async fn run_prefix(ctx: &Context, msg: &Message, args: Vec<&str>) {
    let command_name = args.first().map(|s| s.as_ref());
    let embed = create_help_embed(ctx, command_name).await;
    let builder = CreateMessage::new().embed(embed).reference_message(msg);
    if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
        println!("[HELP CMD] Failed to send prefix response: {:?}", e);
    }
}
