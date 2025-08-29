-- Add migration script here
-- Step 1: Create the new `items` table to store item definitions.
CREATE TABLE items (
    item_id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    sell_price BIGINT
);

-- Step 2: Create the new `inventories` table to link users to items.
CREATE TABLE inventories (
    inventory_id SERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES profiles(user_id) ON DELETE CASCADE,
    item_id INTEGER NOT NULL REFERENCES items(item_id) ON DELETE CASCADE,
    quantity BIGINT NOT NULL DEFAULT 0,
    UNIQUE(user_id, item_id) -- Ensures a user can't have two rows for the same item.
);

-- Step 3: Populate the `items` table with the items from your `item.rs` enum.
-- The IDs (1, 2, 3...) should correspond to the order in your enum for easy mapping.
INSERT INTO items (item_id, name, description, sell_price) VALUES
(1, 'Fish', 'A common fish. Good for selling in bulk.', 10),
(2, 'Ore', 'A chunk of raw, unprocessed ore.', 50),
(3, 'Gem', 'A polished, valuable gemstone.', 250),
(4, 'GoldenFish', 'An incredibly rare and valuable fish.', 1000),
(5, 'LargeGeode', 'A heavy, unassuming rock. Cannot be sold.', NULL);

-- Step 4: (Optional but Recommended) Migrate existing item counts to the new system.
-- This ensures no user loses their items during the upgrade.
INSERT INTO inventories (user_id, item_id, quantity)
SELECT user_id, 1, fish FROM profiles WHERE fish > 0;

INSERT INTO inventories (user_id, item_id, quantity)
SELECT user_id, 2, ores FROM profiles WHERE ores > 0;

INSERT INTO inventories (user_id, item_id, quantity)
SELECT user_id, 3, gems FROM profiles WHERE gems > 0;

-- Step 5: Remove the old, now-redundant columns from the `profiles` table.
ALTER TABLE profiles
DROP COLUMN fish,
DROP COLUMN ores,
DROP COLUMN gems,
DROP COLUMN rare_finds; -- `rare_finds` is now represented by items like GoldenFish/LargeGeode.