# Self-Hosting Guide

Complete reference for deploying Cream in production.

## Docker Compose

```yaml
version: "3.9"
services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: cream
      POSTGRES_USER: cream
      POSTGRES_PASSWORD: changeme
    volumes:
      - pgdata:/var/lib/postgresql/data
    ports:
      - "5432:5432"

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"

  cream-api:
    build: ./backend
    depends_on: [postgres, redis]
    environment:
      DATABASE_URL: "postgres://cream:changeme@postgres:5432/cream"
      REDIS_URL: "redis://redis:6379"
      JWT_SECRET: "${JWT_SECRET}"
      CORS_ALLOWED_ORIGINS: "https://dashboard.example.com"
      HOST: "0.0.0.0"
      PORT: "8080"
    ports:
      - "8080:8080"
      - "9090:9090"  # Prometheus metrics

  dashboard:
    build: ./frontend
    depends_on: [cream-api]
    environment:
      CREAM_API_URL: "http://cream-api:8080"
    ports:
      - "3000:3000"

volumes:
  pgdata:
```

## Kubernetes

Cream is stateless — deploy as a Deployment with horizontal pod autoscaling. PostgreSQL and Redis should be managed services (AWS RDS, ElastiCache) or operated via Helm charts (Bitnami).

Key considerations:
- Set `HOST=0.0.0.0` for container networking
- Use a `Secret` for `JWT_SECRET`, `PROVIDER_KEY_ENCRYPTION_SECRET`, database credentials
- Expose `/health` as the liveness/readiness probe
- Metrics port (9090) should be a separate `Service` for internal-only Prometheus scraping

## Reverse Proxy (TLS Termination)

### Nginx

```nginx
server {
    listen 443 ssl http2;
    server_name api.cream.example.com;

    ssl_certificate     /etc/letsencrypt/live/api.cream.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/api.cream.example.com/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### Caddy

```
api.cream.example.com {
    reverse_proxy localhost:8080
}
```

Caddy handles TLS automatically via Let's Encrypt.

## Environment Variable Reference

Every configuration option read by `AppConfig::from_env()`.

### Required

| Variable | Type | Description |
|----------|------|-------------|
| `DATABASE_URL` | String | PostgreSQL connection URL (e.g. `postgres://user:pass@host:5432/cream`) |
| `REDIS_URL` | String | Redis connection URL (e.g. `redis://localhost:6379`) |

### Server

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `HOST` | String | `0.0.0.0` | Bind address |
| `PORT` | u16 | `8080` | HTTP listen port |
| `CORS_ALLOWED_ORIGINS` | String | (empty) | Comma-separated allowed origins. Empty requires `ALLOW_PERMISSIVE_CORS=true` |
| `ALLOW_PERMISSIVE_CORS` | Bool | `false` | Allow all CORS origins (development only) |

### Authentication

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `JWT_SECRET` | String | (unset) | HMAC secret for JWT signing. At least 32 characters. Required for operator auth |
| `JWT_ACCESS_TTL_SECS` | i64 | `900` | Access token lifetime (15 minutes) |
| `JWT_REFRESH_TTL_SECS` | i64 | `604800` | Refresh token lifetime (7 days) |
| `OPERATOR_API_KEY` | String | (unset) | Legacy shared operator key. At least 32 characters. Deprecated in favor of JWT |

### Rate Limiting

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `RATE_LIMIT_REQUESTS` | u64 | `100` | Max requests per agent per window |
| `RATE_LIMIT_WINDOW_SECS` | u64 | `60` | Rate limit window duration in seconds |

### Webhooks

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `WEBHOOK_DELIVERY_TIMEOUT_SECS` | u64 | `10` | HTTP timeout for outbound webhook requests |
| `WEBHOOK_MAX_RETRIES` | u16 | `5` | Max delivery attempts per webhook event |
| `ALLOW_INSECURE_WEBHOOKS` | Bool | `false` | Allow `http://` webhook URLs (production should use HTTPS only) |

### Escalation Monitor

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `ESCALATION_CHECK_INTERVAL_SECS` | u64 | `30` | How often the escalation timeout monitor runs |

### Slack Integration

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `SLACK_BOT_TOKEN` | String | (unset) | Slack bot OAuth token (`xoxb-...`) |
| `SLACK_CHANNEL_ID` | String | (unset) | Slack channel ID for escalation messages |
| `SLACK_SIGNING_SECRET` | String | (unset) | Slack app signing secret for callback verification |

### Email Integration

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `SMTP_HOST` | String | (unset) | SMTP server hostname |
| `SMTP_PORT` | u16 | `587` | SMTP port (587 for STARTTLS) |
| `SMTP_USERNAME` | String | (unset) | SMTP username |
| `SMTP_PASSWORD` | String | (unset) | SMTP password |
| `EMAIL_FROM` | String | (unset) | Sender address for notification emails |
| `ESCALATION_EMAIL_TO` | String | (unset) | Recipient address for escalation emails |
| `RESEND_API_KEY` | String | (unset) | Resend API key (alternative to SMTP) |
| `DASHBOARD_BASE_URL` | String | (unset) | Dashboard URL for deep links in emails |

### Encryption

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `PROVIDER_KEY_ENCRYPTION_SECRET` | String | (unset) | Hex-encoded AES-256 key (64 hex chars = 32 bytes) for encrypting provider API keys at rest |

### Observability

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `LOG_FORMAT` | String | `json` | Log output: `json` (production) or `pretty` (development) |
| `LOG_LEVEL` | String | `info` | Global log level. Overridden by `RUST_LOG` if set |
| `LOG_BODIES` | Bool | `false` | Log request/response bodies at DEBUG level with PII redaction |
| `OTEL_ENABLED` | Bool | `false` | Enable OpenTelemetry distributed tracing |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | String | (unset) | OTLP gRPC endpoint (required when OTEL_ENABLED=true) |
| `OTEL_SERVICE_NAME` | String | `cream-api` | Service name reported in traces |
| `METRICS_ENABLED` | Bool | `true` | Enable Prometheus metrics endpoint |
| `METRICS_PORT` | u16 | `9090` | Port for `/metrics` HTTP listener |

### TLS

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `TLS_CERT_PATH` | String | (unset) | Path to TLS certificate (PEM). Both cert and key must be set together |
| `TLS_KEY_PATH` | String | (unset) | Path to TLS private key (PEM) |
| `HSTS_MAX_AGE` | u64 | `31536000` | HSTS max-age in seconds (1 year) |

### Security

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `CREDENTIAL_ROTATION_WARN_DAYS` | u64 | `90` | Warn when agent API key is older than this many days |

### Audit Export

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `AUDIT_EXPORT_S3_BUCKET` | String | (unset) | S3 bucket for async audit exports |
| `AUDIT_EXPORT_S3_REGION` | String | (unset) | AWS region for S3 bucket |
| `AUDIT_EXPORT_S3_PREFIX` | String | (unset) | Key prefix for S3 exports |

## Database Backups

```bash
# Backup
pg_dump -Fc cream > cream_$(date +%Y%m%d).dump

# Restore
pg_restore -d cream cream_20260413.dump
```

The `audit_entries` table is append-only (UPDATE/DELETE triggers raise errors). This is by design — the audit ledger is immutable.

## Scaling Notes

- **API server is stateless.** Run multiple instances behind a load balancer. All state lives in PostgreSQL and Redis.
- **Redis is the bottleneck** for rate limiting and idempotency checks. Redis Cluster or a managed Redis service handles this at scale.
- **Background workers** (escalation monitor, webhook delivery, credential age monitor, alert engine) run in every instance but are safe to run concurrently — they use conditional updates and idempotent operations.
