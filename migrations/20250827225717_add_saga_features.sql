-- Add migration script here
-- The master table for all possible pets/units in the game.
CREATE TABLE pets (
    pet_id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    base_attack INT NOT NULL DEFAULT 10,
    base_defense INT NOT NULL DEFAULT 10,
    base_health INT NOT NULL DEFAULT 100
);

-- The new profile extension for the main game loop.
CREATE TABLE player_saga_profile (
    user_id BIGINT PRIMARY KEY REFERENCES profiles(user_id) ON DELETE CASCADE,
    current_ap INT NOT NULL DEFAULT 4,
    max_ap INT NOT NULL DEFAULT 4,
    current_tp INT NOT NULL DEFAULT 5,
    max_tp INT NOT NULL DEFAULT 5,
    last_tp_update TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    story_progress INT NOT NULL DEFAULT 0
);

-- A join table to track which pets each player owns and their individual stats.
CREATE TABLE player_pets (
    player_pet_id SERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES profiles(user_id) ON DELETE CASCADE,
    pet_id INT NOT NULL REFERENCES pets(pet_id),
    nickname VARCHAR(255),
    current_level INT NOT NULL DEFAULT 1,
    current_xp INT NOT NULL DEFAULT 0,
    current_attack INT NOT NULL,
    current_defense INT NOT NULL,
    current_health INT NOT NULL,
    is_in_party BOOLEAN NOT NULL DEFAULT FALSE
);

-- Create an index for faster lookups of a player's pets.
CREATE INDEX ON player_pets (user_id);