-- Add migration script here
-- First, add the new craftable items to the master items table.
-- We are starting with a simple Health Potion.
INSERT INTO items (item_id, name, description)
VALUES
    (11, 'Health Potion', 'A basic potion that restores a small amount of health to a pet during battle.');

-- A master table for all crafting recipes.
CREATE TABLE recipes (
    recipe_id SERIAL PRIMARY KEY,
    -- The item_id of the item that this recipe creates.
    output_item_id INT NOT NULL REFERENCES items(item_id),
    output_quantity INT NOT NULL DEFAULT 1
);

-- A join table to define the ingredients required for each recipe.
CREATE TABLE recipe_ingredients (
    recipe_id INT NOT NULL REFERENCES recipes(recipe_id),
    item_id INT NOT NULL REFERENCES items(item_id),
    quantity INT NOT NULL,
    PRIMARY KEY (recipe_id, item_id)
);

-- Populate the tables with our first recipe: Health Potion.
-- It will require 5 Slime Gels and 1 Gem.
INSERT INTO recipes (recipe_id, output_item_id) VALUES (1, 11); -- Health Potion recipe

INSERT INTO recipe_ingredients (recipe_id, item_id, quantity)
VALUES
    (1, 8, 5), -- 5 Slime Gel (item_id 8)
    (1, 3, 1); -- 1 Gem (item_id 3)