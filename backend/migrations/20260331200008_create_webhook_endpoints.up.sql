-- Phase 3, Migration 8: webhook_endpoints
CREATE TABLE webhook_endpoints (
    id          UUID PRIMARY KEY,
    url         TEXT NOT NULL,
    secret_hash TEXT NOT NULL,
    events      JSONB NOT NULL DEFAULT '["*"]',
    status      TEXT NOT NULL DEFAULT 'active',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
