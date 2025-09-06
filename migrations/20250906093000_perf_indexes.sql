-- Performance indexes added on 2025-09-06
-- Focus: optimize frequent saga & training queries.

-- Composite ordering index for saga unit listing:
-- Query pattern: SELECT ... FROM player_units pu JOIN units u ... WHERE pu.user_id = $1
--   ORDER BY pu.is_in_party DESC, pu.current_level DESC
-- The PK (player_unit_id) doesn't aid this ordering; create composite to allow index-only scan.
CREATE INDEX IF NOT EXISTS idx_player_units_user_party_level
    ON player_units (user_id, is_in_party DESC, current_level DESC);

-- Training completion polling pattern:
--   WHERE user_id = $1 AND is_training = TRUE AND training_ends_at <= $2
-- Add partial index to accelerate lookups and range predicate.
CREATE INDEX IF NOT EXISTS idx_player_units_training_due
    ON player_units (user_id, training_ends_at)
    WHERE is_training = TRUE;

-- (Optional future) If research queries begin filtering by tamed_count thresholds or last_updated ranges,
-- consider adding an index on unit_research_progress(user_id, unit_id) – already covered by PK.
