# Changelog

- [0.3.0](#030--2026-03-31) — Database schema and migrations
- [0.2.1](#021--2026-03-31) — Formatting fixes for CI compliance
- [0.2.0](#020--2026-03-31) — Core domain models crate
- [0.1.0](#010--2026-03-31) — Monorepo skeleton, tooling & infrastructure

---

## 0.3.0 — 2026-03-31

**Phase 3: Database Schema and Migrations**

Creates the PostgreSQL schema that persists the domain model. 9 tables, 17 indexes, CHECK constraints, and append-only audit enforcement.

### Added

- **9 reversible SQLx migrations** (18 files) creating: `agent_profiles`, `agents`, `policy_rules`, `payments`, `virtual_cards`, `audit_log`, `provider_health`, `webhook_endpoints`, `idempotency_keys`
- **Append-only audit enforcement** — `BEFORE UPDATE` and `BEFORE DELETE` triggers on `audit_log` that raise exceptions, preventing mutation at the database level
- **Reusable `set_updated_at()` trigger function** — auto-updates `updated_at` on 4 tables (`agent_profiles`, `agents`, `policy_rules`, `payments`)
- **CHECK constraints** on `agents.status`, `policy_rules.action`, `virtual_cards.card_type`, `virtual_cards.status`, `provider_health.circuit_state`
- **GIN index on audit justification category** and **computed B-tree index on audit request amount** for efficient audit queries
- **Phase 3 implementation plan** (`docs/executing/phase-3-implementation-plan.md`)

### Removed

- `backend/migrations/.gitkeep` — replaced by real migration files

### Verification

| Check | Result |
|-------|--------|
| `sqlx migrate run` (9 migrations) | ✅ All applied |
| Audit INSERT / UPDATE blocked / DELETE blocked | ✅ Pass |
| CHECK constraints reject invalid values | ✅ Pass |
| Full rollback + re-apply | ✅ Pass |
| `cargo fmt --all -- --check` | ✅ Pass |
| `cargo clippy --workspace -- -D warnings` | ✅ Pass |
| `cargo test --workspace` | ✅ 27/27 passing |

---

## 0.2.1 — 2026-03-31

**Post-review formatting fixes for CI compliance**

Caught during Phase 1 & 2 review — `cargo fmt --check` was failing, which would block CI.

### Fixed

- **`lib.rs` module ordering** — `mod` declarations reordered to alphabetical (`agent`, `audit`, `card`, …) to satisfy `rustfmt` default sort; prior order was dependency-logical but non-canonical
- **`lib.rs` prelude re-export ordering** — `ProviderId` moved before `ProviderHealth` in the `provider` re-export to match `rustfmt` alphabetical expectation
- **`error.rs` attribute formatting** — multi-line `#[error("justification too short: …")]` collapsed to single line per `rustfmt` preference

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | ✅ Pass |
| `cargo clippy --workspace -- -D warnings` | ✅ Pass |
| `cargo test --workspace` | ✅ 27/27 passing |

---

## 0.2.0 — 2026-03-31

**Phase 2: Core Domain Models Crate (`models`)**

Defines every shared domain type, enum, state machine, and typed ID that the rest of the system references. Pure types with zero business logic.

### Added

- **Typed ID system** (`ids.rs`) — `typed_id!` macro generating 7 newtype UUID wrappers (`PaymentId`, `AgentId`, `AgentProfileId`, `PolicyRuleId`, `AuditEntryId`, `VirtualCardId`, `WebhookEndpointId`) with prefixed Display/FromStr/Serde, plus string-based `IdempotencyKey`
- **Payment state machine** (`payment.rs`) — `PaymentStatus` enum with compile-time-enforced transitions, `Payment` entity with `transition()` method, `Currency` enum (25 fiat + 8 crypto), `RailPreference`, `PaymentRequest`/`PaymentResponse`
- **Structured justification** (`justification.rs`) — `Justification` struct + `PaymentCategory` controlled vocabulary enum
- **Recipient model** (`recipient.rs`) — `Recipient` with `RecipientType` (Merchant/Individual/Wallet/BankAccount)
- **Agent identity** (`agent.rs`) — `Agent`, `AgentProfile` (versioned spending authority), `AgentStatus`, `CountryCode`
- **Policy types** (`policy.rs`) — `PolicyRule`, recursive `PolicyCondition` tree (All/Any/Not/FieldCheck), `ComparisonOp` (10 operators), `EscalationConfig`/`EscalationChannel`
- **Provider types** (`provider.rs`) — `ProviderId`, `ProviderHealth`, `CircuitState`, `RoutingCandidate`/`RoutingDecision`
- **Virtual card types** (`card.rs`) — `VirtualCard`, `CardType`, `CardControls`, `CardStatus`
- **Audit types** (`audit.rs`) — `AuditEntry`, `PolicyEvaluationRecord`, `ProviderResponseRecord`, `HumanReviewRecord`
- **Domain errors** (`error.rs`) — `DomainError` enum with 8 variants via `thiserror`
- **Prelude module** (`lib.rs`) — re-exports all 40+ types for convenient downstream imports
- **27 unit tests** covering state machine transitions, serde roundtrips, ID parsing, and currency classification

---

## 0.1.0 — 2026-03-31

**Phase 1: Monorepo Skeleton, Tooling & Infrastructure**

Establishes the complete project structure, build tooling, local infrastructure, and CI pipeline so every subsequent phase has a working environment to build against.

### Added

- **Rust workspace** with 6 crates (`models`, `policy`, `providers`, `router`, `audit`, `api`) arranged as a strict compile-time-enforced dependency DAG
- **Workspace dependency centralisation** — all shared crate versions declared once in root `Cargo.toml`, referenced via `{ workspace = true }`
- **`cream-api` binary** with structured tracing (`tracing-subscriber`, `EnvFilter`, `RUST_LOG` support)
- **Docker Compose** — Postgres 16-alpine (port 5432) and Redis 7-alpine (port 6379) with health checks
- **Justfile** — 15 task runner commands across infrastructure, database, build, test, run, and frontend categories
- **GitHub Actions CI** — two-job pipeline (check: fmt + clippy + build; test: workspace tests) with `SQLX_OFFLINE=true` and `rust-cache`
- **MCP server scaffold** — TypeScript sidecar (`backend/mcp-server/`) with `@modelcontextprotocol/sdk` dependency
- **`.env.example`** documenting `DATABASE_URL`, `REDIS_URL`, `API_HOST`, `API_PORT`, `RUST_LOG`
- **Integration test harness stub** (`backend/tests/common/mod.rs`)
- **Migrations directory** (`backend/migrations/.gitkeep`) ready for SQLx migrations
