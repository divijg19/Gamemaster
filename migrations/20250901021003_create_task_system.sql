-- Add migration script here

-- Step 1: Create a custom ENUM type for the different kinds of tasks.
CREATE TYPE task_type AS ENUM ('Daily', 'Weekly');

-- Step 2: Create the master table for all task definitions.
CREATE TABLE tasks (
    task_id SERIAL PRIMARY KEY,
    task_type task_type NOT NULL,
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    objective_key VARCHAR(100) NOT NULL UNIQUE,
    objective_goal INT NOT NULL DEFAULT 1,
    reward_coins BIGINT,
    reward_item_id INT REFERENCES items(item_id),
    reward_item_quantity INT
);

-- Step 3: Create the table to track individual player progress on tasks.
CREATE TABLE player_tasks (
    player_task_id SERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES profiles(user_id) ON DELETE CASCADE,
    task_id INT NOT NULL REFERENCES tasks(task_id) ON DELETE CASCADE,
    progress INT NOT NULL DEFAULT 0,
    is_completed BOOLEAN NOT NULL DEFAULT FALSE,
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    UNIQUE(user_id, task_id, assigned_at)
);

-- Step 4: Populate the 'tasks' table. ON CONFLICT ensures this is safe to re-run.
INSERT INTO tasks (task_type, title, description, objective_key, objective_goal, reward_coins)
VALUES
    ('Daily', 'Battle Victory', 'Win 3 battles on the World Map.', 'WIN_BATTLES', 3, 500),
    ('Daily', 'Work Hard', 'Work 5 times.', 'WORK_TIMES', 5, 250)
ON CONFLICT (objective_key) DO NOTHING;

-- (✓) FIXED: This weekly task now correctly references item_id 11 for 'Health Potion'
-- and uses a more systematic objective_key, both derived from your Item enum.
INSERT INTO tasks (task_type, title, description, objective_key, objective_goal, reward_item_id, reward_item_quantity)
VALUES
    ('Weekly', 'Master Crafter', 'Craft a Health Potion.', 'CRAFT_ITEM_11', 1, 11, 2)
ON CONFLICT (objective_key) DO NOTHING;