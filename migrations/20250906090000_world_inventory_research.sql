-- Add world map, enemy/reward mapping, items master, inventories, and research progress tables.
-- This migration is additive to baseline 20250905000100 and MUST NOT modify prior schema objects.

-- Items master list (static enum-mapped). Minimal columns currently required by code: name.
CREATE TABLE IF NOT EXISTS items (
    item_id INT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT NULL
);

-- Seed known item ids (idempotent). Using display names from enum properties.
INSERT INTO items (item_id, name, description) VALUES
 (1,'Fish','A common fish. Good for selling in bulk.'),
 (2,'Ore','A chunk of raw, unprocessed ore.'),
 (3,'Gem','A polished, valuable gemstone.'),
 (4,'Golden Fish','An incredibly rare and valuable fish.'),
 (5,'Large Geode','A heavy rock that may contain something valuable.'),
 (6,'Ancient Relic','A mysterious artifact from a forgotten era.'),
 (7,'XP Booster','Doubles XP gain from working for one hour.'),
 (8,'Slime Gel','A sticky, gelatinous substance.'),
 (9,'Slime Research Data','Combat notes that could be used to tame a slime.'),
 (10,'Taming Lure','A lure used to attract and pacify wild units.'),
 (11,'Health Potion','Restores a small amount of health.'),
 (12,'Wolf Research Data','Observations on wolf behavior.'),
 (13,'Boar Research Data','Notes on boar aggression and patterns.'),
 (14,'Forest Contract Parchment','Blank contract to draft a local human recruit.'),
 (15,'Frontier Contract Parchment','Higher grade contract for seasoned humans.'),
 (16,'Scholar Research Notes','Dense annotations that accelerate discoveries.'),
 (17,'Greater Health Potion','Restores a large amount of health.'),
 (18,'Stamina Draft','Restores action stamina in the saga.'),
 (19,'Focus Tonic','Increases research drop rate briefly.'),
 (20,'Bear Research Data','Scrawlings on bear movement and power.'),
 (21,'Spider Research Data','Web pattern sketches and venom notes.')
ON CONFLICT (item_id) DO NOTHING;

-- Player inventories (normalized instead of JSON in profiles.inventory for scalable queries)
CREATE TABLE IF NOT EXISTS inventories (
    user_id BIGINT NOT NULL REFERENCES profiles(user_id) ON DELETE CASCADE,
    item_id INT NOT NULL REFERENCES items(item_id) ON DELETE RESTRICT,
    quantity BIGINT NOT NULL DEFAULT 0 CHECK (quantity >= 0),
    PRIMARY KEY (user_id, item_id)
);
CREATE INDEX IF NOT EXISTS idx_inventories_item ON inventories(item_id);

-- Research / Taming progress (accumulates encounters/tames for sub-Legendary)
CREATE TABLE IF NOT EXISTS unit_research_progress (
    user_id BIGINT NOT NULL REFERENCES profiles(user_id) ON DELETE CASCADE,
    unit_id INT NOT NULL REFERENCES units(unit_id) ON DELETE CASCADE,
    tamed_count INT NOT NULL DEFAULT 0,
    last_updated TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, unit_id)
);

-- World map nodes (battle / exploration nodes)
CREATE TABLE IF NOT EXISTS map_nodes (
    node_id SERIAL PRIMARY KEY,
    area_id INT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NULL,
    story_progress_required INT NOT NULL DEFAULT 0,
    reward_coins BIGINT NOT NULL DEFAULT 0,
    reward_unit_xp INT NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_map_nodes_area ON map_nodes(area_id);
CREATE INDEX IF NOT EXISTS idx_map_nodes_story_req ON map_nodes(story_progress_required);

-- Node enemies (many-to-many mapping of units to nodes)
CREATE TABLE IF NOT EXISTS node_enemies (
    node_id INT NOT NULL REFERENCES map_nodes(node_id) ON DELETE CASCADE,
    unit_id INT NOT NULL REFERENCES units(unit_id) ON DELETE RESTRICT,
    PRIMARY KEY (node_id, unit_id)
);

-- Node rewards (item drops & probabilities)
CREATE TABLE IF NOT EXISTS node_rewards (
    node_id INT NOT NULL REFERENCES map_nodes(node_id) ON DELETE CASCADE,
    item_id INT NOT NULL REFERENCES items(item_id) ON DELETE RESTRICT,
    quantity INT NOT NULL DEFAULT 1 CHECK (quantity > 0),
    drop_chance REAL NOT NULL DEFAULT 1.0 CHECK (drop_chance >= 0 AND drop_chance <= 1.0),
    PRIMARY KEY (node_id, item_id)
);

-- Example seed node (optional) to allow early testing; safe idempotent.
INSERT INTO map_nodes (node_id, area_id, name, description, story_progress_required, reward_coins, reward_unit_xp)
SELECT 1, 1, 'Forest Entrance', 'The edge of a quiet forest teeming with low-tier creatures.', 0, 25, 5
WHERE NOT EXISTS (SELECT 1 FROM map_nodes WHERE node_id = 1);
SELECT setval(pg_get_serial_sequence('map_nodes','node_id'), GREATEST((SELECT MAX(node_id) FROM map_nodes),1));
