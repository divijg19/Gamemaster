-- Add migration script here
-- Defines distinct areas in the game world.
CREATE TABLE map_areas (
    area_id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE
);

-- Defines specific points of interest (nodes) on the world map.
CREATE TABLE map_nodes (
    node_id SERIAL PRIMARY KEY,
    area_id INT NOT NULL REFERENCES map_areas(area_id),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    story_progress_required INT NOT NULL DEFAULT 0
);

-- Links enemies (from the master `pets` table) to specific battle nodes.
CREATE TABLE node_enemies (
    node_id INT NOT NULL REFERENCES map_nodes(node_id),
    pet_id INT NOT NULL REFERENCES pets(pet_id),
    PRIMARY KEY (node_id, pet_id)
);

-- Links loot (from the master `items` table) to specific battle nodes.
CREATE TABLE node_rewards (
    node_id INT NOT NULL REFERENCES map_nodes(node_id),
    item_id INT NOT NULL REFERENCES items(item_id),
    quantity INT NOT NULL DEFAULT 1,
    drop_chance REAL NOT NULL DEFAULT 1.0, -- A value from 0.0 to 1.0
    PRIMARY KEY (node_id, item_id)
);

-- Populate the world with the first area and a few battle nodes.
INSERT INTO map_areas (area_id, name) VALUES (1, 'Whispering Woods');

INSERT INTO map_nodes (node_id, area_id, name, description, story_progress_required)
VALUES
    (1, 1, 'Forest Entrance', 'A lone slime bounces lazily near the edge of the woods.', 0),
    (2, 1, 'Shaded Grove', 'Two slimes lurk in the dappled sunlight beneath the trees.', 1);

-- The "Wild Slime" is pet_id 4 from our previous migration.
INSERT INTO node_enemies (node_id, pet_id)
VALUES
    (1, 4), -- Forest Entrance has one slime.
    (2, 4); -- Shaded Grove also has slimes (we can add logic for multiples later).

-- The Slime Gel is item_id 8, Research Data is item_id 9.
INSERT INTO node_rewards (node_id, item_id, quantity, drop_chance)
VALUES
    (1, 8, 1, 1.0), -- Forest Entrance guarantees 1 Slime Gel.
    (1, 9, 1, 0.5); -- Forest Entrance has a 50% chance to drop Research Data.