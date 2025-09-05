-- Baseline schema (recreated fresh on 2025-09-05)
-- This file establishes all current tables and types required by the code.
-- If deploying to a brand-new Shuttle project this is the ONLY migration needed.

-- Enums
DO $$ BEGIN
    CREATE TYPE unit_rarity AS ENUM ('Common','Rare','Epic','Legendary','Unique','Mythical','Fabled');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;
DO $$ BEGIN
    CREATE TYPE unit_kind AS ENUM ('Human','Pet');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;
DO $$ BEGIN
    CREATE TYPE task_type AS ENUM ('Daily','Weekly');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;
DO $$ BEGIN
    CREATE TYPE quest_type_enum AS ENUM ('Battle','Riddle');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;
DO $$ BEGIN
    CREATE TYPE player_quest_status_enum AS ENUM ('Offered','Accepted','Completed','Failed');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

-- Core economy profile
CREATE TABLE IF NOT EXISTS profiles (
    user_id BIGINT PRIMARY KEY,
    balance BIGINT NOT NULL DEFAULT 0,
    last_work TIMESTAMPTZ NULL,
    work_streak INT NOT NULL DEFAULT 0,
    fishing_xp BIGINT NOT NULL DEFAULT 0,
    fishing_level INT NOT NULL DEFAULT 1,
    mining_xp BIGINT NOT NULL DEFAULT 0,
    mining_level INT NOT NULL DEFAULT 1,
    coding_xp BIGINT NOT NULL DEFAULT 0,
    coding_level INT NOT NULL DEFAULT 1,
    inventory JSONB NOT NULL DEFAULT '{}'::jsonb
);

-- Saga profile (AP/TP + story progress)
CREATE TABLE IF NOT EXISTS player_saga_profile (
    user_id BIGINT PRIMARY KEY REFERENCES profiles(user_id) ON DELETE CASCADE,
    current_ap INT NOT NULL DEFAULT 0,
    max_ap INT NOT NULL DEFAULT 10,
    current_tp INT NOT NULL DEFAULT 0,
    max_tp INT NOT NULL DEFAULT 100,
    last_tp_update TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    story_progress INT NOT NULL DEFAULT 0
);

-- Units master list
CREATE TABLE IF NOT EXISTS units (
    unit_id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NULL,
    base_attack INT NOT NULL,
    base_defense INT NOT NULL,
    base_health INT NOT NULL,
    is_recruitable BOOLEAN NOT NULL DEFAULT TRUE,
    kind unit_kind NOT NULL,
    rarity unit_rarity NOT NULL
);

-- Player-owned units (party + training)
CREATE TABLE IF NOT EXISTS player_units (
    player_unit_id SERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES profiles(user_id) ON DELETE CASCADE,
    unit_id INT NOT NULL REFERENCES units(unit_id) ON DELETE RESTRICT,
    nickname TEXT NULL,
    current_level INT NOT NULL DEFAULT 1,
    current_xp INT NOT NULL DEFAULT 0,
    current_attack INT NOT NULL DEFAULT 0,
    current_defense INT NOT NULL DEFAULT 0,
    current_health INT NOT NULL DEFAULT 0,
    is_in_party BOOLEAN NOT NULL DEFAULT FALSE,
    is_training BOOLEAN NOT NULL DEFAULT FALSE,
    training_stat TEXT NULL,
    training_ends_at TIMESTAMPTZ NULL,
    rarity unit_rarity NOT NULL,
    UNIQUE(user_id, unit_id)
);
CREATE INDEX IF NOT EXISTS idx_player_units_user ON player_units(user_id);

-- Human encounters (defeats tracking)
CREATE TABLE IF NOT EXISTS human_encounters (
    user_id BIGINT NOT NULL REFERENCES profiles(user_id) ON DELETE CASCADE,
    unit_id INT NOT NULL REFERENCES units(unit_id) ON DELETE CASCADE,
    defeats INT NOT NULL DEFAULT 0,
    last_defeated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, unit_id)
);

-- Human contract offers (persistent offers)
CREATE TABLE IF NOT EXISTS human_contract_offers (
    user_id BIGINT NOT NULL REFERENCES profiles(user_id) ON DELETE CASCADE,
    unit_id INT NOT NULL REFERENCES units(unit_id) ON DELETE CASCADE,
    cost BIGINT NOT NULL,
    offered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NULL,
    accepted_at TIMESTAMPTZ NULL,
    rarity_snapshot unit_rarity NOT NULL,
    PRIMARY KEY (user_id, unit_id)
);

-- Drafted human contracts (temporary draft state)
CREATE TABLE IF NOT EXISTS drafted_human_contracts (
    user_id BIGINT NOT NULL REFERENCES profiles(user_id) ON DELETE CASCADE,
    unit_id INT NOT NULL REFERENCES units(unit_id) ON DELETE CASCADE,
    drafted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    consumed BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (user_id, unit_id)
);

-- Bond / equippable relations
CREATE TABLE IF NOT EXISTS equippable_unit_bonds (
    bond_id SERIAL PRIMARY KEY,
    host_player_unit_id INT NOT NULL REFERENCES player_units(player_unit_id) ON DELETE CASCADE,
    equipped_player_unit_id INT NOT NULL REFERENCES player_units(player_unit_id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_equipped BOOLEAN NOT NULL DEFAULT TRUE
);

-- Crafting base (items referenced via code enum mapping)
CREATE TABLE IF NOT EXISTS recipes (
    recipe_id SERIAL PRIMARY KEY,
    output_item_id INT NOT NULL,
    output_quantity INT NOT NULL DEFAULT 1
);
CREATE TABLE IF NOT EXISTS recipe_ingredients (
    recipe_id INT NOT NULL REFERENCES recipes(recipe_id) ON DELETE CASCADE,
    item_id INT NOT NULL,
    quantity INT NOT NULL DEFAULT 1,
    PRIMARY KEY (recipe_id, item_id)
);

-- Tasks master & player tasks
CREATE TABLE IF NOT EXISTS tasks (
    task_id SERIAL PRIMARY KEY,
    task_type task_type NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    objective_key TEXT NOT NULL,
    objective_goal INT NOT NULL,
    reward_coins BIGINT NULL,
    reward_item_id INT NULL,
    reward_item_quantity INT NULL
);
CREATE TABLE IF NOT EXISTS player_tasks (
    player_task_id SERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES profiles(user_id) ON DELETE CASCADE,
    task_id INT NOT NULL REFERENCES tasks(task_id) ON DELETE CASCADE,
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    progress INT NOT NULL DEFAULT 0,
    is_completed BOOLEAN NOT NULL DEFAULT FALSE,
    completed_at TIMESTAMPTZ NULL,
    claimed_at TIMESTAMPTZ NULL
);
CREATE INDEX IF NOT EXISTS idx_player_tasks_user ON player_tasks(user_id);

-- Quests master & rewards
CREATE TABLE IF NOT EXISTS quests (
    quest_id SERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    giver_name TEXT NULL,
    difficulty TEXT NULL,
    quest_type quest_type_enum NOT NULL,
    objective_key TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS quest_rewards (
    quest_reward_id SERIAL PRIMARY KEY,
    quest_id INT NOT NULL REFERENCES quests(quest_id) ON DELETE CASCADE,
    reward_coins BIGINT NULL,
    reward_item_id INT NULL,
    reward_item_quantity INT NULL
);

-- Player quest instances
CREATE TABLE IF NOT EXISTS player_quests (
    player_quest_id SERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES profiles(user_id) ON DELETE CASCADE,
    quest_id INT NOT NULL REFERENCES quests(quest_id) ON DELETE CASCADE,
    status player_quest_status_enum NOT NULL DEFAULT 'Offered',
    offered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    accepted_at TIMESTAMPTZ NULL,
    completed_at TIMESTAMPTZ NULL
);
CREATE INDEX IF NOT EXISTS idx_player_quests_user ON player_quests(user_id);

-- Config key/value store
CREATE TABLE IF NOT EXISTS bot_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE OR REPLACE FUNCTION trg_bot_config_updated_at() RETURNS trigger AS $$
BEGIN
    NEW.updated_at := NOW();
    RETURN NEW;
END; $$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS bot_config_set_updated ON bot_config;
CREATE TRIGGER bot_config_set_updated BEFORE UPDATE ON bot_config FOR EACH ROW EXECUTE FUNCTION trg_bot_config_updated_at();

-- Seed minimal data (optional safe idempotent seeds)
INSERT INTO bot_config (key,value) VALUES ('starter_unit_id','1') ON CONFLICT (key) DO NOTHING;

-- Example starter unit (id=1) if none exist yet
INSERT INTO units (name, description, base_attack, base_defense, base_health, is_recruitable, kind, rarity)
SELECT 'Novice Adventurer','Your first companion',5,5,20,TRUE,'Human','Common'
WHERE NOT EXISTS (SELECT 1 FROM units WHERE unit_id=1);
UPDATE units SET unit_id = 1 WHERE name='Novice Adventurer' AND unit_id <> 1;

-- Ensure player_units foreign key consistency if we forced unit_id =1 (Postgres sequence fix)
SELECT setval(pg_get_serial_sequence('units','unit_id'), (SELECT MAX(unit_id) FROM units));
