-- Phase 3, Migration 9: idempotency_keys
CREATE TABLE idempotency_keys (
    key        TEXT PRIMARY KEY,
    payment_id UUID NOT NULL REFERENCES payments(id),
    response   JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

-- Supports periodic cleanup: DELETE FROM idempotency_keys WHERE expires_at < now()
CREATE INDEX idx_idemp_expires ON idempotency_keys(expires_at);
