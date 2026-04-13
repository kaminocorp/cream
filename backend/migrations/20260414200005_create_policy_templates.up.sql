-- Policy template library: pre-built rule sets operators can apply with one click.
CREATE TABLE policy_templates (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL UNIQUE CHECK (char_length(name) BETWEEN 1 AND 200),
    description TEXT NOT NULL,
    category    TEXT NOT NULL
        CHECK (category IN ('starter', 'conservative', 'compliance', 'custom')),
    rules       JSONB NOT NULL,
    is_builtin  BOOLEAN NOT NULL DEFAULT false,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Seed 3 built-in templates.
-- Rules follow the same JSONB schema as policy_rules.condition + action + escalation.
INSERT INTO policy_templates (name, description, category, rules, is_builtin) VALUES
(
    'Starter',
    'Basic spending limits for low-risk agents. $1,000 per-transaction cap, $5,000 daily spend limit, 20 transactions per hour velocity cap.',
    'starter',
    '[
        {"rule_type": "amount_cap", "priority": 10, "condition": {"field_check": {"field": "amount", "op": "greater_than", "value": 1000}}, "action": "BLOCK"},
        {"rule_type": "spend_rate", "priority": 20, "condition": {"field_check": {"field": "daily_spend", "op": "greater_than", "value": 5000}}, "action": "BLOCK"},
        {"rule_type": "velocity_limit", "priority": 30, "condition": {"field_check": {"field": "hourly_count", "op": "greater_than", "value": 20}}, "action": "BLOCK"}
    ]'::jsonb,
    true
),
(
    'Conservative',
    'Strict limits with human review. Payments above $50 require approval. $500 daily cap. First-time merchants flagged. 10 transactions per hour.',
    'conservative',
    '[
        {"rule_type": "amount_cap", "priority": 10, "condition": {"field_check": {"field": "amount", "op": "greater_than", "value": 50}}, "action": "ESCALATE", "escalation": {"channel": "slack", "timeout_minutes": 30, "on_timeout": "BLOCK"}},
        {"rule_type": "amount_cap", "priority": 11, "condition": {"field_check": {"field": "amount", "op": "greater_than", "value": 500}}, "action": "BLOCK"},
        {"rule_type": "spend_rate", "priority": 20, "condition": {"field_check": {"field": "daily_spend", "op": "greater_than", "value": 500}}, "action": "BLOCK"},
        {"rule_type": "velocity_limit", "priority": 30, "condition": {"field_check": {"field": "hourly_count", "op": "greater_than", "value": 10}}, "action": "BLOCK"},
        {"rule_type": "first_time_merchant", "priority": 40, "condition": {"field_check": {"field": "is_first_time_merchant", "op": "equals", "value": true}}, "action": "ESCALATE", "escalation": {"channel": "slack", "timeout_minutes": 60, "on_timeout": "BLOCK"}}
    ]'::jsonb,
    true
),
(
    'Compliance',
    'SOX/PCI-aligned template with geographic restrictions and category controls. Blocks crypto rails, non-US/SG/EU recipients, and gambling/NFT categories. Strict $200 escalation threshold.',
    'compliance',
    '[
        {"rule_type": "amount_cap", "priority": 10, "condition": {"field_check": {"field": "amount", "op": "greater_than", "value": 200}}, "action": "ESCALATE", "escalation": {"channel": "email", "timeout_minutes": 120, "on_timeout": "BLOCK"}},
        {"rule_type": "amount_cap", "priority": 11, "condition": {"field_check": {"field": "amount", "op": "greater_than", "value": 5000}}, "action": "BLOCK"},
        {"rule_type": "category_restriction", "priority": 20, "condition": {"field_check": {"field": "justification.category", "op": "in", "value": ["gambling", "nft", "adult"]}}, "action": "BLOCK"},
        {"rule_type": "rail_restriction", "priority": 30, "condition": {"field_check": {"field": "preferred_rail", "op": "equals", "value": "stablecoin"}}, "action": "BLOCK"},
        {"rule_type": "geographic_restriction", "priority": 40, "condition": {"field_check": {"field": "recipient.country", "op": "not_in", "value": ["US", "SG", "DE", "GB", "FR", "NL", "JP", "AU"]}}, "action": "BLOCK"},
        {"rule_type": "velocity_limit", "priority": 50, "condition": {"field_check": {"field": "hourly_count", "op": "greater_than", "value": 5}}, "action": "ESCALATE", "escalation": {"channel": "email", "timeout_minutes": 60, "on_timeout": "BLOCK"}}
    ]'::jsonb,
    true
);
