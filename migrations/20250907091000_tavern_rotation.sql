-- Per-user tavern rotation + reroll tracking
CREATE TABLE IF NOT EXISTS tavern_user_rotation (
    user_id BIGINT PRIMARY KEY REFERENCES profiles(user_id) ON DELETE CASCADE,
    rotation INT[] NOT NULL,
    generated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    day DATE NOT NULL
);

-- Index for day queries (potential analytics)
CREATE INDEX IF NOT EXISTS idx_tavern_user_rotation_day ON tavern_user_rotation(day);
