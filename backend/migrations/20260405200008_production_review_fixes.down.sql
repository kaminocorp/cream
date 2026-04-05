-- Revert v0.8.8 production review fixes

ALTER TABLE payments DROP CONSTRAINT IF EXISTS chk_payments_failure_reason_length;

DROP INDEX IF EXISTS idx_audit_category;
CREATE INDEX idx_audit_category ON audit_log USING gin ((justification->'category'));

CREATE INDEX IF NOT EXISTS idx_webhook_endpoints_url ON webhook_endpoints(url);

ALTER TABLE virtual_cards DROP CONSTRAINT IF EXISTS chk_virtual_cards_provider_card_id_length;
ALTER TABLE virtual_cards DROP CONSTRAINT IF EXISTS chk_virtual_cards_provider_id_length;

ALTER TABLE payments DROP CONSTRAINT IF EXISTS chk_payments_idempotency_key_length;

DROP INDEX IF EXISTS idx_payments_agent_created;

DROP INDEX IF EXISTS idx_payments_escalation_rule;
ALTER TABLE payments DROP COLUMN IF EXISTS escalation_rule_id;

ALTER TABLE agent_profiles
    ALTER COLUMN max_per_transaction DROP NOT NULL,
    ALTER COLUMN max_per_transaction DROP DEFAULT,
    ALTER COLUMN max_daily_spend DROP NOT NULL,
    ALTER COLUMN max_daily_spend DROP DEFAULT,
    ALTER COLUMN max_weekly_spend DROP NOT NULL,
    ALTER COLUMN max_weekly_spend DROP DEFAULT,
    ALTER COLUMN max_monthly_spend DROP NOT NULL,
    ALTER COLUMN max_monthly_spend DROP DEFAULT;

ALTER TABLE policy_rules DROP CONSTRAINT IF EXISTS policy_rules_action_check;
ALTER TABLE policy_rules
    ADD CONSTRAINT policy_rules_action_check
        CHECK (action IN ('approve', 'block', 'escalate'));
