-- Add migration script here

-- Add a column to track the user's daily work streak.
ALTER TABLE profiles
ADD COLUMN work_streak INTEGER NOT NULL DEFAULT 0;

-- NOTE: When you are ready to implement the full leveling system,
-- you will create another migration and add columns like these.
-- ALTER TABLE profiles ADD COLUMN fishing_xp BIGINT NOT NULL DEFAULT 0;
-- ALTER TABLE profiles ADD COLUMN fishing_level INTEGER NOT NULL DEFAULT 1;
-- ALTER TABLE profiles ADD COLUMN mining_xp BIGINT NOT NULL DEFAULT 0;
-- ALTER TABLE profiles ADD COLUMN mining_level INTEGER NOT NULL DEFAULT 1;
-- etc...