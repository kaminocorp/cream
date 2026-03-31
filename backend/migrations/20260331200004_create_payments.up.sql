-- Phase 3, Migration 4: payments
CREATE TABLE payments (
    id               UUID PRIMARY KEY,
    agent_id         UUID NOT NULL REFERENCES agents(id),
    idempotency_key  TEXT NOT NULL UNIQUE,
    amount           NUMERIC(19,4) NOT NULL,
    currency         TEXT NOT NULL,
    recipient        JSONB NOT NULL,
    preferred_rail   TEXT NOT NULL DEFAULT 'auto',
    justification    JSONB NOT NULL,
    metadata         JSONB,
    status           TEXT NOT NULL DEFAULT 'pending',
    provider_id      TEXT,
    provider_tx_id   TEXT,
    amount_settled   NUMERIC(19,4),
    settled_currency TEXT,
    failure_reason   TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- No CHECK on status: 10 PaymentStatus variants enforced at application layer
-- No explicit idempotency_key index: UNIQUE constraint already creates one

CREATE INDEX idx_payments_agent ON payments(agent_id);
CREATE INDEX idx_payments_status ON payments(status);
CREATE INDEX idx_payments_created ON payments(created_at);

CREATE TRIGGER payments_updated_at
    BEFORE UPDATE ON payments
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
