-- Phase 6.13: Add updated_at to webhook_endpoints for consistency with all
-- other mutable tables (agent_profiles, agents, policy_rules, payments,
-- virtual_cards).
ALTER TABLE webhook_endpoints
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now();

CREATE TRIGGER webhook_endpoints_updated_at
    BEFORE UPDATE ON webhook_endpoints
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
