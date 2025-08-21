//! This module implements a state-of-the-art, interactive help command.
//!
//! Features:
//! - A categorized main menu for easy browsing.
//! - An interactive dropdown menu for slash command users to get details in-place.
//! - Dynamic slash command option registration based on the `COMMANDS` array.
//! - A detailed view for specific commands.

use crate::AppState;
use serenity::all::ComponentInteractionDataKind;
use serenity::builder::{
    CreateActionRow, CreateCommand, CreateCommandOption, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponseMessage, CreateMessage, CreateSelectMenu, CreateSelectMenuKind,
    CreateSelectMenuOption, EditMessage,
};
use serenity::model::application::{CommandInteraction, CommandOptionType, ComponentInteraction};
use serenity::model::channel::Message;
use serenity::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum CommandCategory {
    General,
    Economy,
    Games,
    Admin,
}

impl CommandCategory {
    fn name(&self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Economy => "Economy",
            Self::Games => "Games",
            Self::Admin => "Admin",
        }
    }
    fn emoji(&self) -> &'static str {
        match self {
            Self::General => "🔧",
            Self::Economy => "💰",
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
    CommandInfo {
        name: "prefix",
        description: "Views or (admin only) sets the prefix.",
        usage: &["prefix", "prefix set <new_prefix>"],
        details: "Displays the current command prefix. Administrators can use the `set` subcommand to change it.",
        category: CommandCategory::Admin,
    },
];

pub fn register() -> CreateCommand {
    let command = CreateCommand::new("help").description("Shows information about commands");
    let mut option = CreateCommandOption::new(
        CommandOptionType::String,
        "command",
        "The specific command you want help with",
    )
    .required(false);
    for cmd in COMMANDS {
        option = option.add_string_choice(cmd.name, cmd.name);
    }
    command.add_option(option)
}

fn create_command_select_menu() -> CreateActionRow {
    let options = COMMANDS
        .iter()
        .map(|cmd| {
            CreateSelectMenuOption::new(cmd.name, cmd.name)
                .description(cmd.description)
                .emoji(cmd.category.emoji().chars().next().unwrap())
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
            embed = embed.title("Help Menu")
                 .description(format!("Here are my available commands. For more details, use `{}help <command>` or select an option from the dropdown below.", prefix));
            let categories = [
                CommandCategory::General,
                CommandCategory::Economy,
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
        .iter()
        .find(|opt| opt.name == "command")
        .and_then(|opt| opt.value.as_str());
    let embed = create_help_embed(ctx, command_name).await;
    let mut builder = CreateInteractionResponseMessage::new().embed(embed);
    if command_name.is_none() {
        builder = builder.components(vec![create_command_select_menu()]);
    }
    let response = serenity::builder::CreateInteractionResponse::Message(builder);
    if let Err(e) = interaction.create_response(&ctx.http, response).await {
        println!("[HELP CMD] Error sending initial slash response: {:?}", e);
    }
}

pub async fn handle_interaction(ctx: &Context, interaction: &mut ComponentInteraction) {
    // (✓) CORRECTED: The `StringSelect` variant is a struct, so we must destructure it
    // with `{ values }` to bind its `values` field to a new variable.
    let selected_command =
        if let ComponentInteractionDataKind::StringSelect { values } = &interaction.data.kind {
            &values[0]
        } else {
            return;
        };

    let embed = create_help_embed(ctx, Some(selected_command)).await;
    if let Err(e) = interaction.defer(&ctx.http).await {
        println!(
            "[HELP CMD] Failed to defer help dropdown interaction: {:?}",
            e
        );
    }

    let builder = EditMessage::new().embed(embed).components(vec![]);
    if let Err(e) = interaction.message.edit(&ctx.http, builder).await {
        println!("[HELP CMD] Error editing message for dropdown: {:?}", e);
    }
}

pub async fn run_prefix(ctx: &Context, msg: &Message, args: Vec<&str>) {
    let command_name = args.first().map(|s| s.as_ref());
    let embed = create_help_embed(ctx, command_name).await;
    let builder = CreateMessage::new().embed(embed).reference_message(msg);
    if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
        println!("[HELP CMD] Failed to send prefix response: {:?}", e);
    }
}
