# API Reference

## Interactive Documentation

- **Swagger UI**: `GET /docs` — interactive API explorer with try-it-out
- **OpenAPI Spec**: `GET /v1/openapi.json` — machine-readable spec for client SDK generation

## Authentication

Cream supports two authentication modes:

### Agent API Key (Bearer)

Agents authenticate with their API key in the `Authorization` header:

```
Authorization: Bearer cream_<64 hex chars>
```

Agent keys are issued via `POST /v1/agents` (operator-only) and returned exactly once. The backend stores only the SHA-256 hash.

### Operator JWT (Bearer)

Operators authenticate with JWT access tokens:

```
Authorization: Bearer eyJhbGciOiJIUzI1NiI...
```

Obtain tokens via `POST /v1/auth/login`. Refresh via `POST /v1/auth/refresh`. Access tokens expire after 15 minutes (configurable via `JWT_ACCESS_TTL_SECS`).

## Endpoints

### Payments

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `POST` | `/v1/payments` | Agent | Initiate a payment with structured justification |
| `GET` | `/v1/payments/{id}` | Agent/Operator | Get payment status and audit record |
| `POST` | `/v1/payments/{id}/approve` | Operator | Approve an escalated payment |
| `POST` | `/v1/payments/{id}/reject` | Operator | Reject an escalated payment |

### Agents

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/v1/agents` | Operator | List all agents |
| `POST` | `/v1/agents` | Operator | Create a new agent (returns one-time API key) |
| `PATCH` | `/v1/agents/{id}` | Operator | Update agent name, status, or profile |
| `POST` | `/v1/agents/{id}/rotate-key` | Operator | Rotate agent API key |
| `GET` | `/v1/agents/{id}/policy` | Agent/Operator | Get agent's policy profile and rules |
| `PUT` | `/v1/agents/{id}/policy` | Operator | Update agent's policy profile |

### Cards

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `POST` | `/v1/cards` | Agent | Issue a scoped virtual card |
| `PATCH` | `/v1/cards/{id}` | Agent | Update card controls |
| `DELETE` | `/v1/cards/{id}` | Agent | Cancel/revoke a card |

### Audit

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/v1/audit` | Agent/Operator | Query audit log (supports CSV, NDJSON via Accept header) |
| `POST` | `/v1/audit/export` | Operator | Create async audit export to S3 |
| `GET` | `/v1/audit/exports/{id}` | Operator | Poll export job status |

### Webhooks

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/v1/webhooks` | Operator | List webhook endpoints |
| `POST` | `/v1/webhooks` | Operator | Register a webhook endpoint |
| `DELETE` | `/v1/webhooks/{id}` | Operator | Delete a webhook endpoint |
| `GET` | `/v1/webhooks/{id}/deliveries` | Operator | List delivery attempts |
| `POST` | `/v1/webhooks/{id}/test` | Operator | Send a test event |

### Alerts

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/v1/alerts` | Operator | List alert rules |
| `POST` | `/v1/alerts` | Operator | Create an alert rule |
| `PATCH` | `/v1/alerts/{id}` | Operator | Update an alert rule |
| `DELETE` | `/v1/alerts/{id}` | Operator | Disable an alert rule |
| `GET` | `/v1/alerts/history` | Operator | View recently fired alerts |

### Auth

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/v1/auth/status` | None | Check if any operator is registered |
| `POST` | `/v1/auth/register` | None | Register first operator |
| `POST` | `/v1/auth/login` | None | Login, receive JWT tokens |
| `POST` | `/v1/auth/refresh` | None | Rotate refresh token |
| `POST` | `/v1/auth/logout` | None | Revoke refresh token |

### Settings

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/v1/settings/provider-keys` | Operator | List stored provider API keys (masked) |
| `PUT` | `/v1/settings/provider-keys` | Operator | Store or update a provider API key |

### Policy Templates

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/v1/policy-templates` | Agent/Operator | List available templates |
| `GET` | `/v1/policy-templates/{id}` | Agent/Operator | Get a specific template |
| `POST` | `/v1/policy-templates/{template_id}/apply/{agent_id}` | Operator | Apply template to agent |

### Providers

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/v1/providers/health` | Agent/Operator | Provider health status |

### Integrations

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `POST` | `/v1/integrations/slack/callback` | Slack HMAC | Slack interaction callback |

### System

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/health` | None | Health check |
| `GET` | `/v1/openapi.json` | None | OpenAPI 3.1 spec |
| `GET` | `/docs` | None | Swagger UI |

## Pagination

List endpoints support `limit` and `offset` query parameters:

```
GET /v1/audit?limit=50&offset=100
```

Default limit varies by endpoint (typically 50). Maximum varies by endpoint.

## Rate Limiting

- Default: 100 requests per minute per agent API key
- Configurable via `RATE_LIMIT_REQUESTS` and `RATE_LIMIT_WINDOW_SECS`
- Rate-limited responses return `429 Too Many Requests` with a `Retry-After` header

## Error Response Format

All errors return a consistent JSON structure:

```json
{
  "error_code": "VALIDATION_ERROR",
  "message": "amount must be positive",
  "details": {}
}
```

### Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `VALIDATION_ERROR` | 400 | Malformed request or validation failure |
| `UNAUTHORIZED` | 401 | Missing or invalid credentials |
| `POLICY_BLOCKED` | 403 | Policy engine blocked the payment |
| `NOT_FOUND` | 404 | Resource not found |
| `IDEMPOTENCY_CONFLICT` | 409 | Duplicate idempotency key |
| `JUSTIFICATION_INVALID` | 422 | Justification failed structural checks |
| `RATE_LIMITED` | 429 | Rate limit exceeded |
| `INTERNAL_ERROR` | 500 | Unexpected server error |
| `PROVIDER_ERROR` | 502 | Upstream payment provider error |
| `ALL_PROVIDERS_UNAVAILABLE` | 503 | No providers available |

## Request Tracing

Every request is assigned an `X-Request-Id` header (UUID). Include this in support requests to trace through logs:

```bash
curl -v http://localhost:8080/v1/payments/pay_019...
# Response headers include: X-Request-Id: 550e8400-e29b-41d4-a716-446655440000
```

Search structured logs with: `request_id=550e8400-...`
