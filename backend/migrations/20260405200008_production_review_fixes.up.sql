-- v0.8.8: Production review fixes — schema corrections and defense-in-depth constraints.
-- All changes are additive (no reversals of prior hardenings).

-- C2: PolicyAction DB CHECK uses lowercase but Rust serde uses SCREAMING_SNAKE_CASE.
-- Every load_rules() call fails because serde cannot deserialize 'approve' as "APPROVE".
-- Data migration MUST run before the new CHECK to avoid constraint violation on non-empty tables.
ALTER TABLE policy_rules DROP CONSTRAINT IF EXISTS policy_rules_action_check;
UPDATE policy_rules SET action = UPPER(action) WHERE action != UPPER(action);
ALTER TABLE policy_rules
    ADD CONSTRAINT policy_rules_action_check
        CHECK (action IN ('APPROVE', 'BLOCK', 'ESCALATE'));

-- H1: Agent profile spending limits allow NULL in DB but Rust AgentProfile requires
-- non-optional Decimal (> 0). Any profile with NULL limits causes a 500 on auth.
-- Set sensible high defaults (effectively unlimited) for any existing NULL rows,
-- then add NOT NULL constraints.
UPDATE agent_profiles SET max_per_transaction = 999999999.9999
    WHERE max_per_transaction IS NULL;
UPDATE agent_profiles SET max_daily_spend = 999999999.9999
    WHERE max_daily_spend IS NULL;
UPDATE agent_profiles SET max_weekly_spend = 999999999.9999
    WHERE max_weekly_spend IS NULL;
UPDATE agent_profiles SET max_monthly_spend = 999999999.9999
    WHERE max_monthly_spend IS NULL;

ALTER TABLE agent_profiles
    ALTER COLUMN max_per_transaction SET NOT NULL,
    ALTER COLUMN max_per_transaction SET DEFAULT 999999999.9999,
    ALTER COLUMN max_daily_spend SET NOT NULL,
    ALTER COLUMN max_daily_spend SET DEFAULT 999999999.9999,
    ALTER COLUMN max_weekly_spend SET NOT NULL,
    ALTER COLUMN max_weekly_spend SET DEFAULT 999999999.9999,
    ALTER COLUMN max_monthly_spend SET NOT NULL,
    ALTER COLUMN max_monthly_spend SET DEFAULT 999999999.9999;

-- M1: Add escalation_rule_id to payments so the timeout monitor uses the correct
-- rule's timeout_minutes instead of guessing from all profile rules.
ALTER TABLE payments ADD COLUMN escalation_rule_id UUID REFERENCES policy_rules(id);
CREATE INDEX IF NOT EXISTS idx_payments_escalation_rule ON payments(escalation_rule_id)
    WHERE escalation_rule_id IS NOT NULL;

-- M4: Missing composite index on payments(agent_id, created_at) for hot-path
-- load_recent_payments query (called on every payment initiation).
-- Note: Not using CONCURRENTLY because sqlx migrations run in transactions.
CREATE INDEX IF NOT EXISTS idx_payments_agent_created
    ON payments (agent_id, created_at DESC);

-- M5: payments.idempotency_key missing length constraint (idempotency_keys.key has one).
ALTER TABLE payments
    ADD CONSTRAINT chk_payments_idempotency_key_length
        CHECK (LENGTH(idempotency_key) <= 255 AND LENGTH(TRIM(idempotency_key)) > 0);

-- M6: virtual_cards.provider_id and provider_card_id missing length constraints.
ALTER TABLE virtual_cards
    ADD CONSTRAINT chk_virtual_cards_provider_id_length
        CHECK (LENGTH(provider_id) <= 255);
ALTER TABLE virtual_cards
    ADD CONSTRAINT chk_virtual_cards_provider_card_id_length
        CHECK (LENGTH(provider_card_id) <= 500 AND LENGTH(TRIM(provider_card_id)) > 0);

-- M7: Replace redundant regular index with note (UNIQUE already provides the index).
DROP INDEX IF EXISTS idx_webhook_endpoints_url;

-- M7b: Fix audit category index — GIN on justification->'category' doesn't serve
-- text equality queries using ->>. Replace with btree on text extraction.
DROP INDEX IF EXISTS idx_audit_category;
CREATE INDEX idx_audit_category ON audit_log USING btree ((justification->>'category'));

-- M8: payments.failure_reason unbounded TEXT from external provider error messages.
ALTER TABLE payments
    ADD CONSTRAINT chk_payments_failure_reason_length
        CHECK (failure_reason IS NULL OR LENGTH(failure_reason) <= 2000);
