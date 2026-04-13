-- Phase 16-A: Webhook delivery log — tracks every outbound webhook attempt
-- for auditability, retry scheduling, and operator visibility.

-- Add agent_id to webhook_endpoints so we can scope event delivery to the
-- agent whose payment triggered the event. Nullable for backward compatibility
-- with endpoints registered before this migration.
ALTER TABLE webhook_endpoints ADD COLUMN IF NOT EXISTS agent_id UUID REFERENCES agents(id);
CREATE INDEX IF NOT EXISTS idx_webhook_endpoints_agent_id ON webhook_endpoints(agent_id);

-- Delivery log: one row per delivery attempt per endpoint per event.
CREATE TABLE webhook_delivery_log (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    webhook_endpoint_id UUID NOT NULL REFERENCES webhook_endpoints(id),
    event_type          TEXT NOT NULL,
    payload             JSONB NOT NULL,
    status              TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'delivered', 'failed', 'exhausted')),
    http_status         SMALLINT,
    response_body       TEXT,
    attempt             SMALLINT NOT NULL DEFAULT 0
        CHECK (attempt >= 0),
    max_attempts        SMALLINT NOT NULL DEFAULT 5
        CHECK (max_attempts > 0),
    next_retry_at       TIMESTAMPTZ,
    signature           TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    delivered_at        TIMESTAMPTZ,
    last_attempted_at   TIMESTAMPTZ
);

-- Partial index for the retry worker: only rows that are eligible for retry.
CREATE INDEX idx_webhook_delivery_retry
    ON webhook_delivery_log(next_retry_at)
    WHERE status = 'failed' AND attempt < max_attempts;

-- Lookup deliveries by endpoint (for the delivery log UI).
CREATE INDEX idx_webhook_delivery_endpoint
    ON webhook_delivery_log(webhook_endpoint_id);

-- Most-recent-first listing.
CREATE INDEX idx_webhook_delivery_created
    ON webhook_delivery_log(created_at DESC);
