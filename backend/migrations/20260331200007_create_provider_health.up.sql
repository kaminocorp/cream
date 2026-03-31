-- Phase 3, Migration 7: provider_health
CREATE TABLE provider_health (
    provider_id     TEXT PRIMARY KEY,
    is_healthy      BOOLEAN NOT NULL DEFAULT true,
    error_rate_5m   DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    p50_latency_ms  BIGINT NOT NULL DEFAULT 0,
    p99_latency_ms  BIGINT NOT NULL DEFAULT 0,
    circuit_state   TEXT NOT NULL DEFAULT 'closed'
                    CHECK (circuit_state IN ('closed', 'open', 'half_open')),
    last_checked_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
