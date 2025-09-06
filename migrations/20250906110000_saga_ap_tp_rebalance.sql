-- Saga AP/TP rebalance
-- Start players at 4/4 AP and 10/10 TP instead of 0/10 AP and 0/100 TP.
-- Do NOT alter existing rows' current values if they have progressed; only patch zeros.

-- Update max values for existing profiles that still have legacy defaults.
UPDATE player_saga_profile
SET max_ap = 4
WHERE max_ap = 10 AND current_ap = 0 AND story_progress = 0; -- treat brand-new untouched profiles only

UPDATE player_saga_profile
SET max_tp = 10
WHERE max_tp = 100 AND current_tp = 0 AND story_progress = 0;

-- Ensure current values are at least the new starting points for untouched profiles.
UPDATE player_saga_profile
SET current_ap = 4
WHERE current_ap = 0 AND story_progress = 0;

UPDATE player_saga_profile
SET current_tp = 10
WHERE current_tp = 0 AND story_progress = 0;

-- Future inserts will still rely on code-level defaults (handled in saga.rs upsert path).
