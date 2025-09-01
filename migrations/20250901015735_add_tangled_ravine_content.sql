-- Add migration script here

-- Step 1: Create the new enemy, the "Dire Wolf".
-- ON CONFLICT (pet_id) ensures this won't fail if pet 5 already exists.
INSERT INTO pets (pet_id, name, description, base_attack, base_defense, base_health, is_tameable)
VALUES (5, 'Dire Wolf', 'A large and aggressive wolf with matted fur and sharp teeth.', 15, 8, 40, true)
ON CONFLICT (pet_id) DO NOTHING;

-- Step 2: Create the new items.
-- ON CONFLICT (name) is used here because item names must be unique.
INSERT INTO items (name, description, sell_price)
VALUES ('Wolf Pelt', 'A thick and durable pelt from a Dire Wolf.', 45)
ON CONFLICT (name) DO NOTHING;

INSERT INTO items (name, description, sell_price)
VALUES ('Dire Wolf Research Data', 'Data collected from battles with Dire Wolves. Crucial for taming attempts.', NULL)
ON CONFLICT (name) DO NOTHING;

-- Step 3: Create the new map area.
-- ON CONFLICT (name) is used here as area names are unique.
INSERT INTO map_areas (area_id, name) VALUES (2, 'Tangled Ravine')
ON CONFLICT (area_id) DO NOTHING;

-- Step 4: Create three new battle nodes.
-- ON CONFLICT (node_id) ensures this is safe to re-run.
INSERT INTO map_nodes (node_id, area_id, name, description, story_progress_required, reward_coins, reward_pet_xp)
VALUES
    (3, 2, 'Ravine Entrance', 'A single Dire Wolf snarls at you from a rocky outcrop.', 2, 150, 75),
    (4, 2, 'Winding Path', 'The path narrows. Two wolves eye you from the shadows.', 3, 250, 125),
    (5, 2, 'Wolf Den', 'The alpha of the pack blocks the way forward. It looks strong.', 4, 500, 300)
ON CONFLICT (node_id) DO NOTHING;

-- Step 5: Assign the Dire Wolf as an enemy to these new nodes.
-- The primary key is (node_id, pet_id), so we use that for the conflict target.
INSERT INTO node_enemies (node_id, pet_id)
VALUES
    (3, 5),
    (4, 5),
    (5, 5)
ON CONFLICT (node_id, pet_id) DO NOTHING;

-- Step 6: Define the loot for the new nodes.
-- The primary key is (node_id, item_id).
INSERT INTO node_rewards (node_id, item_id, quantity, drop_chance)
SELECT 3, item_id, 1, 1.0 FROM items WHERE name = 'Wolf Pelt'
ON CONFLICT (node_id, item_id) DO NOTHING;

INSERT INTO node_rewards (node_id, item_id, quantity, drop_chance)
SELECT 3, item_id, 1, 0.5 FROM items WHERE name = 'Dire Wolf Research Data'
ON CONFLICT (node_id, item_id) DO NOTHING;

INSERT INTO node_rewards (node_id, item_id, quantity, drop_chance)
SELECT 4, item_id, 2, 0.75 FROM items WHERE name = 'Wolf Pelt'
ON CONFLICT (node_id, item_id) DO NOTHING;

INSERT INTO node_rewards (node_id, item_id, quantity, drop_chance)
SELECT 4, item_id, 1, 0.75 FROM items WHERE name = 'Dire Wolf Research Data'
ON CONFLICT (node_id, item_id) DO NOTHING;

INSERT INTO node_rewards (node_id, item_id, quantity, drop_chance)
SELECT 5, item_id, 3, 1.0 FROM items WHERE name = 'Wolf Pelt'
ON CONFLICT (node_id, item_id) DO NOTHING;