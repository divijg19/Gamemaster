-- Add migration script here
-- Adds columns for job-specific resources and rare finds.
ALTER TABLE profiles
ADD COLUMN fish BIGINT NOT NULL DEFAULT 0,
ADD COLUMN ores BIGINT NOT NULL DEFAULT 0,
ADD COLUMN gems BIGINT NOT NULL DEFAULT 0,
ADD COLUMN rare_finds BIGINT NOT NULL DEFAULT 0;