-- Phase 6.5: Defense-in-depth CHECK on payments.status and improved policy rules index

-- Add CHECK constraint on payments.status to prevent invalid values
-- at the database level. The 10 variants match PaymentStatus in cream-models.
ALTER TABLE payments
    ADD CONSTRAINT chk_payments_status CHECK (
        status IN (
            'pending',
            'validating',
            'pending_approval',
            'approved',
            'submitted',
            'settled',
            'failed',
            'blocked',
            'rejected',
            'timed_out'
        )
    );

-- Replace policy_rules index to include enabled column.
-- The engine queries WHERE profile_id = $1 AND enabled = true ORDER BY priority,
-- so including enabled in the index avoids scanning disabled rules.
DROP INDEX IF EXISTS idx_policy_rules_profile;
CREATE INDEX idx_policy_rules_profile ON policy_rules(profile_id, enabled, priority);
