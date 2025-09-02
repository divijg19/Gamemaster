-- Add migration script here
-- Step 1: Add the new nullable 'claimed_at' column to track reward claims.
ALTER TABLE player_tasks
ADD COLUMN claimed_at TIMESTAMPTZ;

-- Step 2: Modify 'is_completed' to be non-nullable and default to false for new entries.
-- This makes our state tracking more explicit.
ALTER TABLE player_tasks
ALTER COLUMN is_completed SET NOT NULL,
ALTER COLUMN is_completed SET DEFAULT false;

-- Step 3: Backfill the new state for existing tasks.
-- If a task was already marked as completed (and thus auto-claimed its reward),
-- we set 'claimed_at' to its 'completed_at' time to preserve its claimed status.
UPDATE player_tasks
SET claimed_at = completed_at
WHERE is_completed = TRUE AND completed_at IS NOT NULL;