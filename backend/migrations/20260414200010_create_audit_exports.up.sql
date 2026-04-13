-- Phase 17-E: Async audit export job tracking.
CREATE TABLE audit_exports (
    id           UUID PRIMARY KEY,
    status       TEXT NOT NULL DEFAULT 'pending'
                 CHECK (status IN ('pending', 'running', 'completed', 'failed')),
    format       TEXT NOT NULL CHECK (format IN ('csv', 'ndjson')),
    filters      JSONB NOT NULL DEFAULT '{}',
    destination  JSONB NOT NULL,
    rows_exported BIGINT,
    s3_key       TEXT,
    error_message TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX idx_audit_exports_status ON audit_exports(status);
