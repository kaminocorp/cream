# Changelog

- [0.7.3](#073--2026-04-02) — Cross-crate audit: ProviderResponseRecord positive settlement validation, RoutingCandidate score/fee guards, ProviderHealth latency invariant, IdempotencyConfig validation, selector bounds hardening
- [0.7.2](#072--2026-04-02) — Production readiness review: ProviderResponseRecord whitespace guards, router config validation enforcement, MerchantCheckEvaluator doc correction
- [0.7.1](#071--2026-04-02) — Cross-crate consistency review: empty-string guards on audit-bound fields, positive-value validation on spending limits, regex cache comment correction
- [0.7.0](#070--2026-04-01) — Routing engine crate: provider scorer, circuit breakers, idempotency guard, route selector
- [0.6.16](#0616--2026-04-01) — Production readiness review: ProviderId empty-string validation
- [0.6.15](#0615--2026-04-01) — Production readiness review: HumanReviewRecord rejects Escalate decision, Recipient empty-identifier guard, Justification empty/whitespace-only summary guard
- [0.6.14](#0614--2026-04-01) — Production sweep: ProviderResponseRecord string bounds, set_provider transaction_id limit, Equals/NotEquals/Contains case-insensitive matching, ProviderHealth error_rate validation
- [0.6.13](#0613--2026-04-01) — Cross-crate audit: AuditEntry payment_id field, TimedOut terminal status, In/NotIn case-insensitive matching, webhook_endpoints updated_at, down-migration comment
- [0.6.12](#0612--2026-04-01) — Production readiness review: duplicate_detection case-insensitive matching, time_window start==end guard, set_provider terminal status lockdown, IdempotencyKey empty-string validation
- [0.6.11](#0611--2026-04-01) — Cross-crate consistency review: velocity_limit currency-aware filtering, first_time_merchant case-insensitive matching, amount_cap tracing context
- [0.6.10](#0610--2026-04-01) — Input boundary enforcement: positive-amount validation, string length bounds on all audit-persisted fields, escalation infinite-loop prevention, condition tree depth limit, ProviderId encapsulation, AuditQuery private fields, DB constraints for amount/currency/rail/api_key, boundary tests
- [0.6.9](#069--2026-04-01) — Final pre-production sweep: In operator fail-safe logging, metadata field bounds, escalation threshold >= semantics, metadata field resolution in condition evaluator, regex cache eviction, PaymentSummary category, set_provider write-once
- [0.6.8](#068--2026-04-01) — Production review: Decimal precision in condition evaluator, EscalationThresholdEvaluator, Payment provider field encapsulation, AuditEntry on_chain_tx_hash, CountryCode validation
- [0.6.7](#067--2026-04-01) — Production audit: Payment deserialization validates state machine, panic elimination in policy hot path, ProviderError retryability, PaymentCategory::Other bounded, audit/profile schema hardening
- [0.6.6](#066--2026-04-01) — Production hardening: currency-aware spend/duplicate rules, case-insensitive merchant matching, typed ProviderId, regex caching, proportionality stub restricted
- [0.6.5](#065--2026-04-01) — Production readiness review: proportionality stub unregistered, Payment::status encapsulated, NotIn fail-safe, 8 MerchantCheck tests, payments.status CHECK constraint, policy rules index optimized
- [0.6.4](#064--2026-04-01) — Pre-production audit: duplicate_detection guard, spend_rate month fix, time_window .hour(), geographic case-insensitivity, audit query error propagation, offset DoS guard, 5 new tests
- [0.6.3](#063--2026-04-01) — Misconfiguration guard: input validation on velocity/time_window rules, schema fix for virtual_cards, CountryCode type consistency
- [0.6.2](#062--2026-04-01) — Production hardening: spend rate bypass fix, schema alignment, audit writer improvements, 5 new tests
- [0.6.1](#061--2026-04-01) — Cross-crate quality review: timezone support, explicit rule types, regex, audit query builder, 10 new tests
- [0.6.0](#060--2026-04-01) — Provider crate: trait abstraction + mock + registry
- [0.5.0](#050--2026-04-01) — Policy engine crate: 12 rule types + evaluation engine
- [0.4.0](#040--2026-04-01) — Audit crate: append-only writer + query reader
- [0.3.0](#030--2026-03-31) — Database schema and migrations
- [0.2.1](#021--2026-03-31) — Formatting fixes for CI compliance
- [0.2.0](#020--2026-03-31) — Core domain models crate
- [0.1.0](#010--2026-03-31) — Monorepo skeleton, tooling & infrastructure

---

## 0.7.3 — 2026-04-02

**Phase 7.3: Cross-Crate Audit — Settlement Amount Validation, Routing Candidate Guards, Latency Invariant, Idempotency Config Validation, Selector Bounds Hardening**

Full-crate production readiness audit (models, router) targeting six remaining consistency gaps found during a systematic review of all Phases 1–7 code. All changes are additive — no reverts of previous hardenings.

### Fixed

- **`ProviderResponseRecord.amount_settled` accepts zero/negative values — corrupted settlement records (HIGH)** — The custom `Deserialize` validated string field lengths and whitespace (v0.7.2) but had no check on the financial amount. A buggy provider returning `amount_settled: -5.00` or `0.00` would permanently store an invalid settlement in the append-only audit ledger, breaking reconciliation downstream. Added `> Decimal::ZERO` check, matching the established pattern from `PaymentRequest.amount` (v0.6.10)
- **`IdempotencyConfig.lock_ttl_secs` has no validation — zero TTL silently breaks idempotency (HIGH)** — `ScoringWeights` and `CircuitBreakerConfig` both have `validate()` methods called at construction time (v0.7.2), but `IdempotencyConfig` had no equivalent. A `lock_ttl_secs = 0` would create locks with zero TTL — either never expiring (permanent payment block) or expiring instantly (no double-payment protection), depending on the store implementation. Added `validate()` method and changed `IdempotencyGuard::new()` to return `Result<Self, RoutingError>` with validation at construction time, matching the `ProviderScorer::new()` and `CircuitBreaker::new()` pattern
- **`RoutingCandidate.score` accepts NaN/Infinity — breaks comparison-based sorting (MEDIUM)** — Used derived `Deserialize` with no validation. NaN breaks `f64` comparisons (NaN != NaN, NaN < x is always false), which would silently corrupt the scorer's ranking. `ProviderHealth.error_rate_5m` already validates `is_finite()` (v0.6.8) — this field was missed. Added custom `Deserialize` with `is_finite()` check
- **`RoutingCandidate.estimated_fee` accepts negative values — inverts cost optimization (MEDIUM)** — Negative fees would reverse the direction of cost-based scoring (a provider with fee `-$10` would appear cheapest when it should be invalid). Added `>= Decimal::ZERO` check in the same custom `Deserialize` impl
- **`ProviderHealth` accepts `p50_latency_ms > p99_latency_ms` — statistically impossible values (MEDIUM)** — The 99th percentile latency must always be >= the 50th percentile by definition. Invalid data from an external health source would corrupt scoring calculations. Added `p50_latency_ms <= p99_latency_ms` validation in the existing custom `Deserialize`
- **`build_reason()` in selector uses `== 1` check instead of `< 2` — fragile bounds logic (LOW)** — The function checked `candidates.len() == 1` before accessing `candidates[1]`. While functionally correct (the caller guarantees non-empty), the safety depended on code ordering rather than an explicit bounds check. Changed to `candidates.len() < 2` so the guard directly protects the index access regardless of upstream changes

### Added

- `> Decimal::ZERO` validation for `ProviderResponseRecord.amount_settled` in custom `Deserialize`
- Custom `Deserialize` for `RoutingCandidate` with `score.is_finite()` and `estimated_fee >= Decimal::ZERO` checks
- `p50_latency_ms <= p99_latency_ms` validation in `ProviderHealth` custom `Deserialize`
- `IdempotencyConfig::validate()` method with `lock_ttl_secs > 0` check
- `IdempotencyGuard::new()` returns `Result<Self, RoutingError>` with config validation at construction time
- `Debug` impl for `IdempotencyGuard` (required by `Result::unwrap_err()` in tests)
- 12 new tests: ProviderResponseRecord zero/negative amount_settled (2), RoutingCandidate NaN score + negative fee + zero fee + valid (4), ProviderHealth p50 > p99 + p50 == p99 (2), IdempotencyConfig zero TTL + nonzero TTL + default (3), IdempotencyGuard rejects zero TTL (1)

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 305/305 passing (121 models + 14 audit + 103 policy + 17 providers + 50 router) |

---

## 0.7.2 — 2026-04-02

**Phase 7.2: Production Readiness Review — ProviderResponseRecord Whitespace Guards, Router Config Validation Enforcement, MerchantCheckEvaluator Doc Correction**

Full-crate production readiness review (models, policy, router) targeting three remaining consistency gaps: a deserialization path that accepted empty/whitespace-only strings for audit-persisted provider response fields, router config validation methods that existed but were never called at construction time, and a doc comment that directed operators to the wrong field name. All changes are additive — no reverts of previous hardenings.

### Fixed

- **`ProviderResponseRecord.transaction_id` and `.status` accept empty/whitespace-only strings — meaningless audit entries (MEDIUM)** — The custom `Deserialize` validated max length (500 / 255 chars, v0.6.14) but allowed `""` and `"   "` through. The programmatic write path `Payment::set_provider()` already validates `transaction_id.trim().is_empty()` (v0.7.1), but the deserialization path — the boundary for data coming back from provider APIs — had no equivalent guard. A buggy or malicious provider returning whitespace-only values would permanently store meaningless references in the append-only audit ledger. Added `trim().is_empty()` checks for both `transaction_id` and `status` before the max-length checks, matching the pattern established by `HumanReviewRecord.reviewer_id` (v0.7.1) and `RoutingDecision.reason` (v0.7.1)
- **`ProviderScorer::new()` and `CircuitBreaker::new()` accept invalid config — silent scoring/breaker corruption (MEDIUM)** — `ScoringWeights::validate()` and `CircuitBreakerConfig::validate()` contain proper checks for NaN, negative values, zero windows, and out-of-range thresholds, but neither `ProviderScorer::new()` nor `CircuitBreaker::new()` called them. Invalid configs (NaN weights, zero error rate windows) would silently corrupt provider scoring or circuit breaker behavior. Changed both constructors to return `Result<Self, RoutingError>` and call `validate()` at construction time. Relaxed `cooldown_secs == 0` rejection — zero cooldown is semantically valid (instant retry on next request)
- **`MerchantCheckEvaluator` doc comment says field `"merchant"` but code matches `"recipient.identifier"` — operator misconfiguration vector (LOW)** — The doc comment directed operators to use `field: "merchant"` in condition trees, but the implementation matches `field == "recipient.identifier"`. An operator following the docs would create rules that silently fail to match — a policy bypass via misconfiguration. Updated doc comment to match implementation and corrected allow-list/deny-list semantics description

### Added

- `trim().is_empty()` whitespace validation for `ProviderResponseRecord.transaction_id` and `.status` in custom `Deserialize`
- `ProviderScorer::new()` returns `Result<Self, RoutingError>` with `ScoringWeights::validate()` call
- `CircuitBreaker::new()` returns `Result<Self, RoutingError>` with `CircuitBreakerConfig::validate()` call
- `Debug` impl for `ProviderScorer` and `CircuitBreaker` (required by `Result::unwrap_err()` in tests)
- 8 new tests: ProviderResponseRecord empty/whitespace transaction_id (2), empty/whitespace status (2), ProviderScorer rejects NaN/negative weights (2), CircuitBreaker rejects zero window/invalid threshold (2)

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 293/293 passing (113 models + 14 audit + 103 policy + 17 providers + 46 router) |

---

## 0.7.1 — 2026-04-02

**Phase 7.1: Cross-Crate Consistency Review — Empty-String Guards on Audit-Bound Fields, Positive-Value Validation on Spending Limits**

Full-crate production readiness review (models, policy) targeting six remaining consistency gaps where the established validation pattern — empty-string rejection on audit-persisted fields (v0.6.10–v0.6.16) and positive-amount enforcement on financial values (v0.6.10) — was not applied uniformly. All changes are additive — no reverts of previous hardenings.

### Fixed

- **`HumanReviewRecord.reviewer_id` accepts empty/whitespace-only string — anonymous audit entry (MEDIUM)** — The `reviewer_id` field identifies who made the human approval/rejection decision. The custom `Deserialize` validated max length (255 chars, v0.6.10) but allowed `""` and `"   "` through, undermining audit trail accountability. Added `trim().is_empty()` check before the max-length check, matching the pattern established by `Justification.summary` (v0.6.15) and `Recipient.identifier` (v0.6.15)
- **`RoutingDecision.reason` accepts empty/whitespace-only string — meaningless audit entry (MEDIUM)** — The `reason` field is the human-readable explanation of provider selection, persisted permanently to the append-only audit ledger. The custom `Deserialize` validated max length (1000 chars, v0.6.14) but allowed empty strings. Added `trim().is_empty()` check before the max-length check
- **`Payment::set_provider()` accepts empty/whitespace-only `transaction_id` — provider reference without identity (MEDIUM)** — The `set_provider()` method validated max length (500 chars, v0.6.14) but allowed `""` and `"   "`. At the point this method is called, the payment has been dispatched to a provider and should always have a real transaction identifier. Added `trim().is_empty()` check before the max-length check
- **`AgentProfile` spending limits accept zero/negative values — nonsensical limits (MEDIUM)** — `max_per_transaction`, `max_daily_spend`, `max_weekly_spend`, `max_monthly_spend` are `Decimal` fields with no validation. Zero limits would silently block all payments; negative limits are semantically invalid. The database has CHECK constraints (`>= 0` from v0.6.10 migrations), but the Rust model allowed negative values through — breaking the defense-in-depth pattern established for `PaymentRequest.amount` (positive check since v0.6.10). Added custom `Deserialize` with `> Decimal::ZERO` validation on all four limits and `escalation_threshold` when present
- **`CardControls` spending limits accept zero/negative values when present — invalid card limits (LOW)** — `max_per_transaction` and `max_per_cycle` are `Option<Decimal>` with no validation when `Some`. Added custom `Deserialize` with `> Decimal::ZERO` validation when the value is present
- **Regex cache eviction comment claims FIFO but HashMap gives arbitrary order (LOW)** — The comment on the regex cache eviction in the condition evaluator said "oldest entry (by insertion order)" but `HashMap` does not guarantee insertion order — `keys().next()` returns an arbitrary key. Updated the comment to accurately describe the behavior as arbitrary eviction. Functional impact: none (the cache still works correctly; evicted patterns are re-compiled on next use)

### Added

- Custom `Deserialize` for `AgentProfile` with positive-value validation on all spending limits and optional `escalation_threshold`
- Custom `Deserialize` for `CardControls` with positive-value validation on optional spending limits
- 18 new tests: HumanReviewRecord empty/whitespace reviewer_id (2), RoutingDecision empty/whitespace/valid reason (3), set_provider empty/whitespace transaction_id (2), AgentProfile zero/negative limits on 4 fields + escalation_threshold + valid + none-threshold (7), CardControls zero/negative limits + valid + none (4)

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 285/285 passing (109 models + 14 audit + 103 policy + 17 providers + 42 router) |

---

## 0.7.0 — 2026-04-01

**Phase 7: Routing Engine Crate — Provider Scoring, Circuit Breakers, Idempotency Guards**

Implements the `cream-router` crate — the provider selection engine that scores viable providers based on cost, speed, health, and rail preference, enforces per-provider circuit breakers with automatic demotion, and provides cross-provider idempotency guards to prevent double-payments during failover.

### Added

- **`ProviderScorer`** — multi-factor ranking algorithm with configurable weights (cost 0.3, speed 0.2, health 0.3, preference 0.2). Binary filters exclude circuit-broken providers, unsupported currencies, and restricted rails before scoring
- **`CircuitBreaker`** — per-provider Closed → Open → HalfOpen state machine. Trips when error rate exceeds configurable threshold (default 50% over 5-min window). Auto-recovers via HalfOpen testing after cooldown (default 60s). `CircuitBreakerStore` trait enables in-memory unit tests without Redis
- **`IdempotencyGuard`** — distributed lock preventing double-payments across provider failovers. `acquire()` / `release()` / `complete()` semantics with NX+EX Redis lock pattern. `IdempotencyStore` trait enables in-memory unit tests
- **`RouteSelector`** — orchestrates health loading, scoring, and selection. Returns `RoutingDecision` with ranked candidates. `HealthSource` trait decouples health data retrieval
- **`ProviderCapabilities`** — static provider metadata (supported rails, currencies, fee schedule). Scaffold placeholder for Phases 12-14 real provider data
- **`RouterConfig`** — validated configuration for scoring weights, circuit breaker thresholds, and idempotency TTL
- **`RoutingError`** — 7-variant error enum covering no viable provider, all exhausted, idempotency conflict, Redis errors, provider errors, and config errors
- **`StaticHealthSource`** and **`InMemoryCircuitBreakerStore`** / **`InMemoryIdempotencyStore`** — test doubles for Redis-dependent components
- 42 new tests across all modules

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 267/267 passing (91 models + 14 audit + 103 policy + 17 providers + 42 router) |

---

## 0.6.16 — 2026-04-01

**Phase 6.16: Production Readiness Review — ProviderId Empty-String Validation**

Full-crate production readiness audit (models, policy, providers, audit, router, api, migrations) targeting one remaining defense-in-depth gap: `ProviderId` accepted empty strings on all construction paths, inconsistent with the validated-ID pattern established by `IdempotencyKey` and `CountryCode`. An empty provider ID could be written to `RoutingDecision.selected` and persisted permanently to the append-only audit ledger. All changes are additive — no reverts of previous hardenings.

### Fixed

- **`ProviderId` accepts empty strings — invalid provider ID persisted to audit ledger (MEDIUM)** — `ProviderId::new("")` succeeded silently and the derived `Deserialize` had no validation, unlike `IdempotencyKey` (empty-string rejection on `new()`, `try_new()`, and custom `Deserialize` since v0.6.12) and `CountryCode` (format validation on all paths since v0.2.0). An empty provider ID in `RoutingDecision.selected` would permanently store an invalid reference in the append-only audit ledger. Added `assert!` in `new()`, fallible `try_new()` constructor, and custom `Deserialize` impl that rejects empty strings — matching the `IdempotencyKey` pattern exactly

### Added

- `ProviderId::try_new()` fallible constructor for untrusted input
- Custom `Deserialize` for `ProviderId` with empty-string validation
- 5 new tests: ProviderId rejects empty `new()` (1), `try_new()` rejects empty (1), `try_new()` accepts valid (1), deserialize rejects empty (1), deserialize accepts valid (1)

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 225/225 passing (91 models + 14 audit + 103 policy + 17 providers) |

---

## 0.6.15 — 2026-04-01

**Phase 6.15: Production Readiness Review — Escalation Loop via Human Review, Empty Recipient Identifier & Hollow Justification**

Full-crate production readiness audit (models, policy, providers, audit, router, api, migrations) targeting three remaining defense-in-depth gaps in deserialization validation: a human review decision that could re-escalate an already-escalated payment, an empty recipient identifier that would route a payment to nobody, and an empty/whitespace-only justification summary that would permanently store a meaningless entry in the append-only audit ledger. All changes are additive — no reverts of previous hardenings.

### Fixed

- **`HumanReviewRecord.decision` accepts `Escalate` — escalation loop vector (MEDIUM)** — `EscalationConfig::on_timeout` already rejects `Escalate` (v0.6.10) to prevent infinite timeout→escalate→timeout cycles. However, `HumanReviewRecord.decision` had no equivalent guard — a human reviewer could submit `decision: ESCALATE`, re-queuing an already-escalated payment into another escalation cycle. Added validation in custom `Deserialize` that rejects `PolicyAction::Escalate` with a clear error message, consistent with the `EscalationConfig` invariant
- **`Recipient.identifier` accepts empty string — payment to nobody (MEDIUM)** — The `Recipient` custom `Deserialize` validates maximum length (500 chars, added in v0.6.10) but allowed `""` through. An empty identifier is semantically invalid — no provider can route a payment to an empty merchant ID, wallet address, or bank account. Added empty-string check before the max-length check
- **`Justification.summary` accepts empty/whitespace-only string — hollow justification persisted to audit ledger (MEDIUM)** — The `Justification` custom `Deserialize` validates maximum length (2000 chars, added in v0.6.10) but allowed `""` and `"   "` through. The product's core differentiator is structured agent justification — an empty summary defeats the purpose and would permanently store a meaningless entry in the append-only audit ledger. The policy engine's `JustificationQuality` rule catches this downstream, but defense-in-depth at the model boundary prevents invalid data from ever entering the domain. Added `trim().is_empty()` check before the max-length check

### Added

- 6 new tests: HumanReviewRecord rejects Escalate (1), HumanReviewRecord accepts Approve (1), HumanReviewRecord accepts Block (1), Recipient empty identifier rejection (1), Justification empty summary rejection (1), Justification whitespace-only summary rejection (1)

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 220/220 passing (86 models + 14 audit + 103 policy + 17 providers) |

---

## 0.6.14 — 2026-04-01

**Phase 6.14: Production Sweep — Provider Response Bounds, Case-Insensitive Condition Operators & Health Metric Validation**

Cross-crate production readiness review targeting unbounded external-origin strings persisted to the immutable audit ledger, inconsistent case-sensitivity semantics across condition evaluator operators, and unvalidated routing health metrics. All changes are additive — no reverts of previous hardenings.

### Fixed

- **`ProviderResponseRecord.transaction_id` and `.status` unbounded — audit log bloat from provider responses (HIGH)** — These fields are populated from external provider API responses and written to the append-only audit ledger with no length limits. A buggy or malicious provider could return a multi-MB transaction ID or status string, bloating the audit trail permanently. Added custom `Deserialize` with `MAX_PROVIDER_TRANSACTION_ID_LEN` (500 chars) and `MAX_PROVIDER_STATUS_LEN` (255 chars), matching the established bounded-string pattern
- **`Payment::set_provider()` accepts unbounded `transaction_id` — audit log bloat via method call (HIGH)** — The `set_provider()` method is the programmatic write path for provider transaction IDs onto Payment entities. Unlike deserialization paths, it had no length validation. Added `MAX_PROVIDER_TRANSACTION_ID_LEN` check before accepting the value, returning `DomainError::PolicyViolation` on overflow
- **Condition evaluator `Equals`/`NotEquals`/`Contains` are case-sensitive while `In`/`NotIn` are case-insensitive — policy bypass vector (HIGH)** — In v0.6.13, `In`/`NotIn` operators were made case-insensitive via `case_insensitive_contains()`. However, `Equals`, `NotEquals`, and `Contains` still used exact JSON equality / exact `String::contains()`. An operator writing `recipient.identifier Equals "stripe_merch_123"` would fail to match `"STRIPE_MERCH_123"`, but the same check via `In ["stripe_merch_123"]` would succeed. Added `case_insensitive_equals()` helper for `Equals`/`NotEquals` and `to_ascii_lowercase()` for `Contains`, making all string comparison operators consistently case-insensitive
- **`ProviderHealth.error_rate_5m` accepts NaN, Infinity, negative, and >1.0 values — routing engine score corruption (MEDIUM)** — The routing engine uses `error_rate_5m` in provider scoring calculations. `f64::NAN` poisons all comparisons (NaN != NaN, NaN > x is false, etc.), producing undefined ranking behavior. Negative or >1.0 values produce nonsensical scores. Added custom `Deserialize` validating `is_finite()` and range `[0.0, 1.0]`

### Added

- `MAX_PROVIDER_TRANSACTION_ID_LEN` constant (500) and `MAX_PROVIDER_STATUS_LEN` constant (255) in `cream-models`
- Custom `Deserialize` for `ProviderResponseRecord` with per-field length bounds
- Custom `Deserialize` for `ProviderHealth` with `error_rate_5m` range validation
- `case_insensitive_equals()` helper in condition evaluator
- 18 new tests: ProviderResponseRecord bounds (4), set_provider transaction_id bounds (2), Equals/NotEquals case-insensitive (5), Contains case-insensitive (2), ProviderHealth validation (5)

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 214/214 passing (80 models + 14 audit + 103 policy + 17 providers) |

---

## 0.6.13 — 2026-04-01

**Phase 6.13: Cross-Crate Production Audit — Audit Ledger Data Gap, Terminal State Semantics, Condition Evaluator Case-Sensitivity & Schema Consistency**

Full-crate review of all completed code (models, policy, providers, audit, api scaffold) and database migrations targeting data model/query mismatches, state machine semantic gaps, case-sensitivity bypass in the generic condition evaluator, and schema inconsistency across mutable tables. All changes are additive — no reverts of previous hardenings.

### Fixed

- **`AuditEntry` missing `payment_id` field — audit reader drops payment linkage (HIGH)** — The `PgAuditWriter` INSERT included `payment_id` as the 5th column, but the `AuditEntry` Rust struct had no `payment_id` field and all three `PgAuditReader` SELECT queries omitted it from the projection. The data existed in the database but was invisible to Rust code — callers of `get_by_payment()` received entries but could not verify which payment they belonged to. Added `payment_id: Option<PaymentId>` to `AuditEntry`, updated all SELECT queries to include `payment_id` in the projection, updated `AuditRow` intermediate struct and all row mappings across `query()`, `get_by_id()`, and `get_by_payment()`
- **`PaymentStatus::TimedOut` excluded from `is_terminal()` — misleading terminal state check (HIGH)** — `TimedOut` can only transition to `Blocked` (another terminal state). `is_terminal()` returned `false` for `TimedOut`, which is semantically incorrect — no forward progress (settlement) is possible from `TimedOut`. Downstream code checking `is_terminal()` to decide "can this payment still settle?" would incorrectly treat `TimedOut` as active. Added `PaymentStatus::TimedOut` to `is_terminal()`. Note: `counts_toward_spend()` already correctly excluded `TimedOut`, so no policy engine impact
- **`In`/`NotIn` operators in condition evaluator are case-sensitive for strings — bypass vector (MEDIUM)** — The generic condition tree walker's `In` and `NotIn` used `arr.contains(field)` (JSON value equality, case-sensitive for strings). Operators writing custom `PolicyCondition` trees with string-valued `In`/`NotIn` checks (e.g., merchant identifiers, category names) could be bypassed by submitting values with different casing. Dedicated rule evaluators (MerchantCheck, FirstTimeMerchant, DuplicateDetection) already handled case-insensitivity; the generic evaluator was the gap. Added `case_insensitive_contains()` helper that uses `eq_ignore_ascii_case` for string values and falls back to exact JSON equality for non-strings
- **`webhook_endpoints` missing `updated_at` column and trigger — schema inconsistency (MEDIUM)** — Every other mutable table (agent_profiles, agents, policy_rules, payments, virtual_cards) has an `updated_at TIMESTAMPTZ` column with the `set_updated_at()` trigger. `webhook_endpoints` was the only mutable table missing both, meaning webhook endpoint modifications had no timestamp trail. Added migration `20260401200007` with `updated_at` column and trigger
- **Down-migration `20260331200001` `set_updated_at()` drop lacked explanation (LOW)** — Added clarifying comment documenting why the function drop is safe in this position (down migrations execute in reverse chronological order, so this migration runs last after all dependent tables are already dropped)

### Added

- `AuditEntry.payment_id: Option<PaymentId>` field with full reader/writer support
- `case_insensitive_contains()` helper in condition evaluator
- Migration `20260401200007_add_webhook_endpoints_updated_at` (column + trigger)
- 6 new tests: `timed_out_is_terminal` (1), `all_terminal_states_are_terminal` (1), condition evaluator In/NotIn case-insensitive matching (4)

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 196/196 passing (69 models + 14 audit + 96 policy + 17 providers) |

---

## 0.6.12 — 2026-04-01

**Phase 6.12: Production Readiness Review — Duplicate Detection Bypass, Time Window Misconfiguration, State Machine Hardening & Idempotency Validation**

Comprehensive production readiness audit across all completed crates (models, policy, providers, audit, api scaffold). The v0.6.6–v0.6.11 case-insensitive matching fixes were not applied to `DuplicateDetectionEvaluator`; `TimeWindowEvaluator` silently accepted `start == end` configurations producing ambiguous all-block behavior; `Payment::set_provider()` permitted mutation of terminal statuses (Settled, Failed); and `IdempotencyKey` accepted empty strings, defeating idempotency guarantees. All changes are additive — no reverts of previous hardenings.

### Fixed

- **`DuplicateDetectionEvaluator` uses case-sensitive merchant comparison — bypass vector (HIGH)** — In v0.6.6, `MerchantCheckEvaluator` was fixed to use `eq_ignore_ascii_case()`. In v0.6.11, `FirstTimeMerchantEvaluator` was fixed to use `to_ascii_lowercase()`. `DuplicateDetectionEvaluator` was missed in both rounds — it used `==` for `recipient_identifier`. An agent could bypass duplicate detection by submitting `"STRIPE_MERCH_123"` then `"stripe_merch_123"` — same merchant, same amount, same window, passes the check. Added `to_ascii_lowercase()` normalization matching the established pattern
- **`TimeWindowEvaluator` accepts `start == end` — ambiguous all-block behavior (HIGH)** — When `allowed_hours_start == allowed_hours_end` (e.g., both 9), the normal range branch evaluates `hour >= 9 && hour < 9`, which is always false, silently blocking all payments at all hours. An operator intending "allow only hour 9" or "no restriction" gets everything blocked with no warning. Added validation in `extract_hours()` that rejects `start == end` with a `tracing::warn!` and skips the rule
- **`Payment::set_provider()` allows mutation of terminal statuses (MEDIUM)** — `set_provider()` accepted `Settled` and `Failed` statuses in its valid status match. These are terminal states — once a payment reaches settlement or failure, its provider info should be immutable. The write-once guard prevented overwrite if already set, but if provider was never assigned before reaching a terminal state (edge case), the payment could be mutated post-completion. Removed `Settled` and `Failed` from the valid status list, restricting to `Approved | Submitted` only
- **`IdempotencyKey` accepts empty strings — defeats idempotency guarantees (MEDIUM)** — `IdempotencyKey::new("")` created a valid key. If two unrelated requests submitted empty idempotency keys, they would collide in the Redis lock, causing the second to be treated as a duplicate of the first. Added `assert!(!key.is_empty())` in `new()`, `try_new()` fallible constructor for untrusted input, and custom `Deserialize` impl that rejects empty strings at deserialization time

### Added

- `IdempotencyKey::try_new()` fallible constructor for untrusted input
- Custom `Deserialize` for `IdempotencyKey` with empty-string validation
- 8 new tests: duplicate_detection case-insensitive matching (2), time_window start==end rejection (1), set_provider terminal status rejection (2), IdempotencyKey empty-string rejection (3)

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 190/190 passing (67 models + 14 audit + 92 policy + 17 providers) |

---

## 0.6.11 — 2026-04-01

**Phase 6.11: Cross-Crate Consistency Review — Currency Filtering, Case-Insensitive Matching & Tracing**

Comprehensive review of all completed crates (models, policy, providers, audit, api scaffold) targeting inconsistencies introduced across the v0.6.1–0.6.10 hardening cycle. The v0.6.6 currency-awareness fix for `SpendRateEvaluator` and `DuplicateDetectionEvaluator` was not applied to `VelocityLimitEvaluator`; the v0.6.6 case-insensitive matching fix for `MerchantCheckEvaluator` was not applied to `FirstTimeMerchantEvaluator`; and `AmountCapEvaluator` lacked tracing context for triggered decisions. All changes are additive — no reverts of previous hardenings.

### Fixed

- **`VelocityLimitEvaluator` ignores currency — cross-currency bypass (HIGH)** — In v0.6.6, `SpendRateEvaluator` and `DuplicateDetectionEvaluator` were fixed to filter by `p.currency == ctx.request.currency`. `VelocityLimitEvaluator` was missed — it counted all payments regardless of currency. An agent with a 5-transaction/hour limit could submit 5 SGD payments, then switch to USD and submit 5 more, all passing the velocity check. Added `&& p.currency == ctx.request.currency` filter, matching the established pattern
- **`FirstTimeMerchantEvaluator` uses case-sensitive HashSet lookup (MEDIUM)** — In v0.6.6, `MerchantCheckEvaluator` was fixed to use `eq_ignore_ascii_case()` for merchant identifier matching. `FirstTimeMerchantEvaluator` used `HashSet::contains()`, which is case-sensitive. If `known_merchants` contained `"stripe_merch_123"` but the request had `"Stripe_Merch_123"`, it was incorrectly flagged as a first-time merchant. Changed to case-insensitive iteration matching, consistent with `MerchantCheckEvaluator`
- **`AmountCapEvaluator` triggers silently with no tracing context (LOW)** — When `amount_cap` triggered, no log was emitted with the amount, currency, or limit, making it harder to diagnose policy blocks in production. Added `tracing::info!` with amount, currency, and limit fields. Also added doc comment clarifying that profile limits are currency-agnostic numeric ceilings

### Added

- 4 new tests: velocity_limit currency filtering (2), first_time_merchant case-insensitive matching (2)

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 182/182 passing (62 models + 14 audit + 89 policy + 17 providers) |

---

## 0.6.10 — 2026-04-01

**Phase 6.10: Input Boundary Enforcement — Amount Validation, String Bounds, Infinite-Loop Prevention & Schema Constraints**

Comprehensive review targeting unbounded input fields persisted to the append-only audit ledger, missing amount validation allowing zero/negative payments, an infinite escalation loop vector in `EscalationConfig`, unbounded `PolicyCondition` tree depth, a public inner field on `ProviderId` breaking typed-ID encapsulation, bypassable pagination guards on `AuditQuery`, and missing database-level enforcement for payment amounts, currency enums, and API key uniqueness. All changes are additive — no reverts of previous hardenings.

### Fixed

- **`PaymentRequest.amount` accepts zero and negative values (CRITICAL)** — No validation existed on the amount field. A payment with `amount: -100` or `amount: 0` would pass through the models layer and enter the policy engine. Added custom `Deserialize` for `PaymentRequest` that rejects `amount <= 0` at deserialization time. Database migration adds `CHECK (amount > 0)` as defense-in-depth
- **`Justification.summary` is completely unbounded (HIGH)** — The audit ledger is append-only — an agent submitting a 100MB summary would persist it forever with no way to delete. Added `MAX_JUSTIFICATION_SUMMARY_LEN` (2000 chars) with custom `Deserialize`. Also bounded `task_id` and `expected_value` to `MAX_JUSTIFICATION_FIELD_LEN` (500 chars)
- **`EscalationConfig.on_timeout` can be `Escalate` — infinite loop (HIGH)** — If `on_timeout: ESCALATE`, the payment cycles through timeout → escalate → timeout → escalate forever, blocking indefinitely. Added custom `Deserialize` that rejects `on_timeout == Escalate` with a clear error message
- **`PolicyCondition` tree has no depth limit (HIGH)** — Recursive `All(All(All(...)))` nesting 10,000+ levels deep causes stack overflow during deserialization. Added custom `Deserialize` with `MAX_CONDITION_DEPTH` (32 levels) enforced via depth-tracked recursive parsing
- **`Recipient.identifier` and `Recipient.name` unbounded (HIGH)** — Merchant IDs, wallet addresses, and display names had no length limits. Added `MAX_RECIPIENT_IDENTIFIER_LEN` (500) and `MAX_RECIPIENT_NAME_LEN` (255) with custom `Deserialize`
- **`HumanReviewRecord.reviewer_id` and `reason` unbounded (MEDIUM)** — Both fields are persisted to the append-only audit log with no length limits. Added `MAX_REVIEWER_ID_LEN` (255) and `MAX_REVIEW_REASON_LEN` (2000) with custom `Deserialize`
- **`RoutingDecision.reason` unbounded (MEDIUM)** — Routing explanation persisted to audit log with no length limit. Added `MAX_ROUTING_REASON_LEN` (1000) with custom `Deserialize`
- **`ProviderId` inner field is `pub` (MEDIUM)** — `ProviderId(pub String)` exposed the inner string for direct mutation despite `new()` and `as_str()` accessors existing, breaking the typed-ID encapsulation pattern used by all other ID types. Changed to `ProviderId(String)` (private inner)
- **`AuditQuery` fields are public — pagination guards bypassable (MEDIUM)** — `effective_limit()` and `effective_offset()` clamp values to safe ranges, but callers could set `query.offset = 1_000_000_000` directly to bypass the guard. Made all fields private, added builder methods (`AuditQuery::new().agent_id(...).limit(...).offset(...)`) that always route through clamping
- **Missing DB constraints: payment amount, currency, rail, api_key uniqueness (HIGH)** — Added migration `20260401200006` with: `payments.amount > 0`, `payments.amount_settled > 0`, `agents.api_key_hash UNIQUE`, `payments.currency` CHECK (33 valid enum values), `payments.preferred_rail` CHECK (6 valid values), `provider_health.error_rate_5m` between 0.0–1.0, `provider_health` latency non-negative

### Added

- `MAX_JUSTIFICATION_SUMMARY_LEN` constant (2000) and `MAX_JUSTIFICATION_FIELD_LEN` constant (500) in `cream-models`
- `MAX_RECIPIENT_IDENTIFIER_LEN` constant (500) and `MAX_RECIPIENT_NAME_LEN` constant (255) in `cream-models`
- `MAX_REVIEWER_ID_LEN` constant (255) and `MAX_REVIEW_REASON_LEN` constant (2000) in `cream-models`
- `MAX_ROUTING_REASON_LEN` constant (1000) and `MAX_CONDITION_DEPTH` constant (32) in `cream-models`
- Custom `Deserialize` for `PaymentRequest`, `Justification`, `EscalationConfig`, `PolicyCondition`, `Recipient`, `HumanReviewRecord`, `RoutingDecision`
- `AuditQuery` builder API (`new()`, `agent_id()`, `from()`, `to()`, `status()`, `category()`, `min_amount()`, `max_amount()`, `limit()`, `offset()`)
- Migration `20260401200006_payment_amount_checks_and_enum_constraints` (3 amount constraints, 1 unique, 2 enum CHECKs, 2 provider health bounds)
- 22 new tests: amount validation (3), justification bounds (5), recipient bounds (3), escalation loop prevention (3), condition depth limit (2), boundary semantics — amount_cap exact (1), velocity_limit exact (1), spend_rate exact (2), time_window boundaries (2)

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 178/178 passing (62 models + 14 audit + 85 policy + 17 providers) |

---

## 0.6.9 — 2026-04-01

**Phase 6.9: Final Pre-Production Sweep — Fail-Safe Consistency, Bounds Enforcement & Future-Proofing**

Comprehensive review targeting fail-safe inconsistencies in the condition evaluator, unbounded metadata fields, escalation threshold off-by-one semantics, missing field resolution paths, suboptimal cache eviction, and write-once enforcement on provider assignment. All changes are additive — no reverts of previous hardenings.

### Fixed

- **`In` operator silently returns `false` on non-array without warning (HIGH)** — `NotIn` already logged a warning and returned `false` on non-array values (fail-safe, added in v0.6.5), but `In` returned `false` silently. A misconfigured deny-list rule using `In` with a non-array value would silently stop blocking. Added matching `tracing::warn!` for consistency so operators are alerted to misconfigured rules
- **`PaymentMetadata` fields are completely unbounded (HIGH)** — `agent_session_id`, `workflow_id`, and `operator_ref` accepted strings of any length with no validation. A malicious agent could submit multi-megabyte metadata strings that get persisted to the audit log. Added custom `Deserialize` with `MAX_METADATA_FIELD_LEN` (500 chars) validation, matching the existing `PaymentCategory::Other` bound pattern
- **`EscalationThresholdEvaluator` uses `>` instead of `>=` (MEDIUM)** — An operator setting `escalation_threshold: 500` expects payments of exactly $500 to require human approval. The `>` comparison meant exactly-at-threshold payments passed without escalation. Changed to `>=` to match operator intent. Updated test from `passes_at_exact_threshold` to `triggers_at_exact_threshold`
- **Condition evaluator cannot resolve `metadata.*` field paths (MEDIUM)** — The vision doc (Section 5.3) specifies operators can define policy rules matching on payment metadata. `metadata.workflow_id`, `metadata.agent_session_id`, and `metadata.operator_ref` resolved to `null` with a warning, silently passing any metadata-based rules. Added resolution for all three metadata sub-fields
- **Regex cache evicts all entries on overflow (MEDIUM)** — When the 256-pattern regex cache was full, `cache.clear()` evicted every compiled pattern. Under steady-state with 257+ unique patterns, every evaluation triggered full cache invalidation and recompilation. Changed to single-entry eviction so hot patterns survive overflow
- **`PaymentSummary` missing `category` field (LOW)** — The lightweight payment summary used by velocity/spend rate checks had no `category` field, preventing future category-velocity rules (e.g., "max 3 travel payments per day"). Added `category: PaymentCategory` field
- **`set_provider()` allows silent overwrite (LOW)** — Once provider info was set on a payment, a second `set_provider()` call would silently overwrite the original provider ID with no audit trail. During failover, this could mask which provider was actually attempted first. Made `set_provider()` write-once — returns an error if provider is already set

### Added

- `MAX_METADATA_FIELD_LEN` constant (500) in `cream-models`
- Custom `Deserialize` for `PaymentMetadata` with per-field length bounds
- `metadata.agent_session_id`, `metadata.workflow_id`, `metadata.operator_ref` field resolution in condition evaluator
- `category` field on `PaymentSummary` in `cream-policy` context
- 7 new tests: metadata bounds (3), set_provider write-once (1), metadata field resolution (2), In operator non-array (1)

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 156/156 passing (46 models + 14 audit + 79 policy + 17 providers) |

---

## 0.6.8 — 2026-04-01

**Phase 6.8: Production Review — Precision, Encapsulation & Schema Alignment**

Comprehensive review targeting financial precision in the condition evaluator, a missing escalation feature, incomplete field encapsulation on `Payment`, a schema/model mismatch in the audit ledger, and unvalidated `CountryCode` construction. All changes are additive — no reverts of previous hardenings.

### Fixed

- **Condition evaluator uses `f64` for amount comparisons (MEDIUM)** — `compare_numeric` in `evaluator.rs` converted financial amounts to IEEE 754 `f64`, introducing precision risk (e.g., `0.1 + 0.2 != 0.3`). Replaced with `compare_decimal` using `rust_decimal::Decimal` parsing. Handles string-serialized decimals (from `serde-with-str`), integer JSON values, and f64 JSON numbers via string round-trip. All primary rule evaluators already used Decimal directly; this closes the gap in the generic condition tree walker
- **`escalation_threshold` on AgentProfile is defined but never consumed (MEDIUM)** — The field existed in the model, database, and CHECK constraints, but no rule evaluator referenced it. An operator setting `escalation_threshold: 500` would expect payments above $500 to require human approval, but nothing happened. Added `EscalationThresholdEvaluator` that reads `profile.escalation_threshold` and returns `Escalate` (not Block). Registered in `PolicyEngine` as the 12th active evaluator. The evaluator hardcodes `PolicyAction::Escalate` regardless of the rule's action field, ensuring it always requires human approval rather than blocking
- **`Payment.provider_id` and `provider_transaction_id` are `pub` (LOW)** — In v0.6.5, `status` was made private to enforce the state machine, but `provider_id` and `provider_transaction_id` remained public. Runtime code could set them on a Pending payment, bypassing the invariant enforced by the custom Deserializer. Made both fields private, added `provider_id()` and `provider_transaction_id()` getters, and a `set_provider()` method that validates the current status is >= Approved
- **`AuditEntry` model missing `on_chain_tx_hash` field (LOW)** — The `audit_log` database table has an `on_chain_tx_hash TEXT` column, but the `AuditEntry` Rust struct didn't include it. Added `on_chain_tx_hash: Option<String>` to `AuditEntry`, updated `PgAuditWriter` INSERT to bind the field, and updated all three `PgAuditReader` query methods (query, get_by_id, get_by_payment) to SELECT and map the column
- **`CountryCode` accepts any string with no validation (LOW)** — `CountryCode::new("GARBAGE123")` succeeded silently. Added validation requiring exactly 2 ASCII alphabetic characters. `new()` panics on invalid input (for trusted/test contexts), `try_new()` returns `Result` for untrusted input. Custom `Deserialize` impl validates on deserialization. All codes are normalized to uppercase on construction for consistent comparison

### Added

- `EscalationThresholdEvaluator` in `cream-policy` — 12th active rule evaluator
- `compare_decimal` / `as_decimal` functions in `evaluator.rs` — Decimal-precise numeric comparison
- `Payment::provider_id()` getter, `Payment::provider_transaction_id()` getter, `Payment::set_provider()` validated setter
- `CountryCode::try_new()` fallible constructor with validation
- Custom `Deserialize` for `CountryCode` with format validation
- `AuditEntry.on_chain_tx_hash` field with reader/writer support
- 12 new tests: escalation threshold (5), set_provider validation (2), CountryCode validation (5)

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 149/149 passing (42 models + 14 audit + 76 policy + 17 providers) |

---

## 0.6.7 — 2026-04-01

**Phase 6.7: Production Audit — Deserialization Safety, Panic Elimination & Schema Hardening**

Comprehensive audit targeting deserialization bypass vectors, panic risks in the payment hot path, insufficient error classification for circuit breaker integration, unbounded string fields, and missing database constraints. All changes are additive — no reverts of previous hardenings.

### Fixed

- **Payment deserialization bypasses state machine (CRITICAL)** — Serde's derived `Deserialize` ignores field privacy, allowing construction of `Payment` in any status from untrusted JSON. Replaced with custom `Deserialize` impl using shadow struct pattern. Validates: `created_at <= updated_at`, and `provider_id`/`provider_transaction_id` must not be set for pre-submission statuses (Pending, Validating, PendingApproval)
- **`SpendRateEvaluator` uses `.expect()` in payment hot path (HIGH)** — `with_day(1).expect(...)` and `and_hms_opt(0,0,0).expect(...)` were panics in non-test code. Replaced with chained `.and_then()` + `.unwrap_or_else()` that falls back to a 30-day window with a warning log. The fallback is provably unreachable but eliminates all panic surface
- **`TimeWindowEvaluator` bare `.unwrap()` in UTC fallback (HIGH)** — Changed `FixedOffset::east_opt(0).unwrap()` to `.expect("UTC offset 0 is always valid")` with explicit `match` for clarity. Documents the invariant instead of silently panicking
- **`ProviderError` has insufficient variants for production (HIGH)** — Added 7 new error variants: `RateLimited`, `InvalidAmount`, `DuplicatePayment`, `InsufficientFunds`, `ComplianceBlocked`, `UnsupportedCurrency`, `UnsupportedCountry`. Added `is_retryable()` method classifying transient vs permanent errors for circuit breaker and failover logic
- **`PaymentCategory::Other` string unbounded (MEDIUM)** — Custom `Deserialize` impl rejects `Other(String)` values exceeding 500 characters (`MAX_CATEGORY_OTHER_LEN`). Prevents audit log bloat from malicious or runaway category strings
- **Missing `payment_id` index on `audit_log` (MEDIUM)** — `get_by_payment()` queries were full-table-scanning. Added `idx_audit_payment` index
- **Missing composite `(agent_id, timestamp)` index on `audit_log` (MEDIUM)** — The most common audit query pattern ("agent X's entries in date range Y") lacked an efficient index. Added `idx_audit_agent_timestamp`
- **`agent_profiles` amount fields accept negative values (MEDIUM)** — Added CHECK constraints: `max_per_transaction >= 0`, `max_daily_spend >= 0`, `max_weekly_spend >= 0`, `max_monthly_spend >= 0`, `escalation_threshold >= 0`, `version > 0`. Negative limits would silently invert policy enforcement

### Added

- `MAX_CATEGORY_OTHER_LEN` constant (500) in `cream-models`
- Custom `Deserialize` for `Payment` with invariant validation
- Custom `Deserialize` for `PaymentCategory` with length bounds
- `ProviderError::is_retryable()` method for circuit breaker integration
- 7 new `ProviderError` variants for production error classification
- Migration `20260401200005_hardening_indexes_and_checks` (2 indexes, 6 CHECK constraints)
- 9 new tests: Payment serde roundtrip (1), Payment deserialization rejection (2), Payment provider_id on submitted (1), PaymentCategory bounds (3), ProviderError retryability (2)

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 137/137 passing (35 models + 14 audit + 71 policy + 17 providers) |

---

## 0.6.6 — 2026-04-01

**Phase 6.6: Production Hardening — Bypass Vectors & Type Safety**

Comprehensive review targeting cross-currency bypass vectors, case-sensitivity inconsistencies, type safety gaps, and performance concerns. All changes are additive — no reverts of previous hardenings.

### Fixed

- **Spend rate and velocity ignore currency (CRITICAL)** — `SpendRateEvaluator` summed `p.amount` across all currencies without filtering by `ctx.request.currency`. An agent could bypass a $500/day SGD limit by submitting payments in USD. Fixed by adding `p.currency == ctx.request.currency` filter to `sum_payments_since()`
- **Duplicate detection ignores currency (CRITICAL)** — `DuplicateDetectionEvaluator` matched amounts without checking currency. A $100 USD payment followed by a $100 SGD payment to the same merchant was flagged as duplicate (incorrect). Fixed by adding currency equality check
- **Merchant identifier matching is case-sensitive (HIGH)** — `MerchantCheckEvaluator` used JSON value equality for `In`, `NotIn`, and `Equals` operators. A deny-list containing `"stripe_merch_123"` would not match `"Stripe_Merch_123"`. Fixed with `eq_ignore_ascii_case()`, consistent with the geographic evaluator fix in v0.6.4
- **`Payment.provider_id` is untyped `Option<String>` (MEDIUM)** — Changed to `Option<ProviderId>` in both `Payment` and `PaymentResponse`, consistent with the typed-ID discipline used for all other ID fields
- **Regex compiled on every `Matches` evaluation (MEDIUM)** — `evaluator.rs` called `Regex::new(pattern)` per invocation. Added `LazyLock<Mutex<HashMap>>` cache with 256-entry bound and full eviction on overflow. Eliminates repeated compilation in the hot policy evaluation path
- **`ProportionalityEvaluator` publicly accessible (MEDIUM)** — Changed module and struct visibility from `pub` to `pub(crate)`. Prevents external crates from accidentally registering the stub evaluator, which would silently approve all proportionality-matched payments

### Added

- 7 new tests: spend_rate currency filtering (2), duplicate_detection currency filtering (2), merchant_check case-insensitive matching for In/NotIn/Equals (3)

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 128/128 passing (28 models + 14 audit + 71 policy + 15 providers) |

---

## 0.6.5 — 2026-04-01

**Phase 6.5: Production Readiness Review**

Comprehensive review targeting encapsulation, silent bypass vectors, test coverage gaps, and schema defense-in-depth. All changes are additive — no reverts of previous hardenings.

### Fixed

- **ProportionalityEvaluator silently passes all payments (CRITICAL)** — Unregistered from `PolicyEngine` evaluator map. The stub struct is retained in `rules/proportionality.rs` for future LLM implementation, but is no longer wired into the engine. Rules referencing `proportionality` will log a warning and be skipped (same as any unknown rule type), rather than silently approving
- **`Payment::status` field is `pub` — bypasses state machine (HIGH)** — Made `status` private. Added `status()` getter. All mutations now must go through `transition()`, which enforces valid state machine moves and updates `updated_at`
- **`NotIn` operator returns `true` on non-array value (HIGH)** — Changed to return `false` with a warning log. Misconfigured deny/allow-list rules now fail safe (restrictive) instead of fail open (permissive)
- **`MerchantCheckEvaluator` has zero test coverage (HIGH)** — Added 7 tests covering `In` (deny-list), `NotIn` (allow-list), `Equals`, and non-array misconfiguration edge cases for both operators

### Added

- `Payment::status()` getter method on models
- 8 new tests: 7 MerchantCheck + 1 NotIn condition evaluator fail-safe
- Migration `20260401200004_add_payments_status_check_and_policy_index`:
  - CHECK constraint on `payments.status` limiting to the 10 valid `PaymentStatus` variants
  - Replaced `idx_policy_rules_profile(profile_id, priority)` with `(profile_id, enabled, priority)` to avoid scanning disabled rules

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 121/121 passing (28 models + 14 audit + 64 policy + 15 providers) |

---

## 0.6.4 — 2026-04-01

**Phase 6.4: Pre-Production Audit**

Systematic review targeting silent-bypass vectors, correctness edge cases, and query safety across `cream-policy` and `cream-audit`. All changes are additive — no reverts of previous hardenings.

### Fixed

- **DuplicateDetection accepts non-positive window (CRITICAL)** — `DuplicateDetectionEvaluator` now validates that `window_minutes > 0`. Negative or zero values created a future cutoff that never matched any payment, silently disabling the rule
- **SpendRate monthly fallback uses arbitrary 30-day window (HIGH)** — Replaced unreachable fallback with `expect()` (day 1 is always valid in chrono). Eliminates misleading dead code and documents the invariant
- **TimeWindow uses fragile string-based hour parsing (HIGH)** — Replaced `format("%H").parse::<u32>()` with chrono's `.hour()` method via `Timelike` trait. Removes the string formatting → parsing roundtrip and the silent `unwrap_or(0)` fallback
- **Geographic evaluator case-sensitive comparison (MEDIUM)** — `GeographicEvaluator` now uses `eq_ignore_ascii_case()` for `CountryCode` comparison. Mixed-case codes (e.g., profile has `"sg"`, request has `"SG"`) no longer cause false rejections
- **Audit query silently falls back on serialization failure (HIGH)** — `serialize_enum_to_string` now returns `Result` and propagates errors instead of silently querying for `"unknown"` / `"other"`. Prevents audit queries from returning wrong results
- **Audit query offset unbounded (MEDIUM)** — `AuditQuery.effective_offset()` now clamps to 100,000 to prevent full-table scan DoS via large pagination offsets

### Added

- 5 new tests: duplicate_detection negative/zero window, spend_rate monthly calendar boundary, geographic case-insensitive comparison, audit offset clamping

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 113/113 passing (28 models + 14 audit + 56 policy + 15 providers) |

---

## 0.6.3 — 2026-04-01

**Phase 6.3: Misconfiguration Guards & Type Consistency**

Pre-production review targeting rule misconfiguration bypass vectors, schema consistency, and type safety gaps. All changes are additive — no reverts of previous hardenings.

### Fixed

- **Velocity limit accepts negative config (HIGH)** — `VelocityLimitEvaluator` now validates that `max_count` and `window_minutes` are positive. Negative or zero values caused inverted time windows that silently bypassed the rule
- **Time window accepts out-of-range hours (MEDIUM)** — `TimeWindowEvaluator` now validates that `allowed_hours_start` and `allowed_hours_end` are in 0–23 range. Out-of-range values caused comparisons that never matched, silently disabling the rule
- **virtual_cards missing `updated_at` (HIGH)** — New migration adds `updated_at TIMESTAMPTZ` column and `set_updated_at()` trigger, aligning with every other mutable table in the schema
- **CountryCode type inconsistency (LOW)** — `Recipient.country` changed from bare `String` to `CountryCode` newtype, matching `AgentProfile.geographic_restrictions`. Evaluator and geographic rule updated accordingly
- **Unused `mockall` dev-dependency (LOW)** — Removed from `cream-providers` Cargo.toml (MockProvider is hand-written)

### Added

- Migration `20260401200003_add_virtual_cards_updated_at`
- 3 new tests: negative velocity config, zero velocity window, out-of-range time window hours

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 108/108 passing (28 models + 13 audit + 52 policy + 15 providers) |

---

## 0.6.2 — 2026-04-01

**Phase 6.2: Production Hardening Review**

Pre-production code quality assessment. Fixed 9 issues (2 critical, 3 high, 3 medium) across `cream-models`, `cream-policy`, `cream-audit`, and database migrations.

### Fixed

- **Spend rate excluded settled payments (CRITICAL)** — `SpendRateEvaluator` used `!is_terminal()` which excluded settled payments from cumulative spend. Agents could bypass daily limits by waiting for settlements. Added `PaymentStatus::counts_toward_spend()` that includes settled + in-flight, excludes failed/blocked/rejected
- **Velocity limit same bug (HIGH)** — `VelocityLimitEvaluator` had identical terminal-status exclusion. Fixed with same `counts_toward_spend()` method
- **Missing DB columns (CRITICAL+HIGH)** — `timezone` on `agent_profiles` and `rule_type` on `policy_rules` existed in domain models but not in schema. New migration adds both
- **Audit query builder fragile binding (HIGH)** — Refactored split-phase clause/bind pattern to co-located `BindValue` enum that prevents ordering mismatches
- **Audit writer missing payment_id (MEDIUM)** — `AuditWriter::append()` now accepts `Option<PaymentId>` parameter
- **Silent "unknown" status fallback (MEDIUM)** — Audit writer now propagates serialization errors instead of silently degrading
- **Missing indexes + CHECK (MEDIUM)** — Added indexes on `provider_health` and `webhook_endpoints`, plus CHECK constraint on webhook status

### Added

- `PaymentStatus::counts_toward_spend()` method on models
- `BindValue` enum in audit query builder for type-safe bind collection
- Migration `20260401200001_add_timezone_and_rule_type`
- Migration `20260401200002_add_missing_indexes`
- 5 new tests: settled/failed spend rate, settled velocity, payment_id writer, counts_toward_spend

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 105/105 passing (28 models + 13 audit + 49 policy + 15 providers) |

---

## 0.6.1 — 2026-04-01

**Phase 6.1: Cross-Crate Quality Review & Hardening**

Comprehensive code quality review of Phases 1–6. Fixed 11 issues (2 critical, 5 high, 3 medium) across `cream-models`, `cream-policy`, and `cream-audit`.

### Fixed

- **TimeWindowEvaluator timezone support** — time windows now evaluate in the agent profile's timezone, not UTC. Supports per-rule `utc_offset_hours` override
- **Explicit rule_type on PolicyRule** — engine uses `rule_type` field directly instead of fragile inference from condition field names (inference kept as fallback)
- **Real regex for Matches operator** — `ComparisonOp::Matches` now uses `regex::Regex` instead of substring matching. Invalid patterns log a warning
- **Audit query builder refactored** — replaced manual `bind_idx` tracking with `QueryBuilder` helper that auto-increments indices
- **Monthly spend uses calendar month** — `SpendRateEvaluator` now computes start of calendar month instead of rolling 30-day window
- **Warnings on misconfigured rules** — velocity_limit, time_window, and duplicate_detection evaluators log when config extraction fails
- **Warnings on stub evaluators** — ProportionalityEvaluator logs warning when invoked
- **Warnings on unresolved condition fields** — unknown field names in conditions log instead of silently resolving to null
- **Warnings on serialization fallbacks** — audit writer/reader log when enum serialization falls back to defaults

### Added

- `timezone: Option<String>` field on `AgentProfile`
- `rule_type: Option<String>` field on `PolicyRule`
- `regex = "1"` workspace dependency
- 8 TimeWindow tests (normal range, overnight, midnight boundary, timezone, offset override)
- 2 regex tests (valid pattern, invalid pattern graceful failure)

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 100/100 passing (27 models + 12 audit + 46 policy + 15 providers) |

---

## 0.6.0 — 2026-04-01

**Phase 6: Provider Crate (`cream-providers`)**

Defines the `PaymentProvider` trait abstraction, `ProviderRegistry` factory, and a configurable `MockProvider` for end-to-end pipeline testing without external services.

### Added

- **`PaymentProvider` trait** (`traits.rs`) — async trait with 6 methods: `initiate_payment`, `issue_virtual_card`, `update_card_controls`, `cancel_card`, `get_transaction_status`, `health_check`
- **`ProviderRegistry`** (`registry.rs`) — `HashMap<ProviderId, Arc<dyn PaymentProvider>>` with register/get/all/provider_ids methods
- **`MockProvider`** (`mock_provider.rs`) — configurable mock with success/failure, latency simulation, custom settlement status, health reporting. Convenience constructors: `success()`, `failing()`
- **Provider types** (`types.rs`) — `NormalizedPaymentRequest`, `ProviderPaymentResponse`, `TransactionStatus`, `CardConfig`
- **`ProviderError`** (`error.rs`) — 7 error variants covering request failures, timeouts, auth, card errors
- **15 unit tests** — registry CRUD, mock provider payment/card/health operations, custom configs

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 90/90 passing (27 models + 12 audit + 36 policy + 15 providers) |

---

## 0.5.0 — 2026-04-01

**Phase 5: Policy Engine Crate (`cream-policy`)**

Implements the declarative rule evaluation engine with 12 built-in rule types. Purely computational — zero database dependencies, single-digit millisecond evaluation.

### Added

- **`PolicyEngine`** (`engine.rs`) — evaluator registry with priority-ordered evaluation. First-block-wins, escalation-accumulates semantics. Returns `PolicyDecision` with full audit trail of rules evaluated and matched
- **`EvaluationContext`** (`context.rs`) — pre-loaded data bag with request, agent, profile, recent payments, known merchants, and injectable current time
- **`RuleEvaluator` trait** (`evaluator.rs`) — trait for rule implementations, plus condition tree walker for `PolicyCondition` (AND/OR/NOT/FieldCheck) with numeric, string, and set comparisons
- **10 fully implemented rule evaluators:**
  - `AmountCapEvaluator` — per-transaction limit from agent profile
  - `VelocityLimitEvaluator` — max N transactions in time window
  - `SpendRateEvaluator` — daily/weekly/monthly cumulative spend caps
  - `CategoryCheckEvaluator` — allowed payment category enforcement
  - `MerchantCheckEvaluator` — merchant allow/deny list via condition tree
  - `GeographicEvaluator` — recipient country restrictions
  - `RailRestrictionEvaluator` — allowed payment rail enforcement (Auto always passes)
  - `JustificationQualityEvaluator` — non-empty + minimum 10 words (LLM check stubbed)
  - `FirstTimeMerchantEvaluator` — escalates unknown merchants
  - `DuplicateDetectionEvaluator` — same amount+recipient within configurable window
- **2 stub evaluators:**
  - `TimeWindowEvaluator` — allowed hours UTC check (fully implemented)
  - `ProportionalityEvaluator` — stub, requires semantic LLM analysis
- **`PolicyError`** (`error.rs`) — unknown rule type and condition errors
- **36 unit tests** — individual rule evaluators, engine priority/block/escalate semantics, condition tree AND/OR/NOT/In/NotIn evaluation

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 75/75 passing (27 models + 12 audit + 36 policy) |

---

## 0.4.0 — 2026-04-01

**Phase 4: Audit Crate (`cream-audit`)**

Implements the append-only write path and query interface for the immutable audit ledger. Trait-based design allows the API crate to depend on abstract interfaces while tests use mocks.

### Added

- **`AuditWriter` trait + `PgAuditWriter`** (`writer.rs`) — insert-only interface backed by PostgreSQL. No update/delete methods exist at the Rust level, mirroring the database trigger enforcement from Phase 3
- **`AuditReader` trait + `PgAuditReader`** (`reader.rs`) — query interface with `query()`, `get_by_id()`, `get_by_payment()`. Dynamic SQL builder with parameterized queries prevents SQL injection
- **`AuditQuery` filter struct** — optional filters for agent_id, date range, status, category, amount range, with pagination (limit clamped to 1000)
- **`AuditError` type** (`error.rs`) — dedicated error enum covering database, serialization, and not-found cases
- **12 unit tests** — AuditRow deserialization roundtrips, query builder limit/offset logic, invalid status handling, mockall trait verification for both writer and reader
- **`async-trait`** added to workspace dependencies

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 39/39 passing (27 models + 12 audit) |

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
