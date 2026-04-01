-- Phase 6.10: Payment amount validation, enum constraints, and provider health bounds
--
-- Additive migration. Does not modify existing constraints or indexes.

-- 1. Payment amount must be strictly positive — zero/negative amounts are
--    nonsensical and would bypass the policy engine.
ALTER TABLE payments
    ADD CONSTRAINT chk_payments_amount_positive
        CHECK (amount > 0);

-- 2. Settled amount must be positive if present.
ALTER TABLE payments
    ADD CONSTRAINT chk_payments_amount_settled_positive
        CHECK (amount_settled IS NULL OR amount_settled > 0);

-- 3. Unique constraint on api_key_hash — two agents must not share the same
--    credential hash, or auth middleware would resolve to the wrong agent.
ALTER TABLE agents
    ADD CONSTRAINT uk_agents_api_key_hash UNIQUE (api_key_hash);

-- 4. Currency must be a known enum value (matches Rust Currency enum).
ALTER TABLE payments
    ADD CONSTRAINT chk_payments_currency CHECK (currency IN (
        'USD', 'EUR', 'GBP', 'SGD', 'JPY', 'CNY', 'HKD', 'AUD', 'CAD', 'INR',
        'KRW', 'TWD', 'THB', 'MYR', 'IDR', 'PHP', 'VND', 'BRL', 'MXN', 'CHF',
        'SEK', 'NOK', 'DKK', 'NZD', 'AED',
        'BTC', 'ETH', 'USDC', 'USDT', 'SOL', 'MATIC', 'AVAX', 'BASE_ETH'
    ));

-- 5. Preferred rail must be a known enum value (matches Rust RailPreference).
ALTER TABLE payments
    ADD CONSTRAINT chk_payments_preferred_rail CHECK (preferred_rail IN (
        'auto', 'card', 'ach', 'swift', 'local', 'stablecoin'
    ));

-- 6. Provider health error rate must be between 0.0 and 1.0.
ALTER TABLE provider_health
    ADD CONSTRAINT chk_provider_health_error_rate
        CHECK (error_rate_5m >= 0.0 AND error_rate_5m <= 1.0);

-- 7. Provider health latency must be non-negative.
ALTER TABLE provider_health
    ADD CONSTRAINT chk_provider_health_latency_non_negative
        CHECK (p50_latency_ms >= 0 AND p99_latency_ms >= 0);
