# Policy Authoring Guide

Configure spending rules, category restrictions, and escalation thresholds for your agents.

## How Policies Work

Every payment request passes through the policy engine before reaching a provider. The engine evaluates rules in **priority order** (lower number = higher priority) and returns a verdict:

- **APPROVE** — payment proceeds to routing and provider execution
- **BLOCK** — payment is denied immediately (first block wins)
- **ESCALATE** — payment is paused for human approval

Rules are attached to **agent profiles**. Multiple agents can share a profile.

## Agent Profile Settings

Each profile has global spending controls set via `PUT /v1/agents/{id}/policy`:

| Setting | Type | Description |
|---------|------|-------------|
| `max_per_transaction` | Decimal | Max amount per single payment |
| `max_daily_spend` | Decimal | Cumulative daily limit |
| `max_weekly_spend` | Decimal | Cumulative weekly limit |
| `max_monthly_spend` | Decimal | Cumulative monthly limit |
| `allowed_categories` | Array | Whitelist of payment categories |
| `allowed_rails` | Array | Allowed payment rails (`auto`, `card`, `stablecoin`, etc.) |
| `geographic_restrictions` | Array | Allowed recipient country codes |
| `escalation_threshold` | Decimal | Auto-escalate payments above this amount |

Set to `null` to clear a limit (e.g., remove the daily cap).

## Rule Types

Cream includes 12 built-in rule evaluators. Rules are stored as JSON with a condition tree and an action.

### 1. `amount_cap`

Block or escalate payments above/below a specific amount.

```json
{
  "rule_type": "amount_cap",
  "priority": 10,
  "condition": {
    "field_check": {"field": "amount", "op": "greater_than", "value": "1000"}
  },
  "action": "ESCALATE",
  "enabled": true
}
```

### 2. `velocity_limit`

Limit the number of transactions per time window.

```json
{
  "rule_type": "velocity_limit",
  "priority": 20,
  "condition": {
    "field_check": {"field": "velocity_count", "op": "greater_than", "value": "10"}
  },
  "action": "BLOCK"
}
```

### 3. `spend_rate`

Limit cumulative spend per time window (daily/weekly/monthly).

```json
{
  "rule_type": "spend_rate",
  "priority": 15,
  "condition": {
    "field_check": {"field": "daily_spend", "op": "greater_than", "value": "5000"}
  },
  "action": "BLOCK"
}
```

### 4. `category_check`

Whitelist or blacklist payment categories.

```json
{
  "rule_type": "category_check",
  "priority": 5,
  "condition": {
    "field_check": {"field": "category", "op": "not_in", "value": ["software_subscription", "cloud_infrastructure"]}
  },
  "action": "BLOCK"
}
```

### 5. `merchant_check`

Allow or deny payments to specific merchants.

```json
{
  "rule_type": "merchant_check",
  "priority": 5,
  "condition": {
    "field_check": {"field": "recipient.identifier", "op": "not_in", "value": ["stripe_merchant_123", "aws_marketplace"]}
  },
  "action": "ESCALATE"
}
```

Merchant matching is case-insensitive.

### 6. `geographic`

Restrict payments by recipient country.

```json
{
  "rule_type": "geographic",
  "priority": 8,
  "condition": {
    "field_check": {"field": "recipient.country", "op": "not_in", "value": ["SG", "US", "GB"]}
  },
  "action": "BLOCK"
}
```

### 7. `rail_restriction`

Limit which payment rails an agent can use.

```json
{
  "rule_type": "rail_restriction",
  "priority": 10,
  "condition": {
    "field_check": {"field": "preferred_rail", "op": "equals", "value": "stablecoin"}
  },
  "action": "BLOCK"
}
```

### 8. `justification_quality`

Enforce minimum justification standards.

```json
{
  "rule_type": "justification_quality",
  "priority": 1,
  "condition": {
    "field_check": {"field": "justification.summary", "op": "regex", "value": "^.{0,20}$"}
  },
  "action": "BLOCK"
}
```

This blocks justifications shorter than 20 characters.

### 9. `time_window`

Restrict payments to specific hours of the day.

```json
{
  "rule_type": "time_window",
  "priority": 30,
  "condition": {
    "field_check": {"field": "hour", "op": "not_in", "value": [9,10,11,12,13,14,15,16,17]}
  },
  "action": "ESCALATE"
}
```

This escalates payments outside business hours (9am-5pm).

### 10. `first_time_merchant`

Flag payments to merchants the agent hasn't paid before.

```json
{
  "rule_type": "first_time_merchant",
  "priority": 25,
  "condition": {
    "field_check": {"field": "is_first_time_merchant", "op": "equals", "value": "true"}
  },
  "action": "ESCALATE"
}
```

### 11. `duplicate_detection`

Block payments that match a recent payment's amount + recipient within a time window.

```json
{
  "rule_type": "duplicate_detection",
  "priority": 3,
  "condition": {
    "field_check": {"field": "is_duplicate", "op": "equals", "value": "true"}
  },
  "action": "BLOCK"
}
```

### 12. `escalation_threshold`

Auto-escalate payments above a configurable amount. Simpler than `amount_cap` — reads the threshold from the agent profile's `escalation_threshold` field.

```json
{
  "rule_type": "escalation_threshold",
  "priority": 20,
  "condition": {
    "field_check": {"field": "amount", "op": "greater_than", "value": "500"}
  },
  "action": "ESCALATE"
}
```

## Condition Trees

Rules support composable boolean logic:

```json
{
  "condition": {
    "all": [
      {"field_check": {"field": "amount", "op": "greater_than", "value": "500"}},
      {"field_check": {"field": "category", "op": "not_in", "value": ["cloud_infrastructure"]}}
    ]
  },
  "action": "ESCALATE"
}
```

Operators: `all` (AND), `any` (OR), `not` (negation), `field_check` (leaf comparison).

Maximum nesting depth: 32 levels.

## Comparison Operators

| Operator | Description |
|----------|-------------|
| `equals` | Exact match (case-insensitive for strings) |
| `not_equals` | Not equal |
| `greater_than` | Numeric greater than |
| `less_than` | Numeric less than |
| `greater_than_or_equal` | Numeric >= |
| `less_than_or_equal` | Numeric <= |
| `in` | Value is in the provided list |
| `not_in` | Value is not in the provided list |
| `contains` | String contains substring (case-insensitive) |
| `regex` | Matches regular expression |

## Rule Evaluation Order

1. Rules are sorted by `priority` (ascending — lower number = higher priority)
2. Each rule's condition is evaluated
3. **First BLOCK wins** — evaluation stops and the payment is denied
4. **ESCALATE accumulates** — the payment enters PendingApproval
5. If no rules trigger BLOCK or ESCALATE, the payment is **APPROVED**

## Policy Templates

Cream ships with 3 built-in templates accessible via `GET /v1/policy-templates`:

| Template | Description |
|----------|-------------|
| **Starter** | Permissive defaults — $1000/transaction, $5000/day, all categories |
| **Conservative** | Tight controls — $200/transaction, $1000/day, software-only categories, geographic restrictions |
| **Compliance** | Enterprise-grade — escalation on all payments > $100, first-time merchant review, duplicate detection |

Apply a template to an agent:

```bash
curl -X POST https://api.cream.example.com/v1/policy-templates/{template_id}/apply/{agent_id} \
  -H "Authorization: Bearer <operator_jwt>"
```

## Best Practices

1. **Start with templates.** Apply the Starter template, then tighten rules as you learn agent behavior.
2. **Use ESCALATE, not BLOCK, initially.** Escalation lets you review edge cases and refine rules without blocking legitimate payments.
3. **Set daily caps.** Even with per-transaction limits, a daily cap prevents runaway spend loops.
4. **Duplicate detection early.** Set `duplicate_detection` at high priority (low number) to catch retries before they reach providers.
5. **Review the audit log.** Use `GET /v1/audit` to see which rules fire most often and adjust thresholds accordingly.
