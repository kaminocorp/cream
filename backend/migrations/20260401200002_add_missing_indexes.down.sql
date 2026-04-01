ALTER TABLE webhook_endpoints DROP CONSTRAINT IF EXISTS chk_webhook_status;

DROP INDEX IF EXISTS idx_webhook_endpoints_status;
DROP INDEX IF EXISTS idx_webhook_endpoints_url;
DROP INDEX IF EXISTS idx_provider_health_last_checked;
DROP INDEX IF EXISTS idx_provider_health_circuit_state;
