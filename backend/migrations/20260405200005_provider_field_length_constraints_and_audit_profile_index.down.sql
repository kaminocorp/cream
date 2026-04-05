-- Reverse Phase 8.5: provider field length constraints + audit profile index

DROP INDEX IF EXISTS idx_audit_profile;

ALTER TABLE payments
    DROP CONSTRAINT IF EXISTS chk_payments_provider_tx_id_length;

ALTER TABLE payments
    DROP CONSTRAINT IF EXISTS chk_payments_provider_id_length;
