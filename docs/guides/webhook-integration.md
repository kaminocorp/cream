# Webhook Integration Guide

Receive real-time payment event notifications from Cream.

## Registering a Webhook

```bash
curl -X POST https://api.cream.example.com/v1/webhooks \
  -H "Authorization: Bearer <operator_jwt>" \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://your-app.example.com/webhooks/cream",
    "events": ["payment.settled", "payment.failed", "escalation.created"]
  }'
```

Production deployments must use HTTPS URLs. HTTP URLs are rejected unless `ALLOW_INSECURE_WEBHOOKS=true` is set.

## Event Types

| Event | When |
|-------|------|
| `payment.settled` | Payment successfully completed |
| `payment.failed` | Payment failed at provider level |
| `payment.blocked` | Payment blocked by policy engine |
| `payment.timed_out` | Escalation timed out without approval |
| `escalation.created` | Payment entered PendingApproval |

## Webhook Payload

```json
{
  "event_type": "payment.settled",
  "payload": {
    "payment_id": "pay_019abc...",
    "status": "settled",
    "agent_id": "agt_019def...",
    "amount": "149.99",
    "currency": "SGD"
  },
  "timestamp": "2026-04-13T10:30:00Z"
}
```

## Signature Verification

Every webhook delivery includes an HMAC-SHA256 signature in the `X-Cream-Signature` header:

```
X-Cream-Signature: sha256=<hex(HMAC_SHA256(webhook_secret, raw_body))>
```

### Verification Example (Node.js)

```javascript
const crypto = require('crypto');

function verifySignature(body, signature, secret) {
  const expected = 'sha256=' + crypto
    .createHmac('sha256', secret)
    .update(body, 'utf8')
    .digest('hex');
  return crypto.timingSafeEqual(
    Buffer.from(signature),
    Buffer.from(expected)
  );
}
```

### Verification Example (Python)

```python
import hmac, hashlib

def verify_signature(body: bytes, signature: str, secret: str) -> bool:
    expected = "sha256=" + hmac.new(
        secret.encode(), body, hashlib.sha256
    ).hexdigest()
    return hmac.compare_digest(signature, expected)
```

Always use constant-time comparison to prevent timing attacks.

## Retry Policy

Failed deliveries are retried with exponential backoff:

| Attempt | Delay |
|---------|-------|
| 1 | Immediate |
| 2 | 30 seconds |
| 3 | 2 minutes |
| 4 | 10 minutes |
| 5 | 30 minutes |

After 5 failed attempts (configurable via `WEBHOOK_MAX_RETRIES`), the delivery is marked as exhausted. Each attempt has a 10-second timeout (configurable via `WEBHOOK_DELIVERY_TIMEOUT_SECS`).

Your endpoint must return a 2xx status code within the timeout to be considered successful.

## Delivery Log

View delivery history for a webhook endpoint:

```bash
curl https://api.cream.example.com/v1/webhooks/whk_019abc.../deliveries \
  -H "Authorization: Bearer <operator_jwt>"
```

Each delivery record includes: attempt number, HTTP status code, response time, and error message (if failed).

## Testing

Send a test event to verify your endpoint:

```bash
curl -X POST https://api.cream.example.com/v1/webhooks/whk_019abc.../test \
  -H "Authorization: Bearer <operator_jwt>"
```

This sends a synthetic event payload to your registered URL.

## Best Practices

1. **Respond quickly.** Return 200 before processing the event. Use a queue (SQS, Redis, etc.) for heavy processing.
2. **Handle duplicates.** Use the `payment_id` as an idempotency key on your side — the same event may be delivered more than once during retries.
3. **Verify signatures.** Always verify the HMAC signature before processing. Reject unverified payloads.
4. **Log delivery IDs.** Include the `X-Request-Id` header value in your logs for cross-system debugging.
