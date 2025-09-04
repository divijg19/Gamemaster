-- Content Expansion: Areas 1-3, segmented research data, contract items, tavern recruits.
-- Assumes baseline init_core already applied.

-- New Items (research data per species, contract parchments, advanced consumables)
INSERT INTO items (item_id,name) VALUES
 (12,'Wolf Research Data'),
 (13,'Boar Research Data'),
 (14,'Forest Contract Parchment'),
 (15,'Frontier Contract Parchment'),
 (16,'Scholar Research Notes'),
 (17,'Greater Health Potion'),
 (18,'Stamina Draft'),
 (19,'Focus Tonic')
ON CONFLICT DO NOTHING;

-- Add second & third area nodes
INSERT INTO map_nodes (name, description, area_id, reward_coins, reward_unit_xp, story_progress_required) VALUES
 ('Mossy Clearing','Wild beasts roam under filtered sunlight.',2,40,10,0),
 ('Boar Run','A muddy trail patrolled by territorial boars.',2,55,12,1),
 ('Frontier Outpost','Human scouts and mercenaries test travelers.',3,70,14,2),
 ('Scholar Encampment','A protected camp with rare knowledge.',3,90,16,3)
ON CONFLICT DO NOTHING;

-- Seed new units (animals & humans)
INSERT INTO units (name, description, base_attack, base_defense, base_health, is_recruitable, kind, rarity) VALUES
 ('Wolf','A lean predator with a piercing howl.',14,9,85,TRUE,'Pet','Common'),
 ('Boar','A charging mass of tusks and muscle.',18,12,120,TRUE,'Pet','Rare'),
 ('Scout','A vigilant human adept at ambush tactics.',16,12,110,TRUE,'Human','Rare'),
 ('Mercenary Captain','A seasoned commander offering contracts.',24,18,180,TRUE,'Human','Epic')
ON CONFLICT DO NOTHING;

-- Link enemies to nodes (avoid duplicates)
INSERT INTO node_enemies (node_id, unit_id)
SELECT mn.node_id, u.unit_id FROM map_nodes mn JOIN units u ON u.name IN ('Wolf') WHERE mn.name='Mossy Clearing'
ON CONFLICT DO NOTHING;
INSERT INTO node_enemies (node_id, unit_id)
SELECT mn.node_id, u.unit_id FROM map_nodes mn JOIN units u ON u.name IN ('Boar') WHERE mn.name='Boar Run'
ON CONFLICT DO NOTHING;
INSERT INTO node_enemies (node_id, unit_id)
SELECT mn.node_id, u.unit_id FROM map_nodes mn JOIN units u ON u.name IN ('Scout') WHERE mn.name='Frontier Outpost'
ON CONFLICT DO NOTHING;
INSERT INTO node_enemies (node_id, unit_id)
SELECT mn.node_id, u.unit_id FROM map_nodes mn JOIN units u ON u.name IN ('Mercenary Captain') WHERE mn.name='Frontier Outpost'
ON CONFLICT DO NOTHING;

-- Node rewards expansion (baseline probabilities)
INSERT INTO node_rewards (node_id,item_id,quantity,drop_chance)
SELECT mn.node_id, 12, 1, 0.45 FROM map_nodes mn WHERE mn.name='Mossy Clearing' ON CONFLICT DO NOTHING; -- Wolf Research Data
INSERT INTO node_rewards (node_id,item_id,quantity,drop_chance)
SELECT mn.node_id, 13, 1, 0.40 FROM map_nodes mn WHERE mn.name='Boar Run' ON CONFLICT DO NOTHING; -- Boar Research Data
INSERT INTO node_rewards (node_id,item_id,quantity,drop_chance)
SELECT mn.node_id, 18, 1, 0.20 FROM map_nodes mn WHERE mn.name='Boar Run' ON CONFLICT DO NOTHING; -- Greater Health Potion
INSERT INTO node_rewards (node_id,item_id,quantity,drop_chance)
SELECT mn.node_id, 16, 1, 0.30 FROM map_nodes mn WHERE mn.name='Scholar Encampment' ON CONFLICT DO NOTHING; -- Scholar Research Notes
INSERT INTO node_rewards (node_id,item_id,quantity,drop_chance)
SELECT mn.node_id, 14, 1, 0.25 FROM map_nodes mn WHERE mn.name='Frontier Outpost' ON CONFLICT DO NOTHING; -- Forest Contract Parchment
INSERT INTO node_rewards (node_id,item_id,quantity,drop_chance)
SELECT mn.node_id, 15, 1, 0.20 FROM map_nodes mn WHERE mn.name='Scholar Encampment' ON CONFLICT DO NOTHING; -- Frontier Contract Parchment

-- Tavern initial recruit list (mark humans & high value pets as recruitable)
UPDATE units SET is_recruitable = TRUE WHERE name IN ('Squire','Archer','Scholar');

-- Story progression quest seeds (if missing minimal quest rows)
INSERT INTO quests (quest_type,title,description,objective_key,objective_goal,reward_coins)
VALUES ('Battle','Secure the Clearing','Defeat 3 Wolves.','DefeatWolf',3,120)
ON CONFLICT DO NOTHING;
INSERT INTO quests (quest_type,title,description,objective_key,objective_goal,reward_coins)
VALUES ('Battle','Break the Boar Line','Defeat 2 Boars.','DefeatBoar',2,180)
ON CONFLICT DO NOTHING;
INSERT INTO quests (quest_type,title,description,objective_key,objective_goal,reward_coins)
VALUES ('Battle','Scout the Frontier','Defeat a Mercenary Captain.','DefeatMercCaptain',1,250)
ON CONFLICT DO NOTHING;

-- Basic tasks referencing new content
INSERT INTO tasks (task_type,title,description,objective_key,objective_goal,reward_coins)
VALUES ('Daily','Study Wolves','Collect 1 Wolf Research Data.','CollectItem:12',1,60)
ON CONFLICT DO NOTHING;

-- End of content expansion