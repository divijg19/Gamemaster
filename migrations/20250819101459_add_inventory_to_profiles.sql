-- migrations/20250819101459_add_inventory_to_profiles.sql

-- Add a column to store player inventory.
-- Using JSONB is efficient for storing a list of item IDs and quantities.
ALTER TABLE profiles
ADD COLUMN inventory JSONB NOT NULL DEFAULT '{}'::jsonb;