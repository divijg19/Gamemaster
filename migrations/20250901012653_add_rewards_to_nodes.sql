-- Add migration script here
-- Add migration script here
ALTER TABLE map_nodes
ADD COLUMN reward_coins BIGINT NOT NULL DEFAULT 50,
ADD COLUMN reward_pet_xp INT NOT NULL DEFAULT 25;

-- Update existing nodes with specific values (optional but good practice)
UPDATE map_nodes SET reward_coins = 50, reward_pet_xp = 25 WHERE node_id = 1;
UPDATE map_nodes SET reward_coins = 100, reward_pet_xp = 50 WHERE node_id = 2;