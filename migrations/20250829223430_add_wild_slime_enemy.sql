-- Add migration script here
-- Adds the first wild creature for players to encounter in battle.
INSERT INTO pets (name, description, base_attack, base_defense, base_health)
VALUES
    ('Wild Slime', 'A common creature of the woods, surprisingly resilient.', 8, 5, 50);