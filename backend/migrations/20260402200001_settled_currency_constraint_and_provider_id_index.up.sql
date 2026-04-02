-- Phase 7.4: settled_currency enum constraint + provider_id index
--
-- Additive migration. Does not modify existing constraints or indexes.

-- 1. settled_currency must be a known enum value when present.
--    The currency column already has chk_payments_currency (v0.6.10) but
--    settled_currency was unconstrained — a buggy provider could permanently
--    store an invalid currency code in the payments table.
ALTER TABLE payments
    ADD CONSTRAINT chk_payments_settled_currency CHECK (settled_currency IS NULL OR settled_currency IN (
        'USD', 'EUR', 'GBP', 'SGD', 'JPY', 'CNY', 'HKD', 'AUD', 'CAD', 'INR',
        'KRW', 'TWD', 'THB', 'MYR', 'IDR', 'PHP', 'VND', 'BRL', 'MXN', 'CHF',
        'SEK', 'NOK', 'DKK', 'NZD', 'AED',
        'BTC', 'ETH', 'USDC', 'USDT', 'SOL', 'MATIC', 'AVAX', 'BASE_ETH'
    ));

-- 2. Index on provider_id for provider-level reconciliation and settlement
--    queries. The payments table has indexes on agent_id, status, and
--    created_at but was missing provider_id — forcing sequential scans for
--    per-provider reporting.
CREATE INDEX IF NOT EXISTS idx_payments_provider_id
    ON payments(provider_id);
