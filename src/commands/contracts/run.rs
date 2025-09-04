use crate::database;
use chrono::{DateTime, Utc};
use serenity::builder::{
    CreateActionRow, CreateButton, CreateCommand, CreateEmbed, CreateEmbedFooter, CreateSelectMenu,
    CreateSelectMenuKind, CreateSelectMenuOption,
};
use serenity::model::application::CommandDataOptionValue;
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::Context;

pub fn register() -> CreateCommand {
    CreateCommand::new("contracts")
        .description("View human encounter progress, draft or accept contracts")
        .add_option(
            serenity::builder::CreateCommandOption::new(
                serenity::model::application::CommandOptionType::Integer,
                "draft",
                "Draft a contract for the specified human unit id (if ready)",
            )
            .required(false),
        )
        .add_option(
            serenity::builder::CreateCommandOption::new(
                serenity::model::application::CommandOptionType::Integer,
                "accept",
                "Accept a previously drafted contract (recruits the human)",
            )
            .required(false),
        )
}

pub struct ContractsView {
    pub description: String,
    pub embed: CreateEmbed,
    pub components: Vec<CreateActionRow>,
}

const CONTRACTS_PAGE_SIZE: usize = 25;

pub fn build_contracts_embed(
    statuses: &[crate::database::human::ContractStatusRow],
    drafted: &[crate::database::models::DraftedHumanContract],
    legacy_offers: &[crate::database::models::HumanContractOffer],
    page: usize,
) -> ContractsView {
    let mut lines: Vec<String> = Vec::new();
    if statuses.is_empty() {
        lines.push("No human encounters tracked yet. Fight human enemies to begin.".to_string());
    } else {
        lines.push("**Human Recruitment Progress**".to_string());
        let start = page * CONTRACTS_PAGE_SIZE;
        let end = (start + CONTRACTS_PAGE_SIZE).min(statuses.len());
        for (unit, defeats, required, drafted, recruited, last_defeat) in
            statuses.iter().skip(start).take(end - start)
        {
            let pct = (*defeats as f32 / *required as f32).min(1.0);
            let filled = (pct * 10.0).round() as i32; // 0-10
            let mut bar = String::new();
            for i in 0..10 {
                if i < filled {
                    bar.push('#');
                } else {
                    bar.push('-');
                }
            }
            let status = if *recruited {
                "Recruited"
            } else if *drafted {
                "Drafted"
            } else if *defeats >= *required {
                "Ready"
            } else {
                "Progress"
            };
            let last = last_defeat.map(relative_time).unwrap_or("-".into());
            lines.push(format!(
                "`{:>3}` {:<18} [{}] {}/{} {:<9} {}",
                unit.unit_id,
                unit.name.chars().take(18).collect::<String>(),
                bar,
                defeats,
                required,
                status,
                last
            ));
        }
        lines.push(
            "Select a Ready human in Draft menu to create a contract; then Accept menu to recruit."
                .to_string(),
        );
    }
    // After main progress, show drafted summary & any legacy offers (not paginated, concise)
    if !drafted.is_empty() {
        lines.push("".into());
        lines.push("**Drafted Contracts**".into());
        for d in drafted.iter().take(5) {
            // cap display
            let age = relative_time(d.drafted_at);
            lines.push(format!(
                "Unit {:>3} drafted {}{}",
                d.unit_id,
                age,
                if drafted.len() > 5 && d == drafted.last().unwrap() {
                    " (more...)"
                } else {
                    ""
                }
            ));
        }
    }
    if !legacy_offers.is_empty() {
        lines.push("".into());
        lines.push("**Legacy Offers**".into());
        for o in legacy_offers.iter().take(5) {
            // cap display
            let age = relative_time(o.offered_at);
            lines.push(format!(
                "Unit {:>3} cost {} ({}{})",
                o.unit_id,
                o.cost,
                age,
                if legacy_offers.len() > 5 && o == legacy_offers.last().unwrap() {
                    ", more..."
                } else {
                    ""
                }
            ));
        }
    }
    let total_pages = if statuses.is_empty() {
        1
    } else {
        ((statuses.len() - 1) / CONTRACTS_PAGE_SIZE) + 1
    };
    let mut embed = CreateEmbed::new()
        .title("Contracts")
        .description(lines.join("\n"));
    if total_pages > 1 {
        embed = embed.footer(CreateEmbedFooter::new(format!(
            "Page {}/{}",
            page + 1,
            total_pages
        )));
    }
    // Build component rows
    let mut rows: Vec<CreateActionRow> = Vec::new();
    // Draft select
    let ready: Vec<_> = statuses
        .iter()
        .filter(|(_u, d, req, drafted, recruited, _)| !*drafted && !*recruited && *d >= *req)
        .collect();
    if !ready.is_empty() {
        let opts: Vec<_> = ready
            .iter()
            .take(25)
            .map(|(u, _d, _r, _dr, _rec, _last)| {
                CreateSelectMenuOption::new(
                    u.name.chars().take(18).collect::<String>(),
                    u.unit_id.to_string(),
                )
            })
            .collect();
        rows.push(CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                "contracts_draft_select",
                CreateSelectMenuKind::String { options: opts },
            )
            .placeholder("Draft contract for..."),
        ));
    }
    let drafted_list: Vec<_> = statuses
        .iter()
        .filter(|(_u, _d, _r, drafted, recruited, _)| *drafted && !*recruited)
        .collect();
    if !drafted_list.is_empty() {
        let opts: Vec<_> = drafted_list
            .iter()
            .take(25)
            .map(|(u, _d, _r, _dr, _rec, _last)| {
                CreateSelectMenuOption::new(
                    u.name.chars().take(18).collect::<String>(),
                    u.unit_id.to_string(),
                )
            })
            .collect();
        rows.push(CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                "contracts_accept_select",
                CreateSelectMenuKind::String { options: opts },
            )
            .placeholder("Accept contract for..."),
        ));
    }
    // Refresh + pagination + Play buttons row
    let mut nav_buttons = vec![
        CreateButton::new("contracts_refresh")
            .label("Refresh")
            .style(serenity::model::application::ButtonStyle::Secondary),
    ];
    if total_pages > 1 && page > 0 {
        nav_buttons.push(
            CreateButton::new(format!("contracts_page_{}", page - 1))
                .label("Prev")
                .style(serenity::model::application::ButtonStyle::Secondary),
        );
    }
    if total_pages > 1 && page + 1 < total_pages {
        nav_buttons.push(
            CreateButton::new(format!("contracts_page_{}", page + 1))
                .label("Next")
                .style(serenity::model::application::ButtonStyle::Secondary),
        );
    }
    nav_buttons.push(
        CreateButton::new("saga_play")
            .label("Play / Menu")
            .style(serenity::model::application::ButtonStyle::Primary),
    );
    rows.push(CreateActionRow::Buttons(nav_buttons));
    ContractsView {
        description: lines.join("\n"),
        embed,
        components: rows,
    }
}

fn relative_time(ts: DateTime<Utc>) -> String {
    let now = Utc::now();
    let secs = (now - ts).num_seconds();
    if secs < 60 {
        format!("{}s ago", secs)
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86_400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86_400)
    }
}

pub async fn run_slash(ctx: &Context, interaction: &mut CommandInteraction) {
    let Some(app_state) = crate::AppState::from_ctx(ctx).await else {
        return;
    };
    let db = &app_state.db;
    let draft_unit_id = interaction.data.options.iter().find_map(|o| {
        if o.name == "draft" {
            if let CommandDataOptionValue::Integer(v) = o.value {
                Some(v as i32)
            } else {
                None
            }
        } else {
            None
        }
    });
    let accept_unit_id = interaction.data.options.iter().find_map(|o| {
        if o.name == "accept" {
            if let CommandDataOptionValue::Integer(v) = o.value {
                Some(v as i32)
            } else {
                None
            }
        } else {
            None
        }
    });
    let mut lines: Vec<String> = Vec::new(); // will only hold action feedback before embed build
    // Optional debug: if user supplies a numeric argument in content (legacy style) treat as encounter id to surface raw encounter fields.
    if let Some(raw) = interaction
        .data
        .options
        .iter()
        .find(|o| o.name == "encounter_id")
        .and_then(|o| match o.value {
            CommandDataOptionValue::Integer(v) => Some(v),
            _ => None,
        })
        && let Ok(enc) = database::human::get_encounter_by_id(db, raw as i32).await
    {
        lines.push(format!(
            "Encounter (unit {}) user:{} defeats:{} last:{}",
            enc.unit_id, enc.user_id, enc.defeats, enc.last_defeated_at
        ));
    }
    if let Some(uid) = draft_unit_id {
        match database::human::draft_contract(db, interaction.user.id, uid).await {
            Ok(_) => lines.push(format!("📝 Drafted contract for unit {}.", uid)),
            Err(e) => lines.push(format!("⚠️ Draft failed: {}", e)),
        }
        app_state.invalidate_user_caches(interaction.user.id).await;
    }
    if let Some(uid) = accept_unit_id {
        match database::human::accept_drafted_contract(db, interaction.user.id, uid).await {
            Ok(name) => lines.push(format!(
                "✅ Accepted drafted contract: **{}** recruited!",
                name
            )),
            Err(e) => lines.push(format!("⚠️ Accept failed: {}", e)),
        }
        app_state.invalidate_user_caches(interaction.user.id).await;
    }
    let view =
        match database::human::list_contract_status_cached(&app_state, interaction.user.id).await {
            Ok(statuses) => {
                let drafted = database::human::list_drafted_contracts(db, interaction.user.id)
                    .await
                    .unwrap_or_default();
                let legacy = database::human::list_legacy_open_offers(db, interaction.user.id)
                    .await
                    .unwrap_or_default();
                build_contracts_embed(&statuses, &drafted, &legacy, 0)
            }
            Err(e) => ContractsView {
                description: format!("Error loading progress: {}", e),
                embed: CreateEmbed::new()
                    .title("Contracts")
                    .description(format!("Error loading progress: {}", e)),
                components: vec![CreateActionRow::Buttons(vec![
                    CreateButton::new("contracts_refresh")
                        .label("Refresh")
                        .style(serenity::model::application::ButtonStyle::Secondary),
                ])],
            },
        };
    let resp = serenity::builder::CreateInteractionResponseMessage::new()
        .embed(view.embed.clone())
        .components(view.components.clone());
    let _ = interaction
        .create_response(
            &ctx.http,
            serenity::builder::CreateInteractionResponse::Message(resp),
        )
        .await;
}

pub async fn run_prefix(ctx: &Context, msg: &Message, args: Vec<&str>) {
    let Some(app_state) = crate::AppState::from_ctx(ctx).await else {
        return;
    };
    let db = &app_state.db;
    if let Some(cmd) = args.first() {
        if *cmd == "draft"
            && let Some(id_s) = args.get(1)
            && let Ok(id) = id_s.parse::<i32>()
        {
            match database::human::draft_contract(db, msg.author.id, id).await {
                Ok(_) => {
                    let _ = msg
                        .reply(&ctx.http, format!("Drafted contract for {}", id))
                        .await;
                }
                Err(e) => {
                    let _ = msg.reply(&ctx.http, format!("Draft failed: {}", e)).await;
                }
            }
            return;
        }
        if *cmd == "accept"
            && let Some(id_s) = args.get(1)
            && let Ok(id) = id_s.parse::<i32>()
        {
            match database::human::accept_drafted_contract(db, msg.author.id, id).await {
                Ok(name) => {
                    let _ = msg.reply(&ctx.http, format!("Accepted: {}", name)).await;
                }
                Err(e) => {
                    let _ = msg.reply(&ctx.http, format!("Accept failed: {}", e)).await;
                }
            }
            return;
        }
    }
    if let Ok(rows) = database::human::list_contract_status_cached(&app_state, msg.author.id).await
    {
        let drafted = database::human::list_drafted_contracts(db, msg.author.id)
            .await
            .unwrap_or_default();
        let legacy = database::human::list_legacy_open_offers(db, msg.author.id)
            .await
            .unwrap_or_default();
        let view = build_contracts_embed(&rows, &drafted, &legacy, 0);
        let _ = msg
            .channel_id
            .send_message(
                &ctx.http,
                serenity::builder::CreateMessage::new()
                    .embed(view.embed)
                    .components(view.components),
            )
            .await;
    } else {
        let _ = msg.reply(&ctx.http, "Error loading contract status").await;
    }
}
