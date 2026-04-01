-- Reverse Phase 6.7 hardening indexes and CHECK constraints

ALTER TABLE agent_profiles DROP CONSTRAINT IF EXISTS chk_version_positive;
ALTER TABLE agent_profiles DROP CONSTRAINT IF EXISTS chk_escalation_threshold_non_negative;
ALTER TABLE agent_profiles DROP CONSTRAINT IF EXISTS chk_max_monthly_spend_non_negative;
ALTER TABLE agent_profiles DROP CONSTRAINT IF EXISTS chk_max_weekly_spend_non_negative;
ALTER TABLE agent_profiles DROP CONSTRAINT IF EXISTS chk_max_daily_spend_non_negative;
ALTER TABLE agent_profiles DROP CONSTRAINT IF EXISTS chk_max_per_transaction_non_negative;

DROP INDEX IF EXISTS idx_audit_agent_timestamp;
DROP INDEX IF EXISTS idx_audit_payment;
