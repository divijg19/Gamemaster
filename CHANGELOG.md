# Changelog

All notable changes to this project will be documented here.

The format roughly follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/). Versioning will adopt SemVer once reaching a public 1.0 milestone.

## [Unreleased]
### Added
- Planned per-interaction context cache (profile + party + bonuses)
- Party combat snapshot caching (design draft)
- Rate limiting rollout to remaining handlers (saga implemented)

### In Progress
- Expanded world map & procedural encounter design
- Battle integration tests (snapshot + vitality verification)

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
