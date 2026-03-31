# Cream Backend Architecture — The 5-Minute Rundown

> "Cream is a Rust-based payment control plane with a policy engine, multi-provider routing with circuit breakers, and an append-only audit ledger — exposed as both REST and MCP — designed for AI agents to spend money autonomously within operator-defined guardrails."

## What it is

Cream is a **payment control plane** — it sits between AI agents and payment providers (Stripe, Airwallex, Coinbase, etc.) and governs every transaction. Think of it like an API gateway, but specifically for money. An agent never talks to Stripe directly; it talks to Cream, which decides *whether* the payment is allowed, *which* provider handles it, and *why* — then logs everything immutably.

## The stack

**Rust (Axum)** for the core backend — chosen for performance (sub-300ms payment lifecycle), compile-time correctness (the type system prevents entire categories of payment bugs), and open-source credibility. **PostgreSQL** for persistent storage. **Redis** for fast ephemeral state (rate limits, circuit breakers, idempotency locks). **TypeScript** for the MCP server (protocol adapter so AI agents can connect natively). **Next.js** for the operator dashboard.

## How a payment flows

Every payment follows an 8-step deterministic pipeline:

1. **Agent calls the API** — either REST (`POST /v1/payments`) or MCP tool call. Every request *must* include a structured justification explaining why the agent wants to spend money. No justification, no payment.
2. **Auth middleware** resolves the agent's identity and loads its policy profile (spend limits, allowed categories, geographic restrictions).
3. **Policy engine** evaluates the request against declarative rules in priority order. Rules are pure functions — no database calls, no network I/O — so evaluation completes in single-digit milliseconds. The engine returns one of three verdicts: **approve**, **block**, or **escalate** (require human approval).
4. **Routing engine** selects the optimal provider based on cost, latency, health, and corridor. Each provider has a circuit breaker — if Stripe's error rate spikes, traffic automatically shifts to Airwallex.
5. **Provider execution** dispatches to the selected provider via a standardised trait interface. If the primary provider fails, the orchestrator retries with the next-ranked candidate (failover).
6. **Audit write** — every single step (request, justification, policy decision, routing choice, provider response) is written to an **append-only ledger**. The database physically prevents updates or deletes on audit records via triggers.

## The crate structure

The backend is a Rust workspace with 6 crates, layered as a strict dependency DAG:

- **`models`** — pure types, zero business logic. Every other crate imports this. Contains the payment state machine, typed IDs (an `AgentId` can't be accidentally passed where a `PaymentId` is expected), currency enum, policy rule types.
- **`policy`** — the rule evaluation engine. 12 built-in rule types (amount caps, velocity limits, category restrictions, duplicate detection, etc.). Completely stateless — it receives a pre-loaded context and returns a verdict. No database dependency, trivially unit-testable.
- **`providers`** — defines the `PaymentProvider` trait (Rust's version of an interface). Every payment provider implements the same trait. A `ProviderRegistry` (factory pattern) holds all registered providers. Adding a new provider = implement the trait + register it. Zero changes to core logic.
- **`router`** — provider scoring (weighted multi-factor: cost, speed, health, corridor), circuit breakers (Redis-backed, shared across instances), and idempotency guards (Redis distributed locks preventing double-payments across provider failovers).
- **`audit`** — append-only write path and query interface for the immutable ledger. Trait-based so it can be mocked in tests or swapped for a different storage backend.
- **`api`** — the Axum HTTP server. All 12 REST endpoints, auth middleware, rate limiting, and the **payment orchestrator** which wires the above crates together into the 8-step pipeline. Also runs the escalation timeout monitor (a background Tokio task).

## Key architectural properties

- **Single-tenant by design.** One deployment per operator. No `tenant_id` columns, no row-level isolation complexity. Multi-tenant SaaS is a separate private fork with a FastAPI wrapper handling tenancy and billing.
- **Trait boundaries everywhere.** Auth, audit, providers, and observability are all behind traits. Contributors can swap in Vault for credentials, their own observability platform, or a new payment provider without touching core logic.
- **Money is `rust_decimal`, never floats.** All amounts use fixed-precision decimals. The database uses `NUMERIC(19,4)`. No floating-point anywhere in the payment path.
- **Idempotency is cross-provider.** A Redis lock on the idempotency key prevents double-payments even when a request fails over from one provider to another — the hardest edge case in multi-rail payment systems.
- **The MCP server is a thin TypeScript sidecar.** It translates MCP tool calls into REST API calls. Zero business logic. This means every agent framework in the ecosystem (Claude, GPT-4, LangChain) can connect out of the box using the official MCP SDK.
