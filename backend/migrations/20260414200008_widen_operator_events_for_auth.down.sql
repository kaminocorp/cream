-- Revert operator_events to Phase 15 schema.
ALTER TABLE operator_events ALTER COLUMN target_agent_id SET NOT NULL;

ALTER TABLE operator_events DROP CONSTRAINT IF EXISTS operator_events_event_type_check;
ALTER TABLE operator_events ADD CONSTRAINT operator_events_event_type_check
    CHECK (event_type IN (
        'agent_created', 'agent_updated', 'agent_key_rotated', 'policy_updated'
    ));
