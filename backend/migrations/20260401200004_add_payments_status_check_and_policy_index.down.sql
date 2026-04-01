-- Revert Phase 6.5: Remove CHECK and restore original index

ALTER TABLE payments DROP CONSTRAINT IF EXISTS chk_payments_status;

DROP INDEX IF EXISTS idx_policy_rules_profile;
CREATE INDEX idx_policy_rules_profile ON policy_rules(profile_id, priority);
