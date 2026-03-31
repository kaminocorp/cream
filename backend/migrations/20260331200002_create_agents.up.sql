-- Phase 3, Migration 2: agents
CREATE TABLE agents (
    id           UUID PRIMARY KEY,
    profile_id   UUID NOT NULL REFERENCES agent_profiles(id),
    name         TEXT NOT NULL,
    api_key_hash TEXT NOT NULL,
    status       TEXT NOT NULL DEFAULT 'active'
                 CHECK (status IN ('active', 'suspended', 'revoked')),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_agents_profile ON agents(profile_id);
CREATE INDEX idx_agents_status ON agents(status);

CREATE TRIGGER agents_updated_at
    BEFORE UPDATE ON agents
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
