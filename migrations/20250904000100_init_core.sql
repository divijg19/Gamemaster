-- Fresh baseline schema (combined essential tables) EXPANDED
-- NOTE: Forward-only baseline after reset. Future changes add new migration files.

-- Enums
DO $$ BEGIN CREATE TYPE unit_rarity AS ENUM ('Common','Rare','Epic','Legendary','Unique','Mythical','Fabled'); EXCEPTION WHEN duplicate_object THEN NULL; END $$;
DO $$ BEGIN CREATE TYPE unit_kind AS ENUM ('Human','Pet'); EXCEPTION WHEN duplicate_object THEN NULL; END $$;
DO $$ BEGIN CREATE TYPE task_type AS ENUM ('Daily','Weekly'); EXCEPTION WHEN duplicate_object THEN NULL; END $$;
DO $$ BEGIN CREATE TYPE quest_type_enum AS ENUM ('Battle','Riddle'); EXCEPTION WHEN duplicate_object THEN NULL; END $$;
DO $$ BEGIN CREATE TYPE player_quest_status_enum AS ENUM ('Offered','Accepted','Completed','Failed'); EXCEPTION WHEN duplicate_object THEN NULL; END $$;

-- Core profiles & economy
CREATE TABLE IF NOT EXISTS profiles (
  user_id BIGINT PRIMARY KEY,
  balance BIGINT NOT NULL DEFAULT 0,
  last_work TIMESTAMPTZ,
  work_streak INT NOT NULL DEFAULT 0,
  fishing_xp BIGINT NOT NULL DEFAULT 0,
  fishing_level INT NOT NULL DEFAULT 1,
  mining_xp BIGINT NOT NULL DEFAULT 0,
  mining_level INT NOT NULL DEFAULT 1,
  coding_xp BIGINT NOT NULL DEFAULT 0,
  coding_level INT NOT NULL DEFAULT 1
);

-- Items master
CREATE TABLE IF NOT EXISTS items (
  item_id INT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE
);
INSERT INTO items (item_id,name) VALUES
 (1,'Fish'),(2,'Ore'),(3,'Gem'),(4,'Golden Fish'),(5,'Large Geode'),(6,'Ancient Relic'),
 (7,'XP Booster'),(8,'Slime Gel'),(9,'Slime Research Data'),(10,'Taming Lure'),(11,'Health Potion')
ON CONFLICT DO NOTHING;

-- Inventories
CREATE TABLE IF NOT EXISTS inventories (
  user_id BIGINT REFERENCES profiles(user_id) ON DELETE CASCADE,
  item_id INT REFERENCES items(item_id) ON DELETE CASCADE,
  quantity BIGINT NOT NULL DEFAULT 0,
  PRIMARY KEY (user_id,item_id)
);

-- Units master
CREATE TABLE IF NOT EXISTS units (
  unit_id SERIAL PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  description TEXT,
  base_attack INT NOT NULL,
  base_defense INT NOT NULL,
  base_health INT NOT NULL,
  is_recruitable BOOLEAN NOT NULL DEFAULT TRUE,
  kind unit_kind NOT NULL DEFAULT 'Pet',
  rarity unit_rarity NOT NULL DEFAULT 'Common'
);

-- Player owned units
CREATE TABLE IF NOT EXISTS player_units (
  player_unit_id SERIAL PRIMARY KEY,
  user_id BIGINT REFERENCES profiles(user_id) ON DELETE CASCADE,
  unit_id INT REFERENCES units(unit_id) ON DELETE CASCADE,
  nickname TEXT,
  current_level INT NOT NULL DEFAULT 1,
  current_xp INT NOT NULL DEFAULT 0,
  current_attack INT NOT NULL,
  current_defense INT NOT NULL,
  current_health INT NOT NULL,
  is_in_party BOOLEAN NOT NULL DEFAULT FALSE,
  is_training BOOLEAN NOT NULL DEFAULT FALSE,
  training_stat TEXT,
  training_ends_at TIMESTAMPTZ,
  rarity unit_rarity NOT NULL,
  CONSTRAINT uq_unit_once UNIQUE (user_id, unit_id)
);
CREATE INDEX IF NOT EXISTS idx_player_units_user ON player_units(user_id);

-- Bonds (equippable pet system)
CREATE TABLE IF NOT EXISTS equippable_unit_bonds (
  bond_id SERIAL PRIMARY KEY,
  host_player_unit_id INT NOT NULL REFERENCES player_units(player_unit_id) ON DELETE CASCADE,
  equipped_player_unit_id INT NOT NULL REFERENCES player_units(player_unit_id) ON DELETE CASCADE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  is_equipped BOOLEAN NOT NULL DEFAULT TRUE
);

-- World map nodes
CREATE TABLE IF NOT EXISTS map_nodes (
  node_id SERIAL PRIMARY KEY,
  area_id INT NOT NULL DEFAULT 1,
  name TEXT NOT NULL,
  description TEXT,
  story_progress_required INT NOT NULL DEFAULT 0,
  reward_coins BIGINT NOT NULL DEFAULT 0,
  reward_unit_xp INT NOT NULL DEFAULT 5
);

CREATE TABLE IF NOT EXISTS node_enemies (
  node_id INT REFERENCES map_nodes(node_id) ON DELETE CASCADE,
  unit_id INT REFERENCES units(unit_id) ON DELETE CASCADE,
  PRIMARY KEY (node_id, unit_id)
);

CREATE TABLE IF NOT EXISTS node_rewards (
  node_id INT REFERENCES map_nodes(node_id) ON DELETE CASCADE,
  item_id INT REFERENCES items(item_id) ON DELETE CASCADE,
  quantity INT NOT NULL,
  drop_chance FLOAT4 NOT NULL,
  PRIMARY KEY (node_id,item_id)
);

-- Task system
CREATE TABLE IF NOT EXISTS tasks (
  task_id SERIAL PRIMARY KEY,
  task_type task_type NOT NULL,
  title TEXT NOT NULL,
  description TEXT NOT NULL,
  objective_key TEXT NOT NULL,
  objective_goal INT NOT NULL,
  reward_coins BIGINT,
  reward_item_id INT REFERENCES items(item_id),
  reward_item_quantity INT
);
CREATE TABLE IF NOT EXISTS player_tasks (
  player_task_id SERIAL PRIMARY KEY,
  user_id BIGINT REFERENCES profiles(user_id) ON DELETE CASCADE,
  task_id INT REFERENCES tasks(task_id) ON DELETE CASCADE,
  progress INT NOT NULL DEFAULT 0,
  is_completed BOOLEAN NOT NULL DEFAULT FALSE,
  assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  completed_at TIMESTAMPTZ,
  claimed_at TIMESTAMPTZ,
  UNIQUE(user_id, task_id)
);

-- Quests
CREATE TABLE IF NOT EXISTS quests (
  quest_id SERIAL PRIMARY KEY,
  quest_type quest_type_enum NOT NULL,
  title TEXT NOT NULL,
  description TEXT NOT NULL,
  giver_name TEXT DEFAULT 'Unknown',
  difficulty TEXT DEFAULT 'Normal',
  objective_key TEXT,
  objective_goal INT,
  reward_coins BIGINT,
  reward_item_id INT REFERENCES items(item_id),
  reward_item_quantity INT
);
CREATE TABLE IF NOT EXISTS player_quests (
  player_quest_id SERIAL PRIMARY KEY,
  user_id BIGINT REFERENCES profiles(user_id) ON DELETE CASCADE,
  quest_id INT REFERENCES quests(quest_id) ON DELETE CASCADE,
  status player_quest_status_enum NOT NULL DEFAULT 'Offered',
  offered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  accepted_at TIMESTAMPTZ,
  completed_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_player_quests_user ON player_quests(user_id);

-- Human encounters & drafted contracts
CREATE TABLE IF NOT EXISTS human_encounters (
  user_id BIGINT REFERENCES profiles(user_id) ON DELETE CASCADE,
  unit_id INT REFERENCES units(unit_id) ON DELETE CASCADE,
  defeats INT NOT NULL DEFAULT 0,
  last_defeated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (user_id, unit_id)
);
CREATE TABLE IF NOT EXISTS drafted_human_contracts (
  user_id BIGINT REFERENCES profiles(user_id) ON DELETE CASCADE,
  unit_id INT REFERENCES units(unit_id) ON DELETE CASCADE,
  drafted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  consumed BOOLEAN NOT NULL DEFAULT FALSE,
  PRIMARY KEY (user_id, unit_id)
);

-- Legacy human contract offers (still referenced by legacy module)
CREATE TABLE IF NOT EXISTS human_contract_offers (
  user_id BIGINT REFERENCES profiles(user_id) ON DELETE CASCADE,
  unit_id INT REFERENCES units(unit_id) ON DELETE CASCADE,
  cost BIGINT NOT NULL,
  offered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  expires_at TIMESTAMPTZ,
  accepted_at TIMESTAMPTZ,
  rarity_snapshot unit_rarity NOT NULL,
  PRIMARY KEY (user_id, unit_id)
);

-- Research progress for pet taming
CREATE TABLE IF NOT EXISTS unit_research_progress (
  user_id BIGINT REFERENCES profiles(user_id) ON DELETE CASCADE,
  unit_id INT REFERENCES units(unit_id) ON DELETE CASCADE,
  tamed_count INT NOT NULL DEFAULT 1,
  last_updated TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (user_id, unit_id)
);

-- Crafting system (recipes)
CREATE TABLE IF NOT EXISTS recipes (
  recipe_id SERIAL PRIMARY KEY,
  output_item_id INT NOT NULL REFERENCES items(item_id),
  output_quantity INT NOT NULL DEFAULT 1
);
CREATE TABLE IF NOT EXISTS recipe_ingredients (
  recipe_id INT REFERENCES recipes(recipe_id) ON DELETE CASCADE,
  item_id INT REFERENCES items(item_id) ON DELETE CASCADE,
  quantity INT NOT NULL,
  PRIMARY KEY (recipe_id, item_id)
);

-- Quest rewards mapping table
CREATE TABLE IF NOT EXISTS quest_rewards (
  quest_reward_id SERIAL PRIMARY KEY,
  quest_id INT REFERENCES quests(quest_id) ON DELETE CASCADE,
  reward_coins BIGINT,
  reward_item_id INT REFERENCES items(item_id),
  reward_item_quantity INT
);

-- Saga profile (AP/TP tracking)
CREATE TABLE IF NOT EXISTS player_saga_profile (
  user_id BIGINT PRIMARY KEY REFERENCES profiles(user_id) ON DELETE CASCADE,
  current_ap INT NOT NULL DEFAULT 20,
  max_ap INT NOT NULL DEFAULT 20,
  current_tp INT NOT NULL DEFAULT 10,
  max_tp INT NOT NULL DEFAULT 10,
  last_tp_update TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  story_progress INT NOT NULL DEFAULT 0
);

-- Bot configuration key/value store
CREATE TABLE IF NOT EXISTS bot_config (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

-- Basic seed data
INSERT INTO units (name, description, base_attack, base_defense, base_health, is_recruitable, kind, rarity) VALUES
 ('Squire','A trainee knight, eager but inexperienced.',12,10,100,TRUE,'Human','Common'),
 ('Archer','A nimble archer with a keen eye.',15,8,80,TRUE,'Human','Common'),
 ('Slime','A wobbling mass of goo.',8,6,60,TRUE,'Pet','Common'),
 ('Alpha Wolf','Leader of the pack; radiates feral authority.',22,14,160,TRUE,'Pet','Rare'),
 ('Scholar','A wandering academic seeking lost lore.',10,10,90,TRUE,'Human','Rare')
ON CONFLICT DO NOTHING;

INSERT INTO map_nodes (name, description, reward_coins, reward_unit_xp) VALUES
 ('Training Grounds','A place to test your skills.',25,8)
ON CONFLICT DO NOTHING;

INSERT INTO node_enemies (node_id, unit_id) SELECT 1, unit_id FROM units WHERE name='Slime' ON CONFLICT DO NOTHING;
INSERT INTO node_enemies (node_id, unit_id) SELECT 1, unit_id FROM units WHERE name='Alpha Wolf' ON CONFLICT DO NOTHING;

INSERT INTO node_rewards (node_id,item_id,quantity,drop_chance) VALUES
 (1,8,1,0.65), -- Slime Gel
 (1,9,1,0.40),  -- Slime Research Data
 (1,10,1,0.15)  -- Taming Lure
ON CONFLICT DO NOTHING;

-- Sample tasks
INSERT INTO tasks (task_type,title,description,objective_key,objective_goal,reward_coins)
VALUES ('Daily','Win a Battle','Achieve victory once.','WinBattle',1,50),
       ('Weekly','Train Your Units','Complete 3 training sessions.','CompleteTraining',3,250)
ON CONFLICT DO NOTHING;

-- Sample quest
INSERT INTO quests (quest_type,title,description,objective_key,objective_goal,reward_coins)
VALUES ('Battle','Cull the Slimes','Defeat 5 Slimes in the Training Grounds.','DefeatSlime',5,150)
ON CONFLICT DO NOTHING;
