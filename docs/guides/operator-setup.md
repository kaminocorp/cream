# Operator Setup Guide

Get Cream running and create your first operator account.

## Prerequisites

- **PostgreSQL 15+** with `gen_random_uuid()` support
- **Redis 7+** for rate limiting, circuit breakers, and idempotency locks
- **Rust 1.85+** (if building from source) or Docker

## 1. Database Setup

Create a PostgreSQL database and run migrations:

```bash
createdb cream
export DATABASE_URL="postgres://localhost/cream"
cd backend && sqlx migrate run
```

This creates all tables: `agents`, `agent_profiles`, `payments`, `audit_entries`, `webhook_endpoints`, `operators`, `alert_rules`, and more.

## 2. Redis Setup

Ensure Redis is running:

```bash
redis-cli ping   # should return PONG
export REDIS_URL="redis://localhost:6379"
```

## 3. Minimum Environment Variables

```bash
export DATABASE_URL="postgres://user:pass@localhost:5432/cream"
export REDIS_URL="redis://localhost:6379"
export JWT_SECRET="$(openssl rand -hex 32)"  # at least 32 chars
export CORS_ALLOWED_ORIGINS="http://localhost:3000"
```

See [Self-Hosting Guide](self-hosting.md) for the complete environment variable reference.

## 4. Start the API Server

```bash
# From source
cd backend && cargo run --release

# Or via Docker
docker run -e DATABASE_URL -e REDIS_URL -e JWT_SECRET \
  -e CORS_ALLOWED_ORIGINS -p 8080:8080 cream-api
```

Verify: `curl http://localhost:8080/health` should return `ok`.

## 5. Register First Operator

The first operator registration is only available when no operators exist:

```bash
curl -X POST http://localhost:8080/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Admin",
    "email": "admin@example.com",
    "password": "your-secure-password-here"
  }'
```

Response includes access and refresh JWT tokens. Subsequent registrations are blocked.

## 6. Login and Access Dashboard

```bash
curl -X POST http://localhost:8080/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "admin@example.com", "password": "your-secure-password-here"}'
```

Use the returned `access_token` as `Authorization: Bearer <token>` for operator endpoints.

Start the dashboard frontend:

```bash
cd frontend && npm install && npm run dev
# Dashboard at http://localhost:3000
```

## 7. Create Your First Agent

```bash
curl -X POST http://localhost:8080/v1/agents \
  -H "Authorization: Bearer <operator_token>" \
  -H "Content-Type: application/json" \
  -d '{"name": "my-first-agent", "profile_id": "<profile_id>"}'
```

The response includes a one-time `api_key` (format: `cream_<64 hex chars>`). Store it securely — it cannot be retrieved again.

## 8. Verify with a Test Payment

```bash
curl -X POST http://localhost:8080/v1/payments \
  -H "Authorization: Bearer cream_<agent_api_key>" \
  -H "Content-Type: application/json" \
  -d '{
    "amount": "10.00",
    "currency": "USD",
    "recipient": {"type": "merchant", "identifier": "test_merchant"},
    "justification": {
      "summary": "Test payment to verify agent setup is working correctly",
      "category": "software_subscription"
    },
    "preferred_rail": "auto",
    "idempotency_key": "test-setup-001"
  }'
```

## Next Steps

- [Agent Integration Guide](agent-integration.md) — full payment flow for agent developers
- [Policy Authoring Guide](policy-authoring.md) — configure spending rules
- [Webhook Integration Guide](webhook-integration.md) — receive real-time payment events
- [Self-Hosting Guide](self-hosting.md) — production deployment with TLS, monitoring, and backups
