-- Rename tavern_favor table and favor column to tavern_fame.fame
ALTER TABLE IF EXISTS tavern_favor RENAME TO tavern_fame;
ALTER TABLE tavern_fame RENAME COLUMN favor TO fame;

-- Optional compatibility view for older code paths (will be removed later)
DROP VIEW IF EXISTS tavern_favor;
CREATE VIEW tavern_favor AS
SELECT user_id, fame AS favor, daily_rerolls, last_reroll
FROM tavern_fame;

-- Note: indexes/constraints remain; if any were named on old table, consider renaming as needed.