# Gamemaster Bot

A feature-rich Discord game bot written in Rust (Serenity + SQLx + Tokio) featuring an evolving RPG mode (The Gamemaster Saga), economy, research & bonding systems, and several mini‑games.

## ✨ Key Features
- **Gamemaster Saga**: Turn-based progression with Action Points (AP), Training Points (TP), world map nodes, quests, and battles.
- **Party & Army Management**: Maintain a 5‑unit active party plus a larger army roster; rarity & leveling determine power.
- **Bonding System**: Equip (bond) one unit onto another for stat augment bonuses; cached & summarized in the party UI.
- **Research System**: Passive bonuses unlocked by collecting research data drops (TTL caches for performance).
- **Contracts**: Encounter units in battle, progress defeat counts, draft & accept contracts to recruit them.
- **Training**: Spend TP to asynchronously train units for stat growth; UI auto refreshes after actions.
- **Quests & Quest Log**: Accept battle quests; quest battles disable certain actions (e.g., recruiting/contracts) and give structured rewards.
- **Battle Engine**: Turn-based, logs actions, supports vitality mitigation and bonded equipment bonuses.
- **Mini-Games**: Blackjack, Poker, Rock/Paper/Scissors with modular game trait architecture.
- **Caching Layer**: Short TTL layer for saga profiles, bonds, equipment bonuses, research, and party mapping (hit/miss stats exposed via `/adminutil cachestats`).
- **Global Navigation**: Consistent cross-command navigation row (Saga / Party / Train) with capped nav stack depth.
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

## 🚀 Getting Started
1. Install Rust (stable) and Postgres.
2. Set environment variables (or a `.env` file):
   - `DISCORD_TOKEN` – Bot token
   - `DATABASE_URL` – Postgres connection string
3. Run migrations:
```
sqlx migrate run
```
4. Build & run:
```
cargo run
```
5. (Optional) Register slash commands by starting the bot once; Discord may take a minute to propagate.

## 🧪 Testing
Run all logic tests:
```
cargo test --tests
```
Current coverage includes leveling, TP recharge, cache stats, and first‑time tutorial flow. Add integration tests (battle snapshots, quest completion) as the saga expands.

## 🏗 Architecture Notes
- **Navigation**: Per-user stack of `NavState` objects; capped depth to avoid unbounded memory growth.
- **Caching Strategy**: Micro‑caches (TTL 2–5s) smooth out bursty interaction spam without risking stale long-term state.
- **Battle Bonuses**: Equipment & bond bonuses are pre-applied before the first render; mitigation summary appended on victory.
- **Consistency**: All cross-domain menus append a global nav row; saga battle only shows it on terminal phases to reduce clutter.

## 🔐 Data Integrity & Performance
- Unique constraints on equipped bonds and host/equipped pairs.
- Partial & composite indexes: party ordering, equipped bonds, etc.
- Cache hit/miss counters to guide future optimization (view via `/adminutil cachestats`).

## 🗺 Roadmap (Short-Term)
- Unified per-interaction context cache (profile + party + bonuses)
- Party snapshot caching for faster battle initialization
- Rate limiting across all handlers (saga implemented first)
- Expanded map node progression & procedural encounter generation
- Quest reward variety & scaling
- Admin telemetry & live metrics command

## 🧩 Contributing
1. Fork or branch from `test` (staging) then open PR.
2. Keep patches focused; include tests for new logic where feasible.
3. Run `cargo fmt` / `cargo clippy` if added (not yet enforced, but recommended).

## 📦 Releasing
1. Update `CHANGELOG.md`.
2. Bump version in `Cargo.toml`.
3. Tag & push.

## 📜 License
Currently proprietary / undisclosed. Add SPDX + license file before public release.

## 📑 Changelog
See `CHANGELOG.md` for detailed history.

---
_This README reflects the state as of version 0.1.0 (navigation + caching refactors and saga tutorial integration)._
