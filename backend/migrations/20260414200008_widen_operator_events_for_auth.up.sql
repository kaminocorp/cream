-- Widen operator_events to support auth and template events.
--
-- 1. Make target_agent_id nullable (auth events don't target a specific agent).
-- 2. Replace the CHECK constraint to include all Phase 16 event types.

ALTER TABLE operator_events ALTER COLUMN target_agent_id DROP NOT NULL;

-- Drop the old CHECK and add a wider one.
ALTER TABLE operator_events DROP CONSTRAINT IF EXISTS operator_events_event_type_check;
ALTER TABLE operator_events ADD CONSTRAINT operator_events_event_type_check
    CHECK (event_type IN (
        -- Phase 15 agent lifecycle events
        'agent_created', 'agent_updated', 'agent_key_rotated', 'policy_updated',
        -- Phase 16-B auth events
        'operator_registered', 'operator_login', 'operator_logout',
        'refresh_token_reuse_detected',
        -- Phase 16-G template events
        'template_applied'
    ));
