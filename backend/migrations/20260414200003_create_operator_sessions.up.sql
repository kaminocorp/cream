-- Phase 16-B: Refresh token sessions for operator JWT auth.
-- Token rotation: each refresh returns a new token and revokes the old one.
-- Reuse detection: if a revoked token is presented, all sessions for that
-- operator are revoked (stolen token scenario).

CREATE TABLE operator_sessions (
    id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    operator_id        UUID NOT NULL REFERENCES operators(id) ON DELETE CASCADE,
    refresh_token_hash TEXT NOT NULL UNIQUE,
    expires_at         TIMESTAMPTZ NOT NULL,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at         TIMESTAMPTZ
);

CREATE INDEX idx_sessions_operator ON operator_sessions(operator_id);
CREATE INDEX idx_sessions_active_refresh ON operator_sessions(refresh_token_hash)
    WHERE revoked_at IS NULL;

-- Enrich operator_events with operator identity (nullable for backward
-- compat with events created before operator auth existed).
ALTER TABLE operator_events ADD COLUMN IF NOT EXISTS operator_id UUID REFERENCES operators(id);
