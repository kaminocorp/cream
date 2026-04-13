-- Operator event log for administrative (non-payment) actions.
--
-- The payment audit_log tracks payment lifecycle events; this table tracks
-- operator-initiated administrative mutations: agent creation, updates,
-- key rotations, and policy changes. Like audit_log, it is append-only.

CREATE TABLE operator_events (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    timestamp   TIMESTAMPTZ NOT NULL DEFAULT now(),
    event_type  TEXT        NOT NULL
        CHECK (event_type IN (
            'agent_created', 'agent_updated', 'agent_key_rotated', 'policy_updated'
        )),
    target_agent_id UUID   NOT NULL REFERENCES agents(id),
    details     JSONB       NOT NULL DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Append-only: block UPDATE and DELETE.
CREATE TRIGGER operator_events_no_update
    BEFORE UPDATE ON operator_events
    FOR EACH ROW EXECUTE FUNCTION prevent_audit_mutation();

CREATE TRIGGER operator_events_no_delete
    BEFORE DELETE ON operator_events
    FOR EACH ROW EXECUTE FUNCTION prevent_audit_mutation();

CREATE TRIGGER operator_events_no_truncate
    BEFORE TRUNCATE ON operator_events
    FOR EACH STATEMENT EXECUTE FUNCTION prevent_audit_truncate();

-- Indexes for common query patterns.
CREATE INDEX idx_operator_events_agent ON operator_events(target_agent_id);
CREATE INDEX idx_operator_events_timestamp ON operator_events(timestamp DESC);
CREATE INDEX idx_operator_events_type ON operator_events(event_type);
