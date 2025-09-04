-- Schema Hardening: enforce NOT NULL on foreign keys and quest metadata columns.
-- Forward-only migration. Ensures sqlx compile-time mappings align with non-Option Rust fields.

-- Player units FK columns should never be NULL.
ALTER TABLE player_units
    ALTER COLUMN user_id SET NOT NULL,
    ALTER COLUMN unit_id SET NOT NULL;

-- Player quests FK columns + ensure quest linkage integrity.
ALTER TABLE player_quests
    ALTER COLUMN user_id SET NOT NULL,
    ALTER COLUMN quest_id SET NOT NULL;

-- Quest rewards must always reference a quest.
ALTER TABLE quest_rewards
    ALTER COLUMN quest_id SET NOT NULL;

-- Quests metadata columns should be required (defaults already exist for giver_name & difficulty).
ALTER TABLE quests
    ALTER COLUMN giver_name SET NOT NULL,
    ALTER COLUMN difficulty SET NOT NULL,
    ALTER COLUMN objective_key SET NOT NULL;

-- Player tasks FK columns should be required.
ALTER TABLE player_tasks
    ALTER COLUMN user_id SET NOT NULL,
    ALTER COLUMN task_id SET NOT NULL;
