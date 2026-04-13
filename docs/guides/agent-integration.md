# Agent Integration Guide

How to register an AI agent, make payments, and handle responses.

## Registration Flow

Agents are created by operators via the management API:

```bash
curl -X POST https://api.cream.example.com/v1/agents \
  -H "Authorization: Bearer <operator_jwt>" \
  -H "Content-Type: application/json" \
  -d '{"name": "booking-agent", "profile_id": "prof_<uuid>"}'
```

Response:

```json
{
  "agent": {
    "id": "agt_019abc...",
    "name": "booking-agent",
    "status": "active",
    "profile_name": "Standard",
    ...
  },
  "api_key": "cream_a1b2c3d4e5f6..."
}
```

The `api_key` is returned **exactly once**. Store it in a secrets manager (Vault, AWS Secrets Manager). The backend stores only its SHA-256 hash.

## Making a Payment

```bash
curl -X POST https://api.cream.example.com/v1/payments \
  -H "Authorization: Bearer cream_a1b2c3d4e5f6..." \
  -H "Content-Type: application/json" \
  -d '{
    "amount": "149.99",
    "currency": "SGD",
    "recipient": {
      "type": "merchant",
      "identifier": "stripe_merchant_123",
      "name": "Cloud Provider Inc",
      "country": "US"
    },
    "preferred_rail": "auto",
    "justification": {
      "summary": "Purchasing 2x API credit packs for customer onboarding batch #4421",
      "task_id": "task_8372",
      "category": "software_subscription",
      "expected_value": "Completion of customer onboarding batch within 4h"
    },
    "metadata": {
      "workflow_id": "wf_abc123"
    },
    "idempotency_key": "idem_batch4421_payment1"
  }'
```

### Structured Justification

Every payment **must** include a justification. This is Cream's novel differentiator — no justification, no payment.

| Field | Required | Description |
|-------|----------|-------------|
| `summary` | Yes | Human-readable explanation (min 10 words) |
| `task_id` | No | Reference to the task or workflow |
| `category` | Yes | Controlled vocabulary: `software_subscription`, `cloud_infrastructure`, `api_access`, `travel`, `office_supplies`, `marketing`, `consulting`, `other` |
| `expected_value` | No | Expected outcome — enables proportionality rules |

### Payment Categories

`software_subscription`, `cloud_infrastructure`, `api_access`, `travel`, `office_supplies`, `marketing`, `consulting`, `other`

## Idempotency

Always include an `idempotency_key` to enable safe retries. If a network error occurs, resending the same request with the same key will return the original result rather than creating a duplicate payment.

Keys are scoped per agent and enforced across provider failovers.

## Payment Status Flow

```
Pending → Validating → Approved → Submitted → Settled (success)
                     → PendingApproval → Approved (after human review)
                                       → Rejected
                                       → TimedOut → Blocked
                     → Blocked (policy denied)
         Submitted → Failed (provider error)
```

Terminal states: `Settled`, `Failed`, `Blocked`, `Rejected`.

## Polling for Updates

```bash
curl https://api.cream.example.com/v1/payments/pay_019abc... \
  -H "Authorization: Bearer cream_a1b2c3d4e5f6..."
```

For real-time updates, configure a webhook endpoint (see [Webhook Integration Guide](webhook-integration.md)).

## Reading Your Policy

Agents can read their own policy profile:

```bash
curl https://api.cream.example.com/v1/agents/agt_019abc.../policy \
  -H "Authorization: Bearer cream_a1b2c3d4e5f6..."
```

This returns your spending limits, allowed categories, geographic restrictions, and active policy rules.

## Error Handling

| Status | Error Code | Action |
|--------|-----------|--------|
| 400 | `VALIDATION_ERROR` | Fix request format — check field types and required fields |
| 401 | `UNAUTHORIZED` | Check API key — may be rotated or agent suspended |
| 403 | `POLICY_BLOCKED` | Payment denied by policy — check `details.rule_ids` |
| 409 | `IDEMPOTENCY_CONFLICT` | Duplicate key — retrieve existing payment by ID |
| 422 | `JUSTIFICATION_INVALID` | Justification too short or category invalid |
| 429 | `RATE_LIMITED` | Back off — use `Retry-After` header value |
| 502 | `PROVIDER_ERROR` | Provider issue — safe to retry with same idempotency key |
| 503 | `ALL_PROVIDERS_UNAVAILABLE` | All providers down — retry after delay |

## Python Example

```python
import requests

API_URL = "https://api.cream.example.com"
API_KEY = "cream_a1b2c3d4e5f6..."

resp = requests.post(f"{API_URL}/v1/payments", headers={
    "Authorization": f"Bearer {API_KEY}",
    "Content-Type": "application/json",
}, json={
    "amount": "25.00",
    "currency": "USD",
    "recipient": {"type": "merchant", "identifier": "openai_api"},
    "preferred_rail": "auto",
    "justification": {
        "summary": "Purchasing GPT-4 API credits for document summarization pipeline",
        "category": "api_access",
    },
    "idempotency_key": "doc-summary-pipeline-20260413",
})

if resp.status_code == 200:
    payment = resp.json()
    print(f"Payment {payment['id']}: {payment['status']}")
elif resp.status_code == 403:
    print(f"Blocked by policy: {resp.json()['message']}")
else:
    print(f"Error {resp.status_code}: {resp.json()}")
```
