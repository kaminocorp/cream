-- Reverse Phase 7.4: settled_currency constraint + provider_id index

DROP INDEX IF EXISTS idx_payments_provider_id;

ALTER TABLE payments
    DROP CONSTRAINT IF EXISTS chk_payments_settled_currency;
