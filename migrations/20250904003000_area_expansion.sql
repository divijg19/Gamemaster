-- Area & Enemy Expansion Migration
-- Adds beginner and advanced locations plus new enemy units and research data.
-- Forward-only.

-- New Items (research data for new species)
INSERT INTO items (item_id,name) VALUES
 (20,'Bear Research Data'),
 (21,'Spider Research Data')
ON CONFLICT DO NOTHING;

-- New Units (bandits, beasts, arachnids)
INSERT INTO units (name, description, base_attack, base_defense, base_health, is_recruitable, kind, rarity) VALUES
 ('Bandit','An opportunistic outlaw haunting the roads.',14,10,100,TRUE,'Human','Common'),
 ('Criminal','A hardened lawbreaker with ruthless tactics.',22,16,170,TRUE,'Human','Rare'),
 ('Bear','A towering ursine brute with crushing strength.',28,20,220,TRUE,'Pet','Epic'),
 ('Giant Spider','A venomous spider that weaves ambush webs.',20,14,140,TRUE,'Pet','Rare')
ON CONFLICT DO NOTHING;

-- Map Nodes (beginner & tougher)
-- area_id legend so far: 1 = Starting, 2 = Wilds/Low Frontier, 3 = Frontier / Mid, 4 = Deep Caverns
INSERT INTO map_nodes (area_id,name, description, story_progress_required, reward_coins, reward_unit_xp) VALUES
 (1,'City Outskirts','The edge of civilization; caravans and petty thieves.',0,20,6),
 (1,'Forest Opening','Soft light, timid creatures, a first real step outward.',0,28,7),
 (2,'The Mines','Abandoned shafts now home to skittering things.',1,45,11),
 (3,'Tangled Ravines','Winding gullies perfect for ambush predators.',2,70,15),
 (3,'Wolf Den','Heart of the pack; the air reeks of musk and challenge.',3,85,18),
 (4,'Cavern Entry','A yawning dark threshold breathing cold air.',4,110,22)
ON CONFLICT DO NOTHING;

-- Enemy linking (avoid duplicates by ON CONFLICT semantics)
-- City Outskirts: Slime + Bandit
INSERT INTO node_enemies (node_id, unit_id)
SELECT mn.node_id, u.unit_id FROM map_nodes mn JOIN units u ON u.name='Slime' WHERE mn.name='City Outskirts' ON CONFLICT DO NOTHING;
INSERT INTO node_enemies (node_id, unit_id)
SELECT mn.node_id, u.unit_id FROM map_nodes mn JOIN units u ON u.name='Bandit' WHERE mn.name='City Outskirts' ON CONFLICT DO NOTHING;
-- Forest Opening: Slime + Wolf
INSERT INTO node_enemies (node_id, unit_id)
SELECT mn.node_id, u.unit_id FROM map_nodes mn JOIN units u ON u.name='Slime' WHERE mn.name='Forest Opening' ON CONFLICT DO NOTHING;
INSERT INTO node_enemies (node_id, unit_id)
SELECT mn.node_id, u.unit_id FROM map_nodes mn JOIN units u ON u.name='Wolf' WHERE mn.name='Forest Opening' ON CONFLICT DO NOTHING;
-- The Mines: Giant Spider + Boar
INSERT INTO node_enemies (node_id, unit_id)
SELECT mn.node_id, u.unit_id FROM map_nodes mn JOIN units u ON u.name='Giant Spider' WHERE mn.name='The Mines' ON CONFLICT DO NOTHING;
INSERT INTO node_enemies (node_id, unit_id)
SELECT mn.node_id, u.unit_id FROM map_nodes mn JOIN units u ON u.name='Boar' WHERE mn.name='The Mines' ON CONFLICT DO NOTHING;
-- Tangled Ravines: Bandit + Wolf + Giant Spider
INSERT INTO node_enemies (node_id, unit_id)
SELECT mn.node_id, u.unit_id FROM map_nodes mn JOIN units u ON u.name='Bandit' WHERE mn.name='Tangled Ravines' ON CONFLICT DO NOTHING;
INSERT INTO node_enemies (node_id, unit_id)
SELECT mn.node_id, u.unit_id FROM map_nodes mn JOIN units u ON u.name='Wolf' WHERE mn.name='Tangled Ravines' ON CONFLICT DO NOTHING;
INSERT INTO node_enemies (node_id, unit_id)
SELECT mn.node_id, u.unit_id FROM map_nodes mn JOIN units u ON u.name='Giant Spider' WHERE mn.name='Tangled Ravines' ON CONFLICT DO NOTHING;
-- Wolf Den: Wolf + Alpha Wolf + Bear
INSERT INTO node_enemies (node_id, unit_id)
SELECT mn.node_id, u.unit_id FROM map_nodes mn JOIN units u ON u.name='Wolf' WHERE mn.name='Wolf Den' ON CONFLICT DO NOTHING;
INSERT INTO node_enemies (node_id, unit_id)
SELECT mn.node_id, u.unit_id FROM map_nodes mn JOIN units u ON u.name='Alpha Wolf' WHERE mn.name='Wolf Den' ON CONFLICT DO NOTHING;
INSERT INTO node_enemies (node_id, unit_id)
SELECT mn.node_id, u.unit_id FROM map_nodes mn JOIN units u ON u.name='Bear' WHERE mn.name='Wolf Den' ON CONFLICT DO NOTHING;
-- Cavern Entry: Bear + Giant Spider + Boar + Criminal
INSERT INTO node_enemies (node_id, unit_id)
SELECT mn.node_id, u.unit_id FROM map_nodes mn JOIN units u ON u.name='Bear' WHERE mn.name='Cavern Entry' ON CONFLICT DO NOTHING;
INSERT INTO node_enemies (node_id, unit_id)
SELECT mn.node_id, u.unit_id FROM map_nodes mn JOIN units u ON u.name='Giant Spider' WHERE mn.name='Cavern Entry' ON CONFLICT DO NOTHING;
INSERT INTO node_enemies (node_id, unit_id)
SELECT mn.node_id, u.unit_id FROM map_nodes mn JOIN units u ON u.name='Boar' WHERE mn.name='Cavern Entry' ON CONFLICT DO NOTHING;
INSERT INTO node_enemies (node_id, unit_id)
SELECT mn.node_id, u.unit_id FROM map_nodes mn JOIN units u ON u.name='Criminal' WHERE mn.name='Cavern Entry' ON CONFLICT DO NOTHING;

-- Rewards / Drops
-- Early zones (City Outskirts / Forest Opening) minor materials + low parchment chance
INSERT INTO node_rewards (node_id,item_id,quantity,drop_chance)
SELECT mn.node_id, 8, 1, 0.55 FROM map_nodes mn WHERE mn.name='City Outskirts' ON CONFLICT DO NOTHING; -- Slime Gel
INSERT INTO node_rewards (node_id,item_id,quantity,drop_chance)
SELECT mn.node_id, 9, 1, 0.35 FROM map_nodes mn WHERE mn.name='City Outskirts' ON CONFLICT DO NOTHING; -- Slime Research Data
INSERT INTO node_rewards (node_id,item_id,quantity,drop_chance)
SELECT mn.node_id, 14, 1, 0.08 FROM map_nodes mn WHERE mn.name='City Outskirts' ON CONFLICT DO NOTHING; -- Forest Contract Parchment (rare)
INSERT INTO node_rewards (node_id,item_id,quantity,drop_chance)
SELECT mn.node_id, 12, 1, 0.30 FROM map_nodes mn WHERE mn.name='Forest Opening' ON CONFLICT DO NOTHING; -- Wolf Research Data

-- The Mines (ore + spider + boar research)
INSERT INTO node_rewards (node_id,item_id,quantity,drop_chance)
SELECT mn.node_id, 2, 1, 0.40 FROM map_nodes mn WHERE mn.name='The Mines' ON CONFLICT DO NOTHING; -- Ore
INSERT INTO node_rewards (node_id,item_id,quantity,drop_chance)
SELECT mn.node_id, 21, 1, 0.35 FROM map_nodes mn WHERE mn.name='The Mines' ON CONFLICT DO NOTHING; -- Spider Research Data
INSERT INTO node_rewards (node_id,item_id,quantity,drop_chance)
SELECT mn.node_id, 13, 1, 0.32 FROM map_nodes mn WHERE mn.name='The Mines' ON CONFLICT DO NOTHING; -- Boar Research Data

-- Tangled Ravines (mixed predator research & parchments)
INSERT INTO node_rewards (node_id,item_id,quantity,drop_chance)
SELECT mn.node_id, 12, 1, 0.28 FROM map_nodes mn WHERE mn.name='Tangled Ravines' ON CONFLICT DO NOTHING; -- Wolf Research Data
INSERT INTO node_rewards (node_id,item_id,quantity,drop_chance)
SELECT mn.node_id, 21, 1, 0.26 FROM map_nodes mn WHERE mn.name='Tangled Ravines' ON CONFLICT DO NOTHING; -- Spider Research Data
INSERT INTO node_rewards (node_id,item_id,quantity,drop_chance)
SELECT mn.node_id, 14, 1, 0.15 FROM map_nodes mn WHERE mn.name='Tangled Ravines' ON CONFLICT DO NOTHING; -- Forest Contract Parchment

-- Wolf Den (high wolf & bear research, chance for greater potion)
INSERT INTO node_rewards (node_id,item_id,quantity,drop_chance)
SELECT mn.node_id, 12, 1, 0.42 FROM map_nodes mn WHERE mn.name='Wolf Den' ON CONFLICT DO NOTHING; -- Wolf Research Data
INSERT INTO node_rewards (node_id,item_id,quantity,drop_chance)
SELECT mn.node_id, 20, 1, 0.30 FROM map_nodes mn WHERE mn.name='Wolf Den' ON CONFLICT DO NOTHING; -- Bear Research Data
INSERT INTO node_rewards (node_id,item_id,quantity,drop_chance)
SELECT mn.node_id, 17, 1, 0.18 FROM map_nodes mn WHERE mn.name='Wolf Den' ON CONFLICT DO NOTHING; -- Greater Health Potion

-- Cavern Entry (advanced research + frontier parchment + focus tonic)
INSERT INTO node_rewards (node_id,item_id,quantity,drop_chance)
SELECT mn.node_id, 20, 1, 0.26 FROM map_nodes mn WHERE mn.name='Cavern Entry' ON CONFLICT DO NOTHING; -- Bear Research Data
INSERT INTO node_rewards (node_id,item_id,quantity,drop_chance)
SELECT mn.node_id, 21, 1, 0.24 FROM map_nodes mn WHERE mn.name='Cavern Entry' ON CONFLICT DO NOTHING; -- Spider Research Data
INSERT INTO node_rewards (node_id,item_id,quantity,drop_chance)
SELECT mn.node_id, 15, 1, 0.14 FROM map_nodes mn WHERE mn.name='Cavern Entry' ON CONFLICT DO NOTHING; -- Frontier Contract Parchment
INSERT INTO node_rewards (node_id,item_id,quantity,drop_chance)
SELECT mn.node_id, 19, 1, 0.12 FROM map_nodes mn WHERE mn.name='Cavern Entry' ON CONFLICT DO NOTHING; -- Focus Tonic

-- Quests
INSERT INTO quests (quest_type,title,description,objective_key,objective_goal,reward_coins)
VALUES ('Battle','Into the Outskirts','Chase away 3 Bandits near the city.','DefeatBandit',3,90)
ON CONFLICT DO NOTHING;
INSERT INTO quests (quest_type,title,description,objective_key,objective_goal,reward_coins)
VALUES ('Battle','Den Raid','Defeat an Alpha Wolf in its den.','DefeatAlphaWolf',1,260)
ON CONFLICT DO NOTHING;
INSERT INTO quests (quest_type,title,description,objective_key,objective_goal,reward_coins)
VALUES ('Battle','First Fang Study','Collect 2 Bear Research Data.','CollectItem:20',2,230)
ON CONFLICT DO NOTHING;

-- Tasks
INSERT INTO tasks (task_type,title,description,objective_key,objective_goal,reward_coins)
VALUES ('Daily','Clear the Outskirts','Defeat 2 Bandits.','DefeatBandit',2,55)
ON CONFLICT DO NOTHING;
INSERT INTO tasks (task_type,title,description,objective_key,objective_goal,reward_coins)
VALUES ('Daily','Study Bears','Collect 1 Bear Research Data.','CollectItem:20',1,85)
ON CONFLICT DO NOTHING;
INSERT INTO tasks (task_type,title,description,objective_key,objective_goal,reward_coins)
VALUES ('Weekly','Den Dominance','Defeat 10 Wolves anywhere.','DefeatWolf',10,600)
ON CONFLICT DO NOTHING;

-- End of area expansion
