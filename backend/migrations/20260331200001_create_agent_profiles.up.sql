-- Phase 3, Migration 1: agent_profiles
-- Reusable trigger function for auto-updating updated_at columns.
-- Defined once here; applied to every table that has an updated_at column.
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TABLE agent_profiles (
    id                      UUID PRIMARY KEY,
    name                    TEXT NOT NULL,
    version                 INT NOT NULL DEFAULT 1,
    max_per_transaction     NUMERIC(19,4),
    max_daily_spend         NUMERIC(19,4),
    max_weekly_spend        NUMERIC(19,4),
    max_monthly_spend       NUMERIC(19,4),
    allowed_categories      JSONB NOT NULL DEFAULT '[]',
    allowed_rails           JSONB NOT NULL DEFAULT '[]',
    geographic_restrictions JSONB NOT NULL DEFAULT '[]',
    escalation_threshold    NUMERIC(19,4),
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TRIGGER agent_profiles_updated_at
    BEFORE UPDATE ON agent_profiles
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
