//! This module implements the `work` command, allowing users to earn currency and resources.
//! It features multiple job types with unique payouts, cooldowns, and chances for rare rewards.

use crate::{AppState, database};
use chrono::{Duration, Utc};
use rand::Rng;
use serenity::builder::{CreateEmbed, CreateInteractionResponseFollowup, CreateMessage};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::model::user::User;
use serenity::prelude::*;

pub enum ResourceType {
    Fish(i64),
    Ores(i64),
    Gems(i64),
}
pub struct RareReward {
    pub name: &'static str,
    pub chance: f64,
}
pub struct WorkType {
    pub name: &'static str,
    pub min_payout: i64,
    pub max_payout: i64,
    pub cooldown: Duration,
    pub resource: fn() -> ResourceType,
    pub rare_reward: Option<RareReward>,
}

const JOBS: &[WorkType] = &[
    WorkType {
        name: "fishing",
        min_payout: 25,
        max_payout: 75,
        cooldown: Duration::minutes(30),
        resource: || ResourceType::Fish(rand::rng().random_range(3..=8)),
        rare_reward: Some(RareReward {
            name: "Golden Fish",
            chance: 0.05,
        }),
    },
    WorkType {
        name: "mining",
        min_payout: 100,
        max_payout: 300,
        cooldown: Duration::hours(2),
        resource: || ResourceType::Ores(rand::rng().random_range(5..=15)),
        rare_reward: Some(RareReward {
            name: "Large Geode",
            chance: 0.02,
        }),
    },
    WorkType {
        name: "coding",
        min_payout: 400,
        max_payout: 800,
        cooldown: Duration::hours(8),
        resource: || ResourceType::Gems(rand::rng().random_range(1..=3)),
        rare_reward: None,
    },
];

/// The shared core logic for the work command.
async fn perform_work(
    pool: &database::init::DbPool,
    user: &User,
    chosen_job: &WorkType,
) -> CreateEmbed {
    let profile = match database::profile::get_or_create_profile(pool, user.id).await {
        Ok(p) => p,
        Err(e) => {
            println!(
                "[WORK CMD] Error getting profile for user {}: {:?}",
                user.id, e
            );
            return CreateEmbed::new()
                .title("Error")
                .description("Could not fetch your profile.")
                .color(0xFF0000);
        }
    };

    if let Some(last_work) = profile.last_work {
        let next_work_time = last_work + chosen_job.cooldown;
        if Utc::now() < next_work_time {
            let remaining = next_work_time - Utc::now();
            return CreateEmbed::new()
                .title("On Cooldown")
                .description(format!(
                    "You can work again in **{}**.",
                    format_duration(remaining)
                ))
                .color(0xFF0000);
        }
    }

    let (rewards, reward_lines) = {
        let mut rng = rand::rng();

        // (✓) CORRECTED: The `rewards` struct is now initialized idiomatically as suggested by clippy.
        // We calculate the initial coin payout first...
        let initial_coins = rng.random_range(chosen_job.min_payout..=chosen_job.max_payout);
        // ...and then initialize the struct with that value, letting the rest default.
        let mut rewards = database::profile::WorkRewards {
            coins: initial_coins,
            ..Default::default()
        };

        let mut reward_lines = vec![format!("💰 You earned `{}` coins.", rewards.coins)];

        match (chosen_job.resource)() {
            ResourceType::Fish(amount) => {
                rewards.fish = amount;
                reward_lines.push(format!("🐟 You caught `{}` fish.", amount));
            }
            ResourceType::Ores(amount) => {
                rewards.ores = amount;
                reward_lines.push(format!("⛏️ You mined `{}` ores.", amount));
            }
            ResourceType::Gems(amount) => {
                rewards.gems = amount;
                reward_lines.push(format!("💎 You polished `{}` gems.", amount));
            }
        }

        if let Some(rare) = &chosen_job.rare_reward
            && rng.random_bool(rare.chance)
        {
            rewards.rare_finds = 1;
            reward_lines.push(format!("🌟 **RARE DROP!** You found a **{}**!", rare.name));
        }
        (rewards, reward_lines)
    };

    if let Err(e) = database::profile::update_work_rewards(pool, user.id, &rewards).await {
        println!(
            "[WORK CMD] Failed to update work rewards for user {}: {:?}",
            user.id, e
        );
        return CreateEmbed::new()
            .title("Error")
            .description("Failed to save your rewards.")
            .color(0xFF0000);
    }

    CreateEmbed::new()
        .title(format!(
            "Work Complete: {}!",
            chosen_job.name.to_uppercase()
        ))
        .description(reward_lines.join("\n"))
        .color(0x00FF00)
}

pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    if let Err(e) = interaction.defer_ephemeral(&ctx.http).await {
        println!("[WORK CMD] Failed to defer slash interaction: {:?}", e);
    }

    let pool = { ctx.data.read().await.get::<AppState>().unwrap().db.clone() };

    let job_name = interaction
        .data
        .options
        .iter()
        .find(|opt| opt.name == "job")
        .and_then(|opt| opt.value.as_str())
        .unwrap_or("fishing");

    let chosen_job = JOBS.iter().find(|j| j.name == job_name).unwrap();
    let user = &interaction.user;

    let embed = perform_work(&pool, user, chosen_job).await;
    let builder = CreateInteractionResponseFollowup::new().embed(embed);
    if let Err(e) = interaction.create_followup(&ctx.http, builder).await {
        println!("[WORK CMD] Failed to send slash followup: {:?}", e);
    }
}

pub async fn run_prefix(ctx: &Context, msg: &Message, args: Vec<&str>) {
    let pool = { ctx.data.read().await.get::<AppState>().unwrap().db.clone() };

    let job_name = args.first().cloned().unwrap_or("fishing");
    let chosen_job = match JOBS.iter().find(|j| j.name == job_name) {
        Some(job) => job,
        None => {
            let _ = msg
                .reply(
                    &ctx.http,
                    "That's not a valid job! Try `fishing`, `mining`, or `coding`.",
                )
                .await;
            return;
        }
    };

    let user = &msg.author;
    let embed = perform_work(&pool, user, chosen_job).await;
    let builder = CreateMessage::new().embed(embed).reference_message(msg);
    if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
        println!("[WORK CMD] Failed to send prefix response: {:?}", e);
    }
}

fn format_duration(dur: Duration) -> String {
    let hours = dur.num_hours();
    let minutes = dur.num_minutes() % 60;
    let seconds = dur.num_seconds() % 60;

    let mut parts = Vec::new();
    if hours > 0 {
        parts.push(format!("{}h", hours));
    }
    if minutes > 0 {
        parts.push(format!("{}m", minutes));
    }
    if hours == 0 && minutes == 0 && seconds > 0 {
        parts.push(format!("{}s", seconds));
    }

    if parts.is_empty() {
        "less than a second".to_string()
    } else {
        parts.join(" ")
    }
}
