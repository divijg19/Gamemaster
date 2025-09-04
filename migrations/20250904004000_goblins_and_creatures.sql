-- Add new creature units (Goblins etc.) and integrate into existing nodes.
-- Forward-only content expansion.

INSERT INTO units (name, description, base_attack, base_defense, base_health, is_recruitable, kind, rarity) VALUES
 ('Goblin Scout','A sneaky green scavenger.',14,8,70,TRUE,'Pet','Common'),
 ('Goblin Brute','Muscular goblin that relies on force.',20,12,130,TRUE,'Pet','Rare'),
 ('Forest Boar','A territorial boar with sharp tusks.',18,10,120,TRUE,'Pet','Common')
ON CONFLICT DO NOTHING;

-- Attach new enemies to Training Grounds (node 1) for variety (low chance of encountering stronger goblin)
INSERT INTO node_enemies (node_id, unit_id)
 SELECT 1, unit_id FROM units WHERE name IN ('Goblin Scout','Forest Boar') ON CONFLICT DO NOTHING;

-- Optional: Add a new node (2) if not existing for a tougher mix
INSERT INTO map_nodes (node_id, name, description, reward_coins, reward_unit_xp)
 VALUES (2,'Goblin Camp','A crude encampment buzzing with goblin activity.',35,10)
 ON CONFLICT DO NOTHING;

INSERT INTO node_enemies (node_id, unit_id)
 SELECT 2, unit_id FROM units WHERE name IN ('Goblin Scout','Goblin Brute','Slime') ON CONFLICT DO NOTHING;

-- Basic loot for Goblin Camp (reuse existing items; could add new later)
INSERT INTO node_rewards (node_id,item_id,quantity,drop_chance) VALUES
 (2,8,1,0.50), -- Slime Gel
 (2,9,1,0.30), -- Slime Research Data
 (2,10,1,0.12) -- Taming Lure
ON CONFLICT DO NOTHING;
