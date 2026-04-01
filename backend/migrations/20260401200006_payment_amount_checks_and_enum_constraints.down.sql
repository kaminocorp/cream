-- Reverse Phase 6.10 payment amount checks and enum constraints

ALTER TABLE provider_health DROP CONSTRAINT IF EXISTS chk_provider_health_latency_non_negative;
ALTER TABLE provider_health DROP CONSTRAINT IF EXISTS chk_provider_health_error_rate;
ALTER TABLE payments DROP CONSTRAINT IF EXISTS chk_payments_preferred_rail;
ALTER TABLE payments DROP CONSTRAINT IF EXISTS chk_payments_currency;
ALTER TABLE agents DROP CONSTRAINT IF EXISTS uk_agents_api_key_hash;
ALTER TABLE payments DROP CONSTRAINT IF EXISTS chk_payments_amount_settled_positive;
ALTER TABLE payments DROP CONSTRAINT IF EXISTS chk_payments_amount_positive;
