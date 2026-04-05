-- v0.8.6: Audit ledger CHECK constraints and spending limit strictness
--
-- Three changes, all additive:
--
-- 1. Tighten agent_profiles spending limit CHECKs from >= 0 to > 0.
--    Rust AgentProfile custom Deserialize requires > 0; the DB allowed zero,
--    which would pass the CHECK but fail deserialization on read — permanently
--    locking the agent out of authentication.
--
-- 2. Add CHECK on audit_log.final_status to constrain to valid PaymentStatus
--    enum values. The audit ledger is append-only (triggers prevent UPDATE/DELETE),
--    so invalid values would be permanent.
--
-- 3. Add CHECK on audit_log.on_chain_tx_hash length to match Rust's
--    MAX_ON_CHAIN_TX_HASH_LEN = 256.

-- 1. Replace >= 0 spending limit checks with > 0
ALTER TABLE agent_profiles DROP CONSTRAINT IF EXISTS chk_max_per_transaction_non_negative;
ALTER TABLE agent_profiles
    ADD CONSTRAINT chk_max_per_transaction_positive
        CHECK (max_per_transaction IS NULL OR max_per_transaction > 0);

ALTER TABLE agent_profiles DROP CONSTRAINT IF EXISTS chk_max_daily_spend_non_negative;
ALTER TABLE agent_profiles
    ADD CONSTRAINT chk_max_daily_spend_positive
        CHECK (max_daily_spend IS NULL OR max_daily_spend > 0);

ALTER TABLE agent_profiles DROP CONSTRAINT IF EXISTS chk_max_weekly_spend_non_negative;
ALTER TABLE agent_profiles
    ADD CONSTRAINT chk_max_weekly_spend_positive
        CHECK (max_weekly_spend IS NULL OR max_weekly_spend > 0);

ALTER TABLE agent_profiles DROP CONSTRAINT IF EXISTS chk_max_monthly_spend_non_negative;
ALTER TABLE agent_profiles
    ADD CONSTRAINT chk_max_monthly_spend_positive
        CHECK (max_monthly_spend IS NULL OR max_monthly_spend > 0);

ALTER TABLE agent_profiles DROP CONSTRAINT IF EXISTS chk_escalation_threshold_non_negative;
ALTER TABLE agent_profiles
    ADD CONSTRAINT chk_escalation_threshold_positive
        CHECK (escalation_threshold IS NULL OR escalation_threshold > 0);

-- 2. Constrain audit_log.final_status to valid PaymentStatus enum values
ALTER TABLE audit_log
    ADD CONSTRAINT chk_audit_log_final_status CHECK (final_status IN (
        'pending', 'validating', 'pending_approval', 'approved', 'submitted',
        'settled', 'failed', 'blocked', 'rejected', 'timed_out'
    ));

-- 3. Constrain audit_log.on_chain_tx_hash length (matches MAX_ON_CHAIN_TX_HASH_LEN = 256)
ALTER TABLE audit_log
    ADD CONSTRAINT chk_audit_log_on_chain_tx_hash_length
        CHECK (on_chain_tx_hash IS NULL OR LENGTH(on_chain_tx_hash) <= 256);
