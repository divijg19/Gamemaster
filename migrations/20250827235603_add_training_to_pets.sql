-- Add migration script here
-- Adds columns to the player_pets table to track training status.
ALTER TABLE player_pets
ADD COLUMN is_training BOOLEAN NOT NULL DEFAULT FALSE,
ADD COLUMN training_stat TEXT, -- e.g., 'attack', 'defense'
ADD COLUMN training_ends_at TIMESTAMPTZ;