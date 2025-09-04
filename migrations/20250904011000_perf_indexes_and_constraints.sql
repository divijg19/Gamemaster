-- Performance & integrity enhancements for bonds and player units
-- Adds composite indexes to speed up frequent lookups and ensures uniqueness of equipped bond pairs.

-- Ensure rapid lookup of a user's party & reserves ordered by party status then level
CREATE INDEX IF NOT EXISTS idx_player_units_user_party_level
ON player_units(user_id, is_in_party DESC, current_level DESC);

-- Fast retrieval of equipped bonds by host
CREATE INDEX IF NOT EXISTS idx_equippable_bonds_host_equipped
ON equippable_unit_bonds(host_player_unit_id, is_equipped)
WHERE is_equipped = TRUE;

-- Enforce that a specific equipped_player_unit_id can only be equipped to one host at a time when marked equipped
ALTER TABLE equippable_unit_bonds
ADD CONSTRAINT uq_equipped_unique_once UNIQUE (equipped_player_unit_id)
DEFERRABLE INITIALLY IMMEDIATE;

-- Prevent duplicate host+equipped rows (even if unequipped) for cleanliness
CREATE UNIQUE INDEX IF NOT EXISTS uq_equippable_bonds_pair
ON equippable_unit_bonds(host_player_unit_id, equipped_player_unit_id);
