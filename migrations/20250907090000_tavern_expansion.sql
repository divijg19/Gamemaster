-- Tavern expansion: add many recruitable units across rarities & kinds
-- Deterministic rotation logic in code will surface a subset daily.

INSERT INTO units (name, description, base_attack, base_defense, base_health, is_recruitable, kind, rarity)
VALUES
 -- Commons (Humans)
 ('Town Militia','A basic defender of the town.',6,8,28,TRUE,'Human','Common'),
 ('Scout Ranger','Fast eyes and quick blades.',8,5,24,TRUE,'Human','Common'),
 ('Apprentice Mage','Still learning the basics of spellcraft.',5,4,22,TRUE,'Human','Common'),
 ('Shield Squire','Trains under seasoned knights.',5,9,30,TRUE,'Human','Common'),
 ('Street Brawler','Unrefined but scrappy.',9,3,26,TRUE,'Human','Common'),
 -- Rares
 ('Battle Cleric','Combines martial skill with healing prayers.',9,9,34,TRUE,'Human','Rare'),
 ('Arcane Trickster','Illusions and daggers in harmony.',11,7,30,TRUE,'Human','Rare'),
 ('Beast Tamer','Understands the will of wild creatures.',10,8,32,TRUE,'Human','Rare'),
 ('War Drummer','Bolsters allies with thunderous rhythm.',7,10,36,TRUE,'Human','Rare'),
 -- Epics
 ('Runesmith Adept','Imbues weapons with latent runes.',13,11,40,TRUE,'Human','Epic'),
 ('Shadow Duelist','Strikes from veiled darkness.',16,8,34,TRUE,'Human','Epic'),
 ('Frost Warden','Chills foes and shields allies.',12,14,42,TRUE,'Human','Epic'),
 -- Legendary
 ('Storm Herald','Channels tempests into battle.',18,15,48,TRUE,'Human','Legendary'),
 ('Phoenix Champion','Rises anew, inspiring resilience.',20,16,50,TRUE,'Human','Legendary'),
 -- Pets (mixed rarities)
 ('Forest Wolf','Loyal companion from the woods.',7,5,25,TRUE,'Pet','Common'),
 ('Stone Turtle','Slow but nearly indestructible.',4,14,38,TRUE,'Pet','Rare'),
 ('Ember Drake','Small drake wreathed in flame.',15,9,35,TRUE,'Pet','Epic'),
 ('Celestial Griffin','Majestic guardian beast.',22,18,60,TRUE,'Pet','Legendary'),
 ('Temporal Sprite','Flickers between moments.',9,9,28,TRUE,'Pet','Rare'),
 ('Aether Serpent','Serpentine being of pure mana.',17,13,44,TRUE,'Pet','Epic'),
 ('Ancient Treant','Embodiment of the forest will.',14,20,70,TRUE,'Pet','Legendary'),
 ('Mythic Kitsune','Many-tailed spirit of cunning.',24,18,55,TRUE,'Pet','Mythical')
ON CONFLICT DO NOTHING;

-- Sequence adjustment
SELECT setval(pg_get_serial_sequence('units','unit_id'), (SELECT MAX(unit_id) FROM units));

-- Tavern favor & reroll tracking
CREATE TABLE IF NOT EXISTS tavern_favor (
	user_id BIGINT PRIMARY KEY REFERENCES profiles(user_id) ON DELETE CASCADE,
	favor INT NOT NULL DEFAULT 0,
	last_reroll TIMESTAMPTZ NULL,
	daily_rerolls INT NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_tavern_favor_favor ON tavern_favor(favor);
