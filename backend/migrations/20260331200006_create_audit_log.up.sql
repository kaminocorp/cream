-- Phase 3, Migration 6: audit_log (APPEND-ONLY)
CREATE TABLE audit_log (
    id                UUID PRIMARY KEY,
    timestamp         TIMESTAMPTZ NOT NULL DEFAULT now(),
    agent_id          UUID NOT NULL,
    agent_profile_id  UUID NOT NULL,
    payment_id        UUID REFERENCES payments(id),
    request           JSONB NOT NULL,
    justification     JSONB NOT NULL,
    policy_evaluation JSONB NOT NULL,
    routing_decision  JSONB,
    provider_response JSONB,
    final_status      TEXT NOT NULL,
    human_review      JSONB,
    on_chain_tx_hash  TEXT
);

-- No FK to agents/agent_profiles: audit records intentionally survive agent deletion

-- Secondary indexes (Vision Section 8.2)
CREATE INDEX idx_audit_agent ON audit_log(agent_id);
CREATE INDEX idx_audit_timestamp ON audit_log(timestamp);
CREATE INDEX idx_audit_status ON audit_log(final_status);
CREATE INDEX idx_audit_category ON audit_log USING gin ((justification->'category'));
CREATE INDEX idx_audit_amount ON audit_log USING btree (((request->>'amount')::numeric));

-- Append-only enforcement: block UPDATE and DELETE at the database level
CREATE OR REPLACE FUNCTION prevent_audit_mutation()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'audit_log is append-only: % operations are forbidden', TG_OP;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER audit_log_no_update
    BEFORE UPDATE ON audit_log
    FOR EACH ROW EXECUTE FUNCTION prevent_audit_mutation();

CREATE TRIGGER audit_log_no_delete
    BEFORE DELETE ON audit_log
    FOR EACH ROW EXECUTE FUNCTION prevent_audit_mutation();
