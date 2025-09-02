-- Add migration script here
-- Add migration script here

-- Step 1: Create the "Alpha Dire Wolf" boss enemy.
-- It has significantly higher stats than a regular Dire Wolf (15/8/40).
-- We will use the next available pet_id, which is 6.
INSERT INTO pets (pet_id, name, description, base_attack, base_defense, base_health, is_tameable)
VALUES (6, 'Alpha Dire Wolf', 'A massive wolf with battle scars and intelligent, predatory eyes.', 25, 15, 100, true)
ON CONFLICT (pet_id) DO NOTHING;

-- Step 2: Update the "Wolf Den" (node_id = 5) to feature the new Alpha boss.
-- This replaces the regular Dire Wolf (pet_id = 5) with the Alpha (pet_id = 6).
UPDATE node_enemies
SET pet_id = 6
WHERE node_id = 5 AND pet_id = 5;

-- Step 3: Enhance the boss's loot table to make it more rewarding.
-- We will add a guaranteed high-value Gem (item_id = 3) to its drops.
INSERT INTO node_rewards (node_id, item_id, quantity, drop_chance)
SELECT 5, 3, 1, 1.0
ON CONFLICT (node_id, item_id) DO NOTHING;