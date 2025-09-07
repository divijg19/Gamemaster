# Changelog

All notable changes to this project are documented here. The format follows a lightweight variant of
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and (pre‑1.0) version numbers MAY introduce
breaking changes in minor bumps (`0.x.0`). Dates use `YYYY-MM-DD`.

## [Unreleased]
### Added
- Tavern system: deterministic daily rotation (date‑seeded), per‑user rotation persistence, favor tiers & progress bar, hire confirmation, two‑step reroll (confirm / cancel) with cost & remaining count, session persistence (page + filter), filter buttons (All / Rare+ / Epic+ / Legendary+), and rotation diff caching.
- Tavern daily rotation in‑memory cache (avoids repeated deterministic rebuilds each interaction).
- World map node preview embeds with content fingerprint hashing to avoid stale UI after progression.
- Dedicated SagaView::Tavern integrated into navigation stack (distinct from legacy Recruit view) with refresh/back row support.
- Filter abstraction (`filter_units`) & dynamic filter button rendering with active state.
- Two‑step reroll flow preserving page/filter state across confirmation.
- Additional SQL migrations (performance indexes, tavern favor/rotation tables, research & unit expansion, tavern expansion & rotation updates, saga AP/TP rebalance, tavern filter/rotation enhancements). 

### Changed
- Split generic Recruit view into explicit Tavern view (cleaner intent & future extensibility).
- Centralized filtering logic replacing scattered threshold checks.
- All tavern builds now use cached `build_tavern_state_cached` path (removes duplicate logic / reduces DB + compute churn).
- Navigation: uniform back / refresh rows applied to Tavern view; consistent sizing via shared button helpers.

### Fixed
- Corrupted match arms introduced during early reroll confirmation attempt (now replaced with clean confirm/cancel handlers).
- Potential stale tavern displays after reroll/hire by ensuring session + cache reconciliation and re-render.

### Removed
- Legacy uncached tavern builder (`build_tavern_state`).
- Obsolete `apply_filter` and per-enum threshold method (superseded by `filter_units`).

### In Progress / Planned
- Per-interaction unified context cache (profile + party + bonuses) to reduce multi-fetch overhead.
- Party combat snapshot caching for faster battle initialization.
- Expansion of world map procedural encounters & node diversity.
- Battle integration tests (snapshot + vitality verification).
- Rate limiting rollout to remaining non-saga handlers.

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
