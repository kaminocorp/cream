-- provider_health: queried on every routing decision, needs indexes
CREATE INDEX idx_provider_health_circuit_state ON provider_health(circuit_state);
CREATE INDEX idx_provider_health_last_checked ON provider_health(last_checked_at DESC);

-- webhook_endpoints: needs lookup indexes and status validation
CREATE INDEX idx_webhook_endpoints_url ON webhook_endpoints(url);
CREATE INDEX idx_webhook_endpoints_status ON webhook_endpoints(status);

-- Add CHECK constraint on webhook_endpoints.status (previously unconstrained TEXT)
ALTER TABLE webhook_endpoints
    ADD CONSTRAINT chk_webhook_status CHECK (status IN ('active', 'inactive', 'suspended'));
