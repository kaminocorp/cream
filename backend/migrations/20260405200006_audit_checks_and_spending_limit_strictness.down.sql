-- Reverse v0.8.6 audit checks and spending limit strictness

-- Remove audit_log constraints
ALTER TABLE audit_log DROP CONSTRAINT IF EXISTS chk_audit_log_on_chain_tx_hash_length;
ALTER TABLE audit_log DROP CONSTRAINT IF EXISTS chk_audit_log_final_status;

-- Restore >= 0 spending limit constraints (original from 20260401200005)
ALTER TABLE agent_profiles DROP CONSTRAINT IF EXISTS chk_escalation_threshold_positive;
ALTER TABLE agent_profiles
    ADD CONSTRAINT chk_escalation_threshold_non_negative
        CHECK (escalation_threshold IS NULL OR escalation_threshold >= 0);

ALTER TABLE agent_profiles DROP CONSTRAINT IF EXISTS chk_max_monthly_spend_positive;
ALTER TABLE agent_profiles
    ADD CONSTRAINT chk_max_monthly_spend_non_negative
        CHECK (max_monthly_spend IS NULL OR max_monthly_spend >= 0);

ALTER TABLE agent_profiles DROP CONSTRAINT IF EXISTS chk_max_weekly_spend_positive;
ALTER TABLE agent_profiles
    ADD CONSTRAINT chk_max_weekly_spend_non_negative
        CHECK (max_weekly_spend IS NULL OR max_weekly_spend >= 0);

ALTER TABLE agent_profiles DROP CONSTRAINT IF EXISTS chk_max_daily_spend_positive;
ALTER TABLE agent_profiles
    ADD CONSTRAINT chk_max_daily_spend_non_negative
        CHECK (max_daily_spend IS NULL OR max_daily_spend >= 0);

ALTER TABLE agent_profiles DROP CONSTRAINT IF EXISTS chk_max_per_transaction_positive;
ALTER TABLE agent_profiles
    ADD CONSTRAINT chk_max_per_transaction_non_negative
        CHECK (max_per_transaction IS NULL OR max_per_transaction >= 0);
