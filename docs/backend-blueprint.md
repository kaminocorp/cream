# Backend Blueprint

Cream's Rust backend is a Cargo workspace with 6 crates, layered as an acyclic dependency pyramid — pure domain types at the base, HTTP server at the top.

```
                    cream-api          ← HTTP server, wires everything together
                   /    |    \
          cream-router  cream-audit    ← routing/safety guards, audit ledger
           /       \        |
   cream-providers  cream-policy       ← provider abstraction, rule engine
           \       /
            cream-models               ← shared domain types, zero logic
```

Rust 1.85+. Shared workspace dependencies managed in the root `backend/Cargo.toml`.

---

## Crates

### cream-models

> `backend/crates/models/` — Shared vocabulary. Every other crate depends on this.

Zero business logic. Defines all domain types:

| File | Contents |
|------|----------|
| `ids.rs` | Typed ID newtypes over UUIDv7 with human-readable prefixes (`payment_`, `agent_`, etc.), generated via macro |
| `payment.rs` | Payment state machine: `Pending → Validating → Approved → Submitted → Settled` with terminal states (Blocked, Failed, Rejected). Transition validation enforced at every state change |
| `policy.rs` | `PolicyRule`, recursive `PolicyCondition` tree (All/Any/Not/FieldCheck, max depth 32), `PolicyAction` (Approve/Block/Escalate), `EscalationConfig` |
| `agent.rs` | `Agent` (identity, status, timestamps), `AgentProfile` (spending limits, allowed categories/rails/countries/timezone) |
| `card.rs` | `VirtualCard` (SingleUse/MultiUse), `CardControls` (per-transaction/per-cycle limits, MCC codes) |
| `provider.rs` | `ProviderId`, `ProviderHealth` (error rate, latency, circuit state), `RoutingDecision`, `RoutingCandidate` |
| `audit.rs` | `AuditEntry` — immutable record capturing the full payment decision trace |
| `justification.rs` | `Justification` with `PaymentCategory` enum and bounded detail string |
| `recipient.rs` | `Recipient` with type (BankAccount/Wallet/Email/etc.), identifier, country |
| `error.rs` | `DomainError` variants for ID parse and validation failures |

---

### cream-policy

> `backend/crates/policy/` — Stateless, purely computational rule engine.

No database lookups during evaluation — all context is pre-loaded before the engine runs.

| File | Contents |
|------|----------|
| `engine.rs` | `PolicyEngine` — registers 12 evaluators, evaluates rules sorted by priority. **Semantics:** first Block wins immediately; Escalate accumulates; default is Approve |
| `context.rs` | `EvaluationContext` — immutable snapshot: payment request, agent, profile, recent payments (30 days), known merchants |
| `evaluator.rs` | `RuleEvaluator` trait — `fn evaluate(&self, rule, ctx) -> RuleResult` (Pass/Block/Escalate) |

**12 built-in rule evaluators** in `rules/`:

| Evaluator | Purpose |
|-----------|---------|
| `amount_cap` | Single-transaction amount limits |
| `velocity_limit` | Transaction count per time window |
| `spend_rate` | Cumulative amount per time window |
| `category_check` | Whitelist/blacklist payment categories |
| `merchant_check` | Whitelist/blacklist by merchant ID |
| `geographic` | Country/region restrictions |
| `rail_restriction` | Allowed payment rails |
| `justification_quality` | Minimum detail required in justification |
| `time_window` | Restrict to certain hours/days |
| `first_time_merchant` | Escalate new merchant relationships |
| `duplicate_detection` | Block identical payments in short window |
| `escalation_threshold` | Escalate if total escalations exceed threshold |

---

### cream-providers

> `backend/crates/providers/` — Provider abstraction and registry.

Adding a new provider (Stripe, Airwallex, Coinbase) = implement the trait + register. Zero changes to core logic.

| File | Contents |
|------|----------|
| `traits.rs` | `PaymentProvider` async trait — `initiate_payment`, `issue_virtual_card`, `update_card_controls`, `cancel_card`, `get_transaction_status`, `health_check` |
| `registry.rs` | `ProviderRegistry` — factory pattern, runtime registration via `register()` / `get()` / `all()` |
| `mock_provider.rs` | `MockProvider` — configurable test implementation, deterministic fake responses, no network calls |
| `types.rs` | `NormalizedPaymentRequest`, `ProviderPaymentResponse`, `TransactionStatus`, `CardConfig` |
| `error.rs` | `ProviderError` — each variant knows `is_retryable()` (timeout/rate-limit vs. auth-failure/compliance-block) |

---

### cream-audit

> `backend/crates/audit/` — Append-only audit ledger.

| File | Contents |
|------|----------|
| `writer.rs` | `AuditWriter` trait with `append()` only — no update, no delete, by design. `PgAuditWriter` for PostgreSQL (DB triggers also enforce immutability) |
| `reader.rs` | `AuditReader` trait with `query()`. `PgAuditReader` for PostgreSQL |
| `error.rs` | `AuditError` for serialization/database failures |

---

### cream-router

> `backend/crates/router/` — Provider scoring, circuit breakers, idempotency guards.

| File | Contents |
|------|----------|
| `selector.rs` | `RouteSelector` — loads health data, delegates to scorer, returns ranked `RoutingDecision`. Does **not** execute payments |
| `scorer.rs` | `ProviderScorer` — Phase 1: filter (circuit-broken, unsupported currency/rail). Phase 2: composite score (cost, latency, health). Ranks descending |
| `circuit_breaker.rs` | `CircuitBreaker` — Closed → Open → HalfOpen state machine. Backed by `CircuitBreakerStore` trait (Redis in prod, in-memory in scaffold) |
| `idempotency.rs` | `IdempotencyGuard` — distributed lock via `IdempotencyStore` trait. `acquire()` returns `Acquired` or `Existing(payment_id)`. 24h TTL |
| `config.rs` | `CircuitBreakerConfig`, `IdempotencyConfig`, `ScoringWeights` with validation |

---

### cream-api

> `backend/crates/api/` — Axum HTTP server. Wires all crates together.

#### Wiring and bootstrap

| File | Contents |
|------|----------|
| `main.rs` | Bootstrap: init tracing, connect Postgres/Redis, create all subsystems, register providers, spawn background workers, serve on configured port |
| `lib.rs` | `build_router()` — all route definitions, middleware layers (CORS, rate limit, request ID) |
| `state.rs` | `AppState` — Arc-wrapped bag of dependencies (db, redis, policy engine, route selector, providers, audit, idempotency, circuit breaker, payment repo, config) |
| `config.rs` | `AppConfig::from_env()` — all configuration via environment variables |
| `error.rs` | `ApiError` — maps to HTTP status codes (400, 401, 403, 404, 409, 422, 429, 500, 502, 503) |

#### The orchestrator

| File | Contents |
|------|----------|
| `orchestrator.rs` | `PaymentOrchestrator` — the 8-step payment lifecycle. The single integration point that sequences all crates into a payment flow |

Steps:
1. Schema validation and agent identity (Axum extractors)
2. Justification structural validation
3. Idempotency check (acquire or return existing)
4. Policy engine evaluation → Approve / Block / Escalate
5. Routing — score and rank providers (if Approved)
6. Provider execution with failover (circuit breaker guards each attempt)
7. Human review path (if Escalated — approval/rejection/timeout)
8. Audit write + webhook fire

#### Authentication and extractors

| File | Contents |
|------|----------|
| `extractors/auth.rs` | `AuthenticatedAgent` (bearer token → SHA-256 hash lookup), `AuthenticatedOperator` (shared secret), `AuthenticatedPrincipal` (enum for dual-path handlers). Constant-time comparison |
| `extractors/json.rs` | `ValidatedJson` — serde deserialization + structural validation (justification rules, amount bounds, recipient checks) |

#### Middleware

| File | Contents |
|------|----------|
| `middleware/request_id.rs` | Propagates/generates `X-Request-ID` header |
| `middleware/rate_limit.rs` | Redis-backed per-agent rate limiting. Default 100 req / 60s window. Returns 429 with `Retry-After` |

#### Routes

| Endpoint | Handler | Auth |
|----------|---------|------|
| `POST /v1/payments` | `routes/payments.rs::initiate` | Agent |
| `GET /v1/payments/{id}` | `routes/payments.rs::get_status` | Agent |
| `POST /v1/payments/{id}/approve` | `routes/payments.rs::approve` | Operator |
| `POST /v1/payments/{id}/reject` | `routes/payments.rs::reject` | Operator |
| `POST /v1/cards` | `routes/cards.rs::create` | Agent |
| `PATCH /v1/cards/{id}` | `routes/cards.rs::update` | Agent |
| `DELETE /v1/cards/{id}` | `routes/cards.rs::cancel` | Agent |
| `GET /v1/audit` | `routes/audit.rs::query` | Agent |
| `GET /v1/agents` | `routes/agents.rs::list_agents` | Operator |
| `POST /v1/agents` | `routes/agents.rs::create_agent` | Operator |
| `PATCH /v1/agents/{id}` | `routes/agents.rs::update_agent` | Operator |
| `POST /v1/agents/{id}/rotate-key` | `routes/agents.rs::rotate_agent_key` | Operator |
| `GET /v1/agents/{id}/policy` | `routes/agents.rs::get_policy` | Agent or Operator |
| `PUT /v1/agents/{id}/policy` | `routes/agents.rs::update_policy` | Operator |
| `GET /v1/providers/health` | `routes/providers.rs::health` | Agent |
| `GET /v1/webhooks` | `routes/webhooks.rs::list_webhooks` | Operator |
| `POST /v1/webhooks` | `routes/webhooks.rs::register` | Operator |
| `DELETE /v1/webhooks/{id}` | `routes/webhooks.rs::delete_webhook` | Operator |
| `GET /v1/webhooks/{id}/deliveries` | `routes/webhooks.rs::list_deliveries` | Operator |
| `POST /v1/webhooks/{id}/test` | `routes/webhooks.rs::test_webhook` | Operator |
| `GET /health` | Healthcheck | None |

#### Background workers

| Worker | Purpose |
|--------|---------|
| `orchestrator.rs::escalation_timeout_monitor` | Polls for `PendingApproval` payments past their timeout, transitions to `Blocked` |
| `webhook_worker.rs::webhook_delivery_worker` | Pops events from Redis queue (`cream:webhook:queue`), delivers with HMAC-SHA256 signatures |
| `webhook_worker.rs::webhook_retry_worker` | Retries failed deliveries with exponential backoff (5s, 30s, 2m, 15m, 1h) |

---

## Payment Flow

```
POST /v1/payments
  │
  ├─ Axum extracts & authenticates agent
  │
  └─ Orchestrator::process()
       │
       ├─ Idempotency lock (acquire or return existing)
       ├─ Persist payment (Pending)
       ├─ Validate justification
       ├─ Policy engine evaluates rules
       │
       ├─ Block?   → Blocked state → audit + webhook → 403
       ├─ Escalate? → PendingApproval → audit + webhook → 202
       │               └─ await human POST .../approve or .../reject
       │               └─ or timeout monitor transitions → Blocked
       └─ Approve?  → RouteSelector scores providers
                     → Try providers in rank order (circuit breaker gates each)
                     → Settlement persistence
                     → audit + webhook → 200
```

---

## Infrastructure

### PostgreSQL (source of truth)

~60 migrations in `backend/migrations/`. Key tables:

- `agents` / `agent_profiles` — identity, API key hashes, spending limits, allowed categories
- `policy_rules` — declarative rules per profile, ordered by priority
- `payments` — full lifecycle state including settlement details
- `virtual_cards` — card identity, status, controls
- `audit_log` — append-only (DB triggers prevent UPDATE/DELETE)
- `webhook_endpoints` / `webhook_delivery_log` — registered URLs, delivery attempts, retry schedule

### Redis

| Key pattern | Purpose |
|-------------|---------|
| `cream:idempotency:<key>` | Payment ID lock, 24h TTL |
| `cream:circuit:<provider_id>` | Circuit breaker state |
| `cream:webhook:queue` | Webhook event FIFO queue |
| `cream:ratelimit:<agent_id>:<window>` | Request count per rate-limit window |

### Docker

Multi-stage build (`backend/Dockerfile`): Rust 1.85-slim builder → Debian slim runtime. Non-root `cream` user. Port 8080.

### Environment variables

| Variable | Default | Required |
|----------|---------|----------|
| `DATABASE_URL` | — | Yes |
| `REDIS_URL` | — | Yes |
| `HOST` | `0.0.0.0` | No |
| `PORT` | `8080` | No |
| `OPERATOR_API_KEY` | — | No (min 32 chars if set) |
| `CORS_ALLOWED_ORIGINS` | — | Yes (unless `ALLOW_PERMISSIVE_CORS=true`) |
| `RATE_LIMIT_REQUESTS` | `100` | No |
| `RATE_LIMIT_WINDOW_SECS` | `60` | No |
| `ESCALATION_CHECK_INTERVAL_SECS` | `30` | No |
| `WEBHOOK_DELIVERY_TIMEOUT_SECS` | `10` | No |
| `WEBHOOK_MAX_RETRIES` | `5` | No |

---

## Architectural Patterns

**Trait-based abstractions** — `PaymentProvider`, `AuditWriter`/`AuditReader`, `PaymentRepository`, `CircuitBreakerStore`, `IdempotencyStore` all have mock implementations for testing without external services.

**State machines with validated transitions** — `PaymentStatus` and `CircuitState` enforce legal transitions at every state change.

**Policy engine as pure function** — context is pre-loaded, evaluation involves zero I/O. Rules are sorted by priority and evaluated in order with first-block-wins semantics.

**Provider abstraction via registry** — new providers implement a trait and register at runtime. Core logic never changes.

**Append-only audit** — no update or delete path exists in Rust code. Database triggers are the second line of defense.

**Type-safe IDs** — UUIDv7 newtypes with prefix parsing and validation prevent ID mixups at compile time.

**Layered auth** — Axum `FromRequestParts` extractors resolve identity before the handler runs. Agent auth via hashed API keys, operator auth via shared secret.
