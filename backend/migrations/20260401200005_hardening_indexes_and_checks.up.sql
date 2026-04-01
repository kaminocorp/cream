-- Phase 6.7: Production hardening — indexes and CHECK constraints
--
-- Additive migration. Does not modify existing constraints or indexes.

-- 1. Missing index on audit_log.payment_id (used by get_by_payment queries)
CREATE INDEX IF NOT EXISTS idx_audit_payment
    ON audit_log(payment_id);

-- 2. Composite index for the most common audit query pattern:
--    "all audit entries for agent X in date range Y"
CREATE INDEX IF NOT EXISTS idx_audit_agent_timestamp
    ON audit_log(agent_id, "timestamp" DESC);

-- 3. CHECK constraints on agent_profiles amount fields.
--    Negative limits would silently invert policy enforcement.
ALTER TABLE agent_profiles
    ADD CONSTRAINT chk_max_per_transaction_non_negative
        CHECK (max_per_transaction IS NULL OR max_per_transaction >= 0);

ALTER TABLE agent_profiles
    ADD CONSTRAINT chk_max_daily_spend_non_negative
        CHECK (max_daily_spend IS NULL OR max_daily_spend >= 0);

ALTER TABLE agent_profiles
    ADD CONSTRAINT chk_max_weekly_spend_non_negative
        CHECK (max_weekly_spend IS NULL OR max_weekly_spend >= 0);

ALTER TABLE agent_profiles
    ADD CONSTRAINT chk_max_monthly_spend_non_negative
        CHECK (max_monthly_spend IS NULL OR max_monthly_spend >= 0);

ALTER TABLE agent_profiles
    ADD CONSTRAINT chk_escalation_threshold_non_negative
        CHECK (escalation_threshold IS NULL OR escalation_threshold >= 0);

-- 4. Version must be positive (zero or negative versions are meaningless).
ALTER TABLE agent_profiles
    ADD CONSTRAINT chk_version_positive
        CHECK (version > 0);
