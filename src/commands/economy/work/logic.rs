//! Contains the core logic for the `work` command.

use super::jobs::{JOBS, Job};
use super::ui;
use crate::commands::economy::core;
use crate::database;
use chrono::Utc;
use rand::{Rng, rng};
use serenity::builder::CreateEmbed;
use serenity::model::user::User;
use sqlx::PgPool;

/// The shared core logic for the work command.
pub async fn perform_work(pool: &PgPool, user: &User, job_name: &str) -> CreateEmbed {
    let chosen_job = match JOBS.iter().find(|j| j.name == job_name) {
        Some(job) => job,
        None => {
            return ui::create_error_embed(
                "That's not a valid job! Try `fishing`, `mining`, or `coding`.",
            );
        }
    };

    let mut profile = match database::economy::get_or_create_profile(pool, user.id).await {
        Ok(p) => p,
        Err(_) => return ui::create_error_embed("Could not fetch your profile."),
    };

    if let Some(last_work) = profile.last_work {
        let next_work_time = last_work + chosen_job.cooldown;
        if Utc::now() < next_work_time {
            return ui::create_cooldown_embed(next_work_time - Utc::now());
        }
    }

    let streak = core::profile::check_and_update_streak(&mut profile);

    let (current_level, current_xp) = match chosen_job.name {
        "fishing" => (profile.fishing_level, profile.fishing_xp),
        "mining" => (profile.mining_level, profile.mining_xp),
        "coding" => (profile.coding_level, profile.coding_xp),
        _ => (1, 0),
    };

    let (rewards, reward_lines, streak_bonus) =
        calculate_rewards(current_level, chosen_job, streak);

    let (new_level, new_xp, level_up_info) =
        core::profile::handle_leveling(current_level, current_xp, rewards.xp);

    let _progression_update = if level_up_info.is_some() || rewards.xp > 0 {
        Some(database::models::ProgressionUpdate {
            job_name: chosen_job.name.to_string(),
            new_level,
            new_xp,
        })
    } else {
        None
    };

    // --- Main Transaction for Critical Data ---
    {
        let mut tx = match pool.begin().await {
            Ok(tx) => tx,
            Err(_) => return ui::create_error_embed("Failed to start database transaction."),
        };

        if database::economy::update_work_stats(&mut tx, user.id, &rewards, chosen_job.name)
            .await
            .is_err()
        {
            tx.rollback().await.ok();
            return ui::create_error_embed("Failed to save your work stats.");
        }

        if tx.commit().await.is_err() {
            return ui::create_error_embed("Failed to commit your rewards to the database.");
        }
    }
    // --- End of Transaction ---

    // (✓) FIXED: Update tasks *after* the main transaction is committed.
    // This prevents Executor type errors and ensures tasks are only updated on success.
    // We use .ok() because a failed task update should not show an error to the user.
    database::tasks::update_task_progress(pool, user.id, "Work", 1)
        .await
        .ok();

    for (item, quantity) in &rewards.items {
        database::tasks::update_task_progress(
            pool,
            user.id,
            &format!("GatherItem:{}", item.id()), // e.g., "GatherItem:1"
            *quantity as i32,
        )
        .await
        .ok();
    }

    ui::create_success_embed(
        chosen_job,
        &rewards,
        reward_lines,
        streak_bonus,
        level_up_info,
    )
}

fn calculate_rewards(
    current_level: i32,
    job: &Job,
    streak: i32,
) -> (database::models::WorkRewards, Vec<String>, i64) {
    let mut rng = rng();
    let base_coins = rng.random_range(job.min_payout..=job.max_payout);
    let streak_bonus = if streak > 1 {
        (base_coins as f64 * (streak as f64 * 0.01).min(0.25)).round() as i64
    } else {
        0
    };

    let mut rewards = database::models::WorkRewards {
        coins: base_coins + streak_bonus,
        xp: job.xp_gain,
        items: Vec::new(),
    };

    let mut reward_lines = vec![format!("💰 You earned `{}` coins.", base_coins)];

    let (item, amount) = (job.resource)(current_level);
    reward_lines.push(format!(
        "{} You found `{}` {}.",
        item.emoji(),
        amount,
        item.display_name()
    ));
    rewards.items.push((item, amount));

    if let Some((rare_item, chance)) = &job.rare_reward
        && rng.random_bool(*chance)
    {
        rewards.items.push((*rare_item, 1));
        reward_lines.push(format!(
            "🌟 **RARE DROP!** You found a **{}**!",
            rare_item.display_name()
        ));
    }

    (rewards, reward_lines, streak_bonus)
}
