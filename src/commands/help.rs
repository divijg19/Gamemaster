//! This module implements the `help` command in both prefix and slash formats.
//! It provides a general overview of all commands or detailed info for a specific command.

use crate::AppState;
use serenity::builder::{
    CreateEmbed, CreateEmbedFooter, CreateInteractionResponseFollowup, CreateMessage,
};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::*;

/// A struct to hold all the descriptive information for a single command.
struct CommandInfo {
    name: &'static str,
    description: &'static str,
    usage: &'static [&'static str],
    details: &'static str,
    admin_only: bool,
}

/// The single source of truth for all command information.
/// To add a command to the help menu, simply add a new entry here.
const COMMANDS: &[CommandInfo] = &[
    CommandInfo {
        name: "ping",
        description: "Checks the bot's latency.",
        usage: &["ping"],
        details: "Pings the Discord gateway to check the bot's heartbeat latency. A quick way to see if the bot is responsive.",
        admin_only: false,
    },
    CommandInfo {
        name: "prefix",
        description: "Views or sets the command prefix.",
        usage: &["prefix", "prefix set <new_prefix>"],
        details: "Displays the current command prefix. Administrators can use the `set` subcommand to change it.",
        admin_only: true, // The 'set' part is admin-only
    },
    CommandInfo {
        name: "profile",
        description: "Displays your or another user's profile.",
        usage: &["profile", "profile @user"],
        details: "Shows your economic profile, including your coin balance and inventory of items gathered from working.",
        admin_only: false,
    },
    CommandInfo {
        name: "work",
        description: "Work a job to earn coins and resources.",
        usage: &["work <job_name>"],
        details: "Allows you to perform a job to earn rewards. Each job has a different cooldown and payout.\nAvailable jobs: `fishing`, `mining`, `coding`.",
        admin_only: false,
    },
    CommandInfo {
        name: "rps",
        description: "Challenge a user to a game of Rock, Paper, Scissors.",
        usage: &[
            "rps @user <bet_amount>",
            "rps @user <bet_amount> [-b N] [-r N]",
        ],
        details: "Starts a game of Rock, Paper, Scissors. You can specify a format like `-b 3` (Best of 3) or `-r 5` (Race to 5).",
        admin_only: false,
    },
];

/// The shared core logic that builds the appropriate help embed.
async fn create_help_embed(ctx: &Context, command_name_opt: Option<&str>) -> CreateEmbed {
    let prefix = {
        let data = ctx.data.read().await;
        let app_state = data.get::<AppState>().unwrap();
        app_state.prefix.read().await.clone()
    };

    let mut embed = CreateEmbed::new().footer(CreateEmbedFooter::new(format!(
        "My current prefix is: {}",
        prefix
    )));

    match command_name_opt {
        // Case: A specific command was requested (e.g., /help command:ping)
        Some(name) => {
            if let Some(cmd) = COMMANDS.iter().find(|c| c.name == name) {
                let usage_string = cmd
                    .usage
                    .iter()
                    .map(|u| format!("`{}{}`", prefix, u))
                    .collect::<Vec<_>>()
                    .join("\n");
                embed = embed
                    .title(format!("Help: {}", cmd.name))
                    .description(cmd.details)
                    .field("Usage", usage_string, false)
                    .color(0x5865F2);
            } else {
                embed = embed
                    .title("Command Not Found")
                    .description(format!("Sorry, I don't know a command called `{}`.", name))
                    .color(0xFF0000);
            }
        }
        // Case: No specific command, show the main menu.
        None => {
            embed = embed.title("Help Menu").description("Here are my available commands. For more details on a specific command, use `/help command:<name>`.");

            for cmd in COMMANDS.iter().filter(|c| !c.admin_only) {
                embed = embed.field(format!("`{}{}`", prefix, cmd.name), cmd.description, true);
            }

            embed = embed.field("\u{200B}", "**Admin Commands**", false); // Separator
            for cmd in COMMANDS.iter().filter(|c| c.admin_only) {
                embed = embed.field(format!("`{}{}`", prefix, cmd.name), cmd.description, true);
            }
        }
    }
    embed
}

/// Entry point for the `/help` slash command.
pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    interaction.defer_ephemeral(&ctx.http).await.ok();

    let command_name = interaction
        .data
        .options
        .iter()
        .find(|opt| opt.name == "command")
        .and_then(|opt| opt.value.as_str());

    let embed = create_help_embed(ctx, command_name).await;
    let builder = CreateInteractionResponseFollowup::new().embed(embed);
    interaction.create_followup(&ctx.http, builder).await.ok();
}

/// Entry point for the `!help` prefix command.
pub async fn run_prefix(ctx: &Context, msg: &Message, args: Vec<&str>) {
    let command_name = args.first().map(|s| s.as_ref());
    let embed = create_help_embed(ctx, command_name).await;
    let builder = CreateMessage::new().embed(embed).reference_message(msg);
    msg.channel_id.send_message(&ctx.http, builder).await.ok();
}
