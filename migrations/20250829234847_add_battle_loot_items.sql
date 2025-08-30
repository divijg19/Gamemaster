-- Add migration script here
-- Adds new items that can be obtained as loot from battles.
INSERT INTO items (item_id, name, description)
VALUES
    (8, 'Slime Gel', 'A sticky, gelatinous substance. Surprisingly useful in crafting.'),
    (9, 'Slime Research Data', 'Notes and observations on the combat patterns of slimes.');