-- Add migration script here

-- Add columns for job-specific experience and levels.
ALTER TABLE profiles
ADD COLUMN fishing_xp BIGINT NOT NULL DEFAULT 0,
ADD COLUMN fishing_level INTEGER NOT NULL DEFAULT 1,
ADD COLUMN mining_xp BIGINT NOT NULL DEFAULT 0,
ADD COLUMN mining_level INTEGER NOT NULL DEFAULT 1,
ADD COLUMN coding_xp BIGINT NOT NULL DEFAULT 0,
ADD COLUMN coding_level INTEGER NOT NULL DEFAULT 1;