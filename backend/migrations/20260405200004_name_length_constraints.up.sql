-- Enforce name length bounds at the DB level, matching Rust-side MAX_NAME_LEN = 255.
-- Defence-in-depth: prevents oversized names from reaching the append-only audit ledger
-- via direct DB manipulation or future ORM changes.

ALTER TABLE agent_profiles
    ADD CONSTRAINT chk_agent_profiles_name_length
        CHECK (LENGTH(name) <= 255 AND LENGTH(TRIM(name)) > 0);

ALTER TABLE agents
    ADD CONSTRAINT chk_agents_name_length
        CHECK (LENGTH(name) <= 255 AND LENGTH(TRIM(name)) > 0);
