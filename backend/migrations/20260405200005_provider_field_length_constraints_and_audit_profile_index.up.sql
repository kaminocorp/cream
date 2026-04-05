-- Phase 8.5: provider field length constraints + audit profile index
--
-- Closes the Rust↔DB validation gap for provider_id (MAX_PROVIDER_ID_LEN = 255)
-- and provider_tx_id (MAX_PROVIDER_TRANSACTION_ID_LEN = 500). The Rust layer
-- validates on all construction paths, but the DB allowed unbounded TEXT —
-- direct DB manipulation or future ORM changes could persist oversized values
-- that break deserialization. Same pattern as 20260405200004 (name length constraints).

-- 1. payments.provider_id — mirrors ProviderId::MAX_PROVIDER_ID_LEN = 255
ALTER TABLE payments
    ADD CONSTRAINT chk_payments_provider_id_length
        CHECK (provider_id IS NULL OR LENGTH(provider_id) <= 255);

-- 2. payments.provider_tx_id — mirrors MAX_PROVIDER_TRANSACTION_ID_LEN = 500
ALTER TABLE payments
    ADD CONSTRAINT chk_payments_provider_tx_id_length
        CHECK (provider_tx_id IS NULL OR LENGTH(provider_tx_id) <= 500);

-- 3. audit_log.agent_profile_id index — the audit ledger is append-only and
--    grows unbounded. Profile-scoped audit queries (e.g., "show all events for
--    this agent profile") require an index to avoid full table scans.
CREATE INDEX IF NOT EXISTS idx_audit_profile ON audit_log(agent_profile_id);
