-- Add migration script here
-- This migration creates the `profiles` table, which will store all user-specific economy data.

CREATE TABLE IF NOT EXISTS profiles (
    -- The user's unique Discord ID.
    -- BIGINT is used because Discord IDs are 64-bit numbers.
    -- It is the PRIMARY KEY, meaning it uniquely identifies each row.
    user_id BIGINT PRIMARY KEY NOT NULL,

    -- The user's currency balance.
    -- BIGINT is used to support very large numbers and prevent overflow.
    -- It defaults to 100, so new players get a starting balance.
    balance BIGINT NOT NULL DEFAULT 100,

    -- The timestamp of when the user last used the `/work` command.
    -- TIMESTAMPTZ (Timestamp with Time Zone) is crucial as it stores the time in UTC,
    -- preventing any issues with time zones and cooldown calculations.
    -- It can be NULL because a new player hasn't worked yet.
    last_work TIMESTAMPTZ
);