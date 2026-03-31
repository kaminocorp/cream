<div align="center">

<img src="assets/logo-readme.png" alt="Cream" width="400" />

# C.R.E.A.M.

### The open-source payment control plane for AI agents.

**Cream is a Rust-based payment control plane with a policy engine, multi-provider routing with circuit breakers, and an append-only audit ledger — exposed as both REST and MCP — designed for AI agents to spend money autonomously within operator-defined guardrails.**

[Architecture](#architecture) | [How It Works](#how-a-payment-flows) | [Tech Stack](#tech-stack) | [Getting Started](#getting-started) | [Roadmap](#roadmap) | [Contributing](#contributing) | [License](#license)

</div>

---

> *Cream is a Rust-based payment control plane with a policy engine, multi-provider routing with circuit breakers, and an append-only audit ledger — exposed as both REST and MCP — designed for AI agents to spend money autonomously within operator-defined guardrails.*

---

## What is Cream?

Cream sits between AI agents and payment providers (Stripe, Airwallex, Coinbase, etc.) and governs every transaction. Think of it like an API gateway, but specifically for money.

An agent never talks to Stripe directly. It talks to Cream, which decides **whether** the payment is allowed, **which** provider handles it, and **why** — then logs everything immutably.

**The core problem:** AI agents are gaining real economic agency, but the infrastructure for governing, auditing, and executing their payments doesn't exist yet. No single provider covers all rails, geographies, and agent frameworks. Cream fills this gap.

### Key Features

- **Unified API** — one interface for card payments, cross-border transfers, and stablecoin micropayments. Agents never know which provider executes the transaction.
- **Policy Engine** — declarative rules evaluated in priority order before any money moves. Amount caps, velocity limits, category restrictions, geographic controls, duplicate detection — 12 built-in rule types.
- **Structured Justification** — every payment request must include a machine-structured reason. No justification, no payment. This creates an agent-authored paper trail for every transaction.
- **Intelligent Routing** — automatically selects the optimal provider based on cost, latency, health, and corridor. Circuit breakers shift traffic away from degraded providers. Failover is automatic.
- **Human-in-the-Loop** — policy rules can require human approval for any transaction class. Operators approve or reject from the dashboard or via Slack notification.
- **Immutable Audit Ledger** — every step of every payment (request, justification, policy decision, routing choice, provider response) is written to an append-only log. The database physically blocks updates and deletes.
- **MCP-Native** — AI agents connect via the Model Context Protocol out of the box. Claude, GPT-4, LangChain, LangGraph — anything that speaks MCP works with Cream.

---

## Architecture

Cream's backend is a Rust workspace with 6 crates, layered as a strict dependency DAG:

```
                        +----------+
                        |  models  |    Pure types. Zero business logic.
                        +----+-----+
                             |
             +---------------+---------------+----------------+
             |               |               |                |
        +----v----+    +-----v-----+   +-----v-----+   +-----v-----+
        |  audit  |    |  policy   |   | providers |   |  router   |
        +---------+    +-----------+   +-----------+   +-----------+
             |               |               |                |
             +---------------+-------+-------+----------------+
                                     |
                                +----v----+
                                |   api   |    Axum HTTP server. Wires everything together.
                                +---------+

                          +------------------+
                          |  mcp-server (TS) |    TypeScript sidecar. Calls the REST API.
                          +------------------+
```

| Crate | Role |
|-------|------|
| **`models`** | Payment state machine, typed IDs, currency enum, policy rule types. Every other crate imports this. |
| **`policy`** | Rule evaluation engine. 12 built-in rule types. Stateless — receives context, returns a verdict. No database dependency. |
| **`providers`** | `PaymentProvider` trait + factory registry. Adding a new provider = implement the trait + register it. Zero changes to core logic. |
| **`router`** | Provider scoring, Redis-backed circuit breakers, cross-provider idempotency guards. |
| **`audit`** | Append-only write path and query interface for the immutable ledger. |
| **`api`** | Axum HTTP server. 12 REST endpoints, auth middleware, rate limiting, payment lifecycle orchestrator. |

The **MCP server** is a separate TypeScript project using the official `@modelcontextprotocol/sdk`. It translates MCP tool calls into REST API calls — zero business logic.

---

## How a Payment Flows

Every payment follows an 8-step deterministic pipeline:

```
[Agent] --> POST /v1/payments (with justification)
              |
   [1] Schema validation
   [2] Agent identity resolution + policy profile loaded
   [3] Justification evaluation (structural + category checks)
   [4] Policy engine (rules evaluated in priority order)
              |
       BLOCK / ESCALATE / APPROVE
              |
   [5] Routing engine (score providers, check circuit breakers)
   [6] Provider execution (dispatch + automatic failover)
   [7] Settlement confirmation
   [8] Audit write (immutable, append-only)
              |
[Agent] <-- { payment_id, status, provider }
```

**Latency target:** Sub-300ms end-to-end for approved autonomous transactions.

---

## Tech Stack

| Layer | Technology | Why |
|-------|-----------|-----|
| **Core backend** | Rust (Axum) | Performance, compile-time correctness, OSS credibility |
| **Database** | PostgreSQL (SQLx) | Compile-time checked queries, append-only audit enforcement |
| **Cache / Locks** | Redis | Rate limits, circuit breakers, cross-provider idempotency locks |
| **MCP server** | TypeScript | Official `@modelcontextprotocol/sdk` for maximum ecosystem compatibility |
| **Dashboard** | Next.js 15 + shadcn/ui | App Router, React Server Components, polished operator UI |
| **Task runner** | justfile | Modern, cross-platform command runner |
| **Infrastructure** | Docker Compose | One-command local dev environment |

### Design Principles

- **Single-tenant by design.** One deployment per operator. Clean, forkable, self-hostable.
- **Trait boundaries everywhere.** Auth, audit, providers, observability — all behind traits. Swap in your own implementations without touching core logic.
- **Money is `rust_decimal`, never floats.** `NUMERIC(19,4)` in the database. No floating-point anywhere in the payment path.
- **Cross-provider idempotency.** Redis distributed locks prevent double-payments even during provider failovers.

---

## Getting Started

> **Status:** Cream is in active development. The scaffold is being built. Check the [Roadmap](#roadmap) for current progress.

### Prerequisites

- Rust (stable)
- Node.js 20+
- Docker & Docker Compose
- [just](https://github.com/casey/just) (task runner)

### Local Development

```bash
# Clone the repo
git clone https://github.com/crimson-sun/cream.git
cd cream

# Start Postgres + Redis
just up

# Run database migrations
just migrate

# Build the backend
just build

# Run the API server
just run-api

# In another terminal, start the MCP server
just run-mcp

# In another terminal, start the frontend
just fe-dev
```

### Available Commands

```bash
just              # List all commands
just up           # Start Docker services (Postgres + Redis)
just down         # Stop Docker services
just build        # Build all Rust crates
just check        # Type-check without building
just test         # Run unit tests
just test-integration  # Run integration tests (requires Docker services)
just clippy       # Lint with Clippy
just fmt          # Format code
just run-api      # Start the API server
just run-mcp      # Start the MCP server
just fe-dev       # Start the frontend dev server
just fe-build     # Build the frontend
```

---

## API Overview

### REST Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/v1/payments` | Initiate a payment with justification |
| `GET` | `/v1/payments/{id}` | Get payment status and audit record |
| `POST` | `/v1/payments/{id}/approve` | Approve an escalated payment |
| `POST` | `/v1/payments/{id}/reject` | Reject an escalated payment |
| `POST` | `/v1/cards` | Issue a scoped virtual card |
| `PATCH` | `/v1/cards/{id}` | Update card controls |
| `DELETE` | `/v1/cards/{id}` | Revoke a card |
| `GET` | `/v1/audit` | Query the audit log |
| `GET` | `/v1/agents/{id}/policy` | Get agent's policy profile |
| `PUT` | `/v1/agents/{id}/policy` | Update agent's policy profile |
| `GET` | `/v1/providers/health` | Provider health status |
| `POST` | `/v1/webhooks` | Register a webhook endpoint |

### MCP Tools

Agents connecting via MCP have access to:

- `initiate_payment` — initiate a payment with structured justification
- `get_payment_status` — check payment status
- `create_virtual_card` — issue a scoped virtual card
- `get_my_policy` — retrieve current policy rules
- `get_audit_history` — query past transactions
- `check_provider_health` — check provider availability

---

## Roadmap

### Phase 1: Scaffold (Current)

- [x] Project documentation and architecture design
- [ ] Monorepo skeleton, Docker Compose, justfile, CI
- [ ] Core domain models (payment state machine, typed IDs, policy types)
- [ ] Database schema and migrations (9 tables, append-only audit)
- [ ] Policy engine (12 built-in rule types)
- [ ] Provider trait + mock provider
- [ ] Routing engine (scoring, circuit breakers, idempotency)
- [ ] API server (12 endpoints, auth, rate limiting, orchestrator)
- [ ] MCP server (TypeScript, 6 tools)
- [ ] Frontend skeleton (8 dashboard pages)
- [ ] CI pipeline (GitHub Actions)

### Phase 2: Provider Integrations

- [ ] Stripe Issuing + PaymentIntents
- [ ] Airwallex Issuing + Payouts
- [ ] Coinbase x402 + AgentKit

### Phase 3: Production-Ready Frontend

- [ ] Real-time transaction feed
- [ ] Escalation queue with approve/reject flows
- [ ] Policy editor (visual rule builder)
- [ ] Audit log browser with justification search
- [ ] Provider health dashboard
- [ ] Agent management UI

### Phase 4: Operational Maturity

- [ ] Webhook delivery + Slack escalation integration
- [ ] Operator onboarding flow
- [ ] Security hardening (credential rotation, mTLS)
- [ ] Observability stubs (bring your own platform)
- [ ] API documentation (OpenAPI spec)
- [ ] Load testing and E2E test suite

### Future: APAC Expansion

- [ ] PayNow (Singapore)
- [ ] GrabPay (Southeast Asia)
- [ ] UPI / Razorpay (India)

### Future: Protocol Layer

- [ ] Stripe ACP (Agentic Commerce Protocol)
- [ ] Google AP2 (Agent Payments Protocol)
- [ ] Visa Trusted Agent Protocol (TAP)

---

## Project Structure

```
cream/
├── backend/
│   ├── Cargo.toml              # Rust workspace
│   ├── crates/
│   │   ├── api/                # Axum HTTP server
│   │   ├── audit/              # Append-only audit ledger
│   │   ├── models/             # Domain types and state machines
│   │   ├── policy/             # Policy evaluation engine
│   │   ├── providers/          # Payment provider trait + implementations
│   │   └── router/             # Provider routing and circuit breakers
│   ├── mcp-server/             # TypeScript MCP server
│   └── migrations/             # PostgreSQL migrations
├── frontend/                   # Next.js 15 dashboard
├── assets/                     # Logo and brand assets
├── docs/
│   ├── tldr.md                 # 5-minute architecture overview
│   ├── background.md           # AI agent payments landscape research
│   ├── vision.md               # Full product specification
│   └── executing/
│       ├── implementation-plan.md  # Detailed scaffold blueprint
│       └── next-phases.md          # Post-scaffold roadmap
├── docker-compose.yml          # Postgres + Redis
├── justfile                    # Task runner commands
└── LICENSE                     # Apache 2.0
```

---

## Contributing

Cream is open source and we welcome contributions. The project is in early development — there's a lot of ground to cover.

**Good first areas:**
- Policy engine rule implementations
- Provider integrations (implement the `PaymentProvider` trait for your favourite PSP)
- Frontend dashboard components
- Documentation and examples

The architecture is designed with contributor experience in mind. Every major system (auth, audit, providers, observability) is behind a trait boundary — you can add new implementations without touching core logic.

See `docs/executing/implementation-plan.md` for the full technical blueprint.

---

## License

Cream is licensed under the [Apache License 2.0](LICENSE).

---

<div align="center">

**Built by [Crimson Sun](https://github.com/crimson-sun)**

*C.R.E.A.M. — Cash Rules Everything Around Me*

</div>
