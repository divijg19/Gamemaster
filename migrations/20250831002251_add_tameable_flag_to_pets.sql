-- Add migration script here
-- Add a flag to the master pets table to indicate which can be tamed.
ALTER TABLE pets
ADD COLUMN is_tameable BOOLEAN NOT NULL DEFAULT FALSE;

-- Update existing pets. Wild Slime should be tameable.
-- The pet_id=4 corresponds to the Wild Slime from a previous migration.
UPDATE pets SET is_tameable = TRUE WHERE pet_id = 4;