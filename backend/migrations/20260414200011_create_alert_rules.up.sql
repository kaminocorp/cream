-- Phase 17-G: Configurable alerting rules engine.
CREATE TABLE alert_rules (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name             TEXT NOT NULL,
    description      TEXT NOT NULL DEFAULT '',
    metric           TEXT NOT NULL,
    condition        TEXT NOT NULL CHECK (condition IN ('gt', 'lt', 'gte', 'lte', 'eq')),
    threshold        NUMERIC(19,4) NOT NULL,
    window_seconds   INT NOT NULL DEFAULT 300,
    cooldown_seconds INT NOT NULL DEFAULT 3600,
    channels         JSONB NOT NULL DEFAULT '["dashboard"]',
    enabled          BOOLEAN NOT NULL DEFAULT true,
    last_fired_at    TIMESTAMPTZ,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TRIGGER alert_rules_updated_at
    BEFORE UPDATE ON alert_rules
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Seed 4 built-in alert presets.
INSERT INTO alert_rules (name, description, metric, condition, threshold, window_seconds, cooldown_seconds, channels, enabled)
VALUES
    ('Provider error spike', 'Fires when provider errors exceed 10 in 5 minutes', 'cream_provider_errors_total', 'gt', 10, 300, 3600, '["dashboard","slack"]', true),
    ('Payment failure rate', 'Fires when total failed payments exceed 15 in 5 minutes', 'cream_payments_total', 'gt', 15, 300, 3600, '["dashboard","slack"]', true),
    ('Escalation queue backup', 'Fires when pending escalations exceed 10', 'cream_escalation_pending_count', 'gt', 10, 300, 3600, '["dashboard","slack","email"]', true),
    ('Rate limit saturation', 'Fires when rate limit hits exceed 50 in 5 minutes', 'cream_rate_limit_hits_total', 'gt', 50, 300, 3600, '["dashboard"]', true);
