-- Add migration script here
-- migrations/20250901200000_create_quest_system.sql

-- An ENUM to represent the different types of quests.
-- We'll start with 'Battle' but can add 'Riddle', 'Gather', etc., later.
CREATE TYPE quest_type_enum AS ENUM ('Battle', 'Riddle');

-- An ENUM for the player's current status on a quest.
CREATE TYPE player_quest_status_enum AS ENUM ('Offered', 'Accepted', 'Completed', 'Failed');

-- Table 1: The master list of all possible quests.
CREATE TABLE quests (
    quest_id SERIAL PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    giver_name VARCHAR(100) NOT NULL DEFAULT 'Guild Clerk', -- The NPC/Guild offering the quest.
    difficulty VARCHAR(50) NOT NULL DEFAULT 'Normal', -- e.g., 'Easy', 'Normal', 'Hard', 'Boss'
    quest_type quest_type_enum NOT NULL,
    -- A generic key for the quest's objective.
    -- For 'Battle' quests, this could be a comma-separated list of enemy pet_ids (e.g., "1,1,2").
    -- For 'Riddle' quests, this would be the answer to the riddle.
    objective_key TEXT NOT NULL
);

-- Table 2: The rewards for each quest. A quest can have multiple rewards.
CREATE TABLE quest_rewards (
    quest_reward_id SERIAL PRIMARY KEY,
    quest_id INT NOT NULL REFERENCES quests(quest_id),
    reward_coins BIGINT,
    reward_item_id INT,
    reward_item_quantity INT
);

-- Table 3: Tracks the relationship between players and quests.
CREATE TABLE player_quests (
    player_quest_id SERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    quest_id INT NOT NULL REFERENCES quests(quest_id),
    status player_quest_status_enum NOT NULL DEFAULT 'Offered',
    offered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    accepted_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    -- Ensures a player can only have one instance of a specific quest at a time.
    UNIQUE(user_id, quest_id)
);

-- Indexes for faster lookups.
CREATE INDEX idx_player_quests_user_id_status ON player_quests (user_id, status);