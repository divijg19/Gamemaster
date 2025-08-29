-- Add migration script here
-- Insert the initial set of pets available for hire in the tavern.
INSERT INTO pets (name, description, base_attack, base_defense, base_health)
VALUES
    ('Squire', 'A trainee knight, eager but inexperienced.', 12, 10, 100),
    ('Archer', 'A nimble archer with a keen eye.', 15, 8, 80),
    ('Mage Apprentice', 'A student of the arcane arts, wielding raw magical power.', 18, 6, 70);