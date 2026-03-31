-- Phase 3, Migration 3: policy_rules
CREATE TABLE policy_rules (
    id         UUID PRIMARY KEY,
    profile_id UUID NOT NULL REFERENCES agent_profiles(id),
    priority   INT NOT NULL,
    condition  JSONB NOT NULL,
    action     TEXT NOT NULL CHECK (action IN ('approve', 'block', 'escalate')),
    escalation JSONB,
    enabled    BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_policy_rules_profile ON policy_rules(profile_id, priority);

CREATE TRIGGER policy_rules_updated_at
    BEFORE UPDATE ON policy_rules
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
