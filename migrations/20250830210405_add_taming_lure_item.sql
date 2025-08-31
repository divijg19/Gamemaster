-- Add migration script here
-- Adds the Taming Lure item to the master items table.
INSERT INTO items (item_id, name, description)
VALUES
    (10, 'Taming Lure', 'A specially crafted lure used to attract and pacify wild creatures, making them easier to tame.');