# Gamemaster Bot

Memorialized and dedicated to Yargit, original author.

A feature-rich Discord game bot written in Rust (Serenity + SQLx + Tokio) featuring an evolving RPG mode (The Gamemaster Saga), a deterministic daily Tavern recruitment system, economy, research & bonding systems, and several mini‑games.

## ✨ Features
- **Gamemaster Saga**: Turn-based progression with Action Points (AP), Training Points (TP), world map nodes, quests, and battles.
- **Tavern Recruitment**: Deterministic daily rotation (date‑seeded), per‑user rotation persistence, fame tiers & progress, rarity‑scaled hire costs, affordable count & average cost stats, two‑step reroll confirmation with remaining count, rotation diff highlighting.
- **Party & Army Management**: Maintain a 5‑unit active party plus a larger army roster; rarity & leveling determine power.
- **Bonding System**: Equip (bond) one unit onto another for stat augment bonuses; cached & summarized in the party UI.
- **Research System**: Passive bonuses unlocked by collecting research data drops (TTL caches for performance).
- **Contracts**: Encounter units in battle, progress defeat counts, draft & accept contracts to recruit them.
- **Training**: Spend TP to asynchronously train units for stat growth; UI auto refreshes after actions.
- **Quests & Quest Log**: Accept battle quests; quest battles disable certain actions (e.g., recruiting/contracts) and give structured rewards.
- **Battle Engine**: Turn-based, logs actions, supports vitality mitigation and bonded equipment bonuses.
- **Mini-Games**: Blackjack, Poker, Rock/Paper/Scissors with modular game trait architecture.
- **Caching Layer**: Short TTL layer for saga profiles, bonds, equipment bonuses, research, and party mapping (hit/miss stats exposed via `/adminutil cachestats`).
- **Global Navigation**: Consistent cross-command navigation row (Saga / Party / Train) with capped per-user nav stack.
- **Area Map Navigation**: Grouped world map nodes by area with focused area view and difficulty‑styled buttons (Easy / Even / Moderate / Hard).
  - AP-aware node buttons and Node Preview start action (disabled with clear label when AP=0); respects Discord's 5-row component limit to prevent overflow.
- **Enhanced Help System**: Interactive category buttons + dropdown selector, saga scaling & rarity information, persistent navigation within help embeds.
- **View System**: Distinct `SagaView::Tavern` vs legacy `Recruit` path for clearer lifecycle & future extensibility.
- **Migrations**: SQLx migrations include performance indexes and data integrity constraints.

## 🗂 Project Structure
```
src/
  commands/        # Slash & prefix command modules (saga, party, bond, research, etc.)
  interactions/    # Component interaction handlers & game session routing
  saga/            # Core saga domain modules (battle engine, leveling, map, quests)
  database/        # SQLx data access + domain queries
  services/        # Caching & saga profile service
  ui/              # Shared styling helpers & navigation
migrations/        # SQL schema & evolution scripts
tests/             # Unit & lightweight logic tests
```

## ⚙️ Tech Stack
- **Runtime**: Tokio async
- **Discord**: Serenity 0.12 (gateway, interactions, component UIs)
- **Database**: PostgreSQL via SQLx (compile‑time checked queries where possible)
- **Deployment**: Shuttle (runtime + shared Postgres)
- **Logging**: `tracing` with structured spans (cache hits/misses, profile fetches)

## 🚀 Getting started
1) Install Rust (stable) and PostgreSQL.

2) Configure environment variables (or a `.env` file):
  - `DISCORD_TOKEN` – your bot token
  - `DATABASE_URL` – Postgres connection string

3) Run migrations, then build and run the bot.

PowerShell (pwsh):

```powershell
sqlx migrate run
```
4. Build & run:
```
cargo run
```

Slash commands will register automatically on first run; Discord can take up to a minute to propagate changes.

## 🧪 Testing
Run logic tests:

```powershell
cargo test --tests
```

Current coverage includes leveling, TP recharge, cache stats, and first‑time tutorial flow. I’ll add more (battle snapshots, quest completion) as the saga expands.

## 🏗 Architecture Notes
- **Navigation**: Per-user stack of `NavState` objects; capped depth prevents unbounded memory growth; Tavern & Area Map views are first‑class variants.
- **Caching Strategy**: Micro‑caches (TTL 2–5s) plus a stabilized daily Tavern rotation cache (date-keyed) reduce redundant deterministic recompute.
- **Battle Bonuses**: Equipment & bond bonuses are pre-applied before the first render; mitigation summary appended on victory.
- **Consistency**: All cross-domain menus append a global nav row; saga battle only shows it on terminal phases to reduce clutter.
- **Tavern Simplicity**: Legacy rarity filters & pagination removed; streamlined 5 base + up to 2 unlock slots design.

## 🔐 Data Integrity & Performance
- Unique constraints on equipped bonds and host/equipped pairs.
- Partial & composite indexes: party ordering, equipped bonds, etc.
- Cache hit/miss counters to guide future optimization (view via `/adminutil cachestats`).

## 🗺 Short‑term plans
- Unified per‑interaction context cache (profile + party + bonuses)
- Party snapshot caching for faster battle initialization
- Extended world map progression & procedural encounter generation
- Battle snapshot integration tests (vitality / mitigation assertions)
- Rate limiting across remaining non‑saga handlers
- Quest reward variety & scaling
- Admin telemetry & live metrics command
- Additional Tavern UX polish (highlight newest rotation changes, richer fame tiers)
- Tavern reroll hardening: make reroll fully transactional (deduct coins + overwrite rotation + increment counter in one DB transaction)

## 🧩 Contributing
This is primarily a personal project. Issues and small PRs are welcome, but I may scope or defer changes that don’t align with the current roadmap. There’s no strict process—please keep patches focused and include tests when practical.

## 📦 Releasing
Releases are informal for now. I’ll update `CHANGELOG.md`, bump `Cargo.toml`, and tag versions when meaningful milestones land.

## 📜 License
Currently proprietary / undecided. I’ll add SPDX + a license file before any public release.

## 📑 Changelog
See `CHANGELOG.md` for the history.

---
_This README reflects the state after post‑0.1.0 Tavern integration & navigation refinements._
