# Changelog

All notable changes to this project are documented here. The format follows a lightweight variant of
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and (pre‑1.0) version numbers MAY introduce
breaking changes in minor bumps (`0.x.0`). Dates use `YYYY-MM-DD`.

## [Unreleased]
### Added
- Tavern rotation enhancements: deterministic daily list, per‑user persistence, favor tiers & progress, hire confirmation, two‑step reroll, rotation diff highlighting, rarity‑scaled hire costs (rounded), affordable stats (✔ markers, avg cost, affordable count).
- In‑memory tavern daily cache (date keyed) reducing rebuild churn.
- World map: node preview embeds; area grouping & navigation (A1..), focused area view (SagaView::MapArea) with persistent nav state.
- Difficulty-based battle node button styling and legend.
- Battle scaling: enemy stat scaling vs story progress gap; reward scaling (coins/XP) by avg enemy rarity (clamped ×2.25).
- Battle UI: condensed log (last 12), vitality mitigation summary, quick Map/Tavern nav buttons after Victory & Defeat.
- Help command UX: category buttons, persistent dropdown, saga scaling & totals fields, navigation section on single-command view.
 - Training menu: global navigation row now shown even when the player has no units (prevents dead‑end view).

### Changed
- Split generic Recruit view into dedicated Tavern view.
- Centralized filtering logic (legacy filters later removed) & cached tavern builder usage everywhere.
- Uniform back/refresh rows; map embed simplified (grouped by area, locked summary) with clearer legend.
- Post-battle flow streamlined with quick navigation buttons.
- Persistent help navigation components across interactions.
 - World Map and Node Preview UX: AP-aware "Start Battle" button labeling and disabling when AP=0; area view now caps action rows to Discord's 5-row limit.

### Fixed
- Stale tavern display after reroll/hire via consistent cache rebuild.
- Early battle nav clutter—only terminal phases append global nav.
- Misc unused variable & doc comment lint warnings.
 - Prevented component overflow interaction failures by capping world map/area rows to 5.
 - Replaced explicit counter loops with `enumerate` to satisfy clippy; resolved minor lints in saga UI.
 - Training "no units" view no longer strands the user; global nav always present.

### Removed
- Legacy uncached tavern builder (`build_tavern_state`).
- Obsolete rarity filter & pagination UI (superseded by lean 5+2 rotation design).
- `apply_filter` helper & per-enum threshold method (replaced, then removed).

### In Progress / Planned
- Per-interaction unified context cache (profile + party + bonuses).
- Party combat snapshot caching for faster battle init.
- Area pagination when >5 areas; richer area metadata (difficulty summary).
- Expanded procedural encounters & node diversity.
- Battle & map integration tests (snapshot, mitigation assertions).
- Rate limiting remaining non-saga handlers.
 - Tavern reroll hardening: single-transaction flow combining balance deduction, rotation overwrite, and reroll counter update.
 - Integration test to verify persistence of fallback enemy generation for empty nodes.

## [0.1.0] - 2025-09-04
### Added
- Gamemaster Saga first-time tutorial (starter hire / skip flow)
- Global cross-command navigation row (Saga / Party / Train)
- Bonding system UI integration with aggregated bonus summary
- Cache layer (profile, bond map, equipment bonuses, research) + hit/miss counters
- Admin cache inspection via `/adminutil cachestats`
- Quests & quest log UI entries; quest battle branching (recruit/contract restrictions)
- Vitality mitigation tracking & victory summary line
- Performance / integrity SQL migration: indexes & unique constraints for bonds
- Contracts UI with pagination & progress bars
- Battle engine refactor (shared attack logic, phase-based component sets)
- Tests: leveling multi-level, TP recharge, cache stats, tutorial path

### Changed
- Party view: parallel bond + equipment bonus fetch, compact bond listing
- Battle equipment bonuses applied at construction (removed interaction-time application flag)
- Navigation stack depth capped & duplicate early tutorial handler removed
- Map fallback ensures node availability beyond story progress 1

### Fixed
- Duplicate saga handler tutorial match causing unreachable pattern warning
- Over-eager nav rendering in battle (now only terminal phases)
- Party UI corruption & stale sequential cache fetch path

### Removed
- Obsolete `applied_equipment` runtime flag (bonuses now pre-applied)

---
Future releases will separate internal refactors from player-visible changes.

---
Guidelines: Use categories (Added / Changed / Deprecated / Removed / Fixed / Security). Keep internal refactors under Changed unless purely cosmetic. Group related Tavern or Saga changes for readability.
