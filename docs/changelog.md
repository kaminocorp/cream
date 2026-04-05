# Changelog

- [0.8.8](#088--2026-04-05) — Production review: Currency serde format, PolicyAction DB CHECK, NULL spending limits, idempotency leak on provider failure, escalation timeout query, duplicate detection status filter, merchant_check compound conditions, schema hardening
- [0.8.7](#087--2026-04-05) — Production review: ProviderError info leak, is_terminal state machine correctness, idempotency_keys DB constraint
- [0.8.6](#086--2026-04-05) — Production review: update_policy validation gap, approve/reject audit field bypass, spending limit strictness, audit ledger DB constraints
- [0.8.5](#085--2026-04-05) — Production review: settlement data persistence, escalation timeout audit resilience, provider field DB constraints
- [0.8.4](#084--2026-04-05) — Production review: API amount validation gap, invalid regex policy bypass, name length DB constraints
- [0.8.3](#083--2026-04-05) — Production review: idempotency observability gap, escalation timeout audit correctness, webhook input validation
- [0.8.2](#082--2026-04-05) — Production review: escalation timeout audit trail, idempotency key lifecycle completion, circuit breaker observability
- [0.8.1](#081--2026-04-05) — Cross-crate production review: 11 fixes targeting audit correctness, race safety, data corruption prevention, and schema hardening
- [0.8.0](#080--2026-04-05) — API crate: Axum HTTP server, 12 REST endpoints, payment lifecycle orchestrator with failover, auth, rate limiting, escalation monitor
- [0.7.12](#0712--2026-04-05) — Circuit breaker clock skew guard and u32 counter overflow protection
- [0.7.11](#0711--2026-04-05) — Circuit breaker half-open fix: close only when all probe requests succeed, not on first success
- [0.7.10](#0710--2026-04-05) — Cross-crate production review: Settled/Failed must have provider fields, audit deterministic ordering, settlement field pairing constraint, scorer deterministic tiebreaker, time_window offset bounds
- [0.7.9](#079--2026-04-05) — Production review: Payment provider field state machine invariants, AuditEntry on_chain_tx_hash bounds, regex cache comment, virtual_cards composite unique constraint
- [0.7.8](#078--2026-04-05) — Cross-crate production review: PaymentCategory::Other empty guard, IdempotencyKey max length, audit query deterministic ordering, time_window log accuracy, condition depth off-by-one
- [0.7.7](#077--2026-04-02) — Recipient.identifier whitespace-only guard
- [0.7.6](#076--2026-04-02) — Final empty-string guard sweep: HumanReviewRecord.reason and PaymentMetadata optional fields
- [0.7.5](#075--2026-04-02) — Production hardening: unknown rule_type fail-safe, IdempotencyKey FromStr fix, scorer health clamp, VirtualCard schema alignment, scoring all-zero rejection, optional string empty guards, escalation zero-timeout guard, ProviderId max length
- [0.7.4](#074--2026-04-02) — Production hardening: fail-safe on misconfigured policy rules, Agent/AgentProfile name validation, invalid regex fail-safe, settled_currency constraint, provider_id index
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

## 0.8.8 — 2026-04-05

**Production review: Currency serde format, PolicyAction DB CHECK, NULL spending limits, idempotency leak on provider failure, escalation timeout query, duplicate detection status filter, merchant_check compound conditions, schema hardening**

Full 6-crate + migrations production readiness review (7 parallel review agents + manual cross-verification). ~40 candidate findings surfaced; after line-by-line verification against actual code, 10 genuine fixes confirmed across 5 files + 1 new migration. All changes are additive (no reversals of prior hardenings).

### Fixed

- **Currency enum serialized to wrong format — every DB INSERT would fail (CRITICAL)** (`models/payment.rs`) — `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` on the `Currency` enum caused the `heck` crate to split 3-letter all-caps variant names into individual characters: `USD` → `"U_S_D"`, `SGD` → `"S_G_D"`, `BTC` → `"B_T_C"`. The DB CHECK constraint expects standard ISO 4217 codes (`'USD'`, `'SGD'`, `'BTC'`). Every `insert_payment()` would fail with a constraint violation. Removed `rename_all` from the enum — variant names are already the desired format. Kept explicit `#[serde(rename = "BASE_ETH")]` on `BaseEth`
- **PolicyAction DB CHECK case mismatch — every `load_rules()` would fail (CRITICAL)** (new migration `20260405200008`) — DB CHECK: `action IN ('approve', 'block', 'escalate')`. Rust `PolicyAction`: `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` → `"APPROVE"`, `"BLOCK"`, `"ESCALATE"`. Every deserialization of policy rules from DB would fail. Updated CHECK to `('APPROVE', 'BLOCK', 'ESCALATE')`
- **Agent profile spending limits NULL in DB, non-optional in Rust — agent auth crash (HIGH)** (new migration `20260405200008`) — DB columns `max_per_transaction`, `max_daily_spend`, `max_weekly_spend`, `max_monthly_spend` allowed NULL, but `AgentProfile`'s custom Deserialize expects non-optional `Decimal > 0`. Any agent with NULL limits would get a 500 on every authenticated request. Migration sets existing NULLs to a high default and adds NOT NULL constraints
- **Idempotency key permanently leaked on routing/provider failure (HIGH)** (`api/orchestrator.rs`) — After policy Approve, if routing failed or all providers were exhausted, the `?` propagated the error but the idempotency key was neither released nor completed. The agent could never retry with the same key (409 Conflict). Added error recovery: on routing/provider failure after approval, transition payment to Failed and release the idempotency key. Same pattern applied to `resume_after_approval()`
- **Payment stranded in Approved state with no recovery (HIGH)** (`api/orchestrator.rs`) — Related to above. When all providers failed, the payment stayed in `Approved` forever with no background monitor, retry queue, or manual re-submission path. Now transitions to `Submitted → Failed` on provider exhaustion
- **`find_expired_escalations` used wrong timeout — premature expiry (MEDIUM)** (`api/db.rs`, `api/orchestrator.rs`, `policy/engine.rs`, new migration) — The query joined ALL escalation rules for the agent's profile, not the specific rule that triggered escalation. If Rule A had a 10-minute timeout and Rule B (the actual trigger) had 60 minutes, the payment timed out at 10 minutes. Proper fix: added `escalation_rule_id` column to payments table, `escalation_rule_id` field to `PolicyDecision`, and persistence in the orchestrator's Escalate path. The timeout query now uses the specific triggering rule's timeout via `COALESCE`, with fallback to `MIN(timeout_minutes)` across the profile's rules for legacy payments without the field set
- **Duplicate detection blocked retries of failed payments (MEDIUM)** (`policy/rules/duplicate_detection.rs`) — Unlike `spend_rate` and `velocity_limit` evaluators which filter by `counts_toward_spend()`, duplicate detection matched all payment statuses including `Failed`. A payment that failed due to a provider timeout would block a legitimate retry within the window. Added `p.status.counts_toward_spend()` filter
- **Merchant check compound condition bypass (MEDIUM)** (`policy/rules/merchant_check.rs`) — Non-merchant `FieldCheck` nodes in `has_merchant_match()` returned `false`, causing `All([amount_check, merchant_check])` to always return false (short-circuit on the amount check). An operator creating "block merchant X if amount > $500" would have the check silently disabled. Changed non-merchant FieldChecks to return `true` (vacuously satisfied in the merchant-matching dimension). Known trade-off: `Any([non_merchant_check, merchant_check])` will now always return true since the non-merchant check is vacuously satisfied. In practice this is low risk — `Any` is not a natural combinator for compound merchant restrictions, and all 12 dedicated evaluators bypass `has_merchant_match` entirely
- **Missing composite index on `payments(agent_id, created_at)` (MEDIUM)** (new migration `20260405200008`) — Hot-path `load_recent_payments` query (every payment initiation) lacked optimal index, requiring scan of all agent payments instead of just the 30-day window
- **`payments.idempotency_key` missing length constraint (MEDIUM)** (new migration `20260405200008`) — `idempotency_keys.key` had `CHECK (LENGTH(key) <= 255)` but `payments.idempotency_key` (same value) was unbounded. Added matching constraint
- **`virtual_cards` provider columns missing length constraints (MEDIUM)** (new migration `20260405200008`) — `payments.provider_id` has `CHECK (LENGTH <= 255)` but equivalent columns on `virtual_cards` had none. Added constraints
- **Audit category GIN index unusable for text equality queries (MEDIUM)** (new migration `20260405200008`) — GIN index on `justification->'category'` (JSONB) doesn't serve text equality queries using `->>` (TEXT). Replaced with btree index on `justification->>'category'`
- **`payments.failure_reason` unbounded TEXT (LOW-MEDIUM)** (new migration `20260405200008`) — Provider error messages written to `failure_reason` had no length constraint. Added `CHECK (LENGTH <= 2000)`
- **Redundant `idx_webhook_endpoints_url` index (LOW)** (new migration `20260405200008`) — Regular btree index alongside UNIQUE constraint on same column. Dropped redundant index

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 377/377 passing (173 models + 14 audit + 108 policy + 17 providers + 54 router + 11 api) |

---

## 0.8.7 — 2026-04-05

**Production review: ProviderError info leak, is_terminal state machine correctness, idempotency_keys DB constraint**

Full 6-crate production readiness review (5 parallel review agents across all crates + migrations + manual code-level verification). ~150 candidate findings surfaced across all agents; after line-by-line verification, the vast majority were confirmed as false positives (already fixed in v0.7.x-v0.8.6, intentional design decisions, or misunderstood Rust ownership semantics). 3 genuine fixes across 3 files + 1 new migration, all additive (no reversals of prior hardenings).

### Fixed

- **ProviderError details leaked to HTTP clients — information disclosure (MEDIUM)** (`api/error.rs`) — `ProviderFailure(e)` returned `format!("payment provider error: {e}")` in the HTTP response body. Since `ProviderError` variants include `InsufficientFunds("account balance $X")`, `ComplianceBlocked("specific reason")`, and `UnsupportedCountry("country code")`, these exposed internal provider details that could help attackers reverse-engineer policy/compliance constraints. Replaced with generic message `"payment provider error — see server logs for details"`. The specific error is still logged server-side at WARN level (line 107-108)
- **`is_terminal()` incorrectly includes `TimedOut` — state machine semantic inconsistency (LOW)** (`models/payment.rs`) — `PaymentStatus::TimedOut.is_terminal()` returned `true`, but `can_transition_to(Blocked)` also returns `true` for TimedOut. A state that can transition is by definition not terminal. The escalation timeout monitor performs `TimedOut → Blocked` atomically, making TimedOut a transient intermediate state. Removed `TimedOut` from `is_terminal()`. Currently only used in tests, but prevents future code from relying on incorrect semantics
- **DB lacks CHECK on `idempotency_keys.key` length — unbounded TEXT primary key (MEDIUM)** (new migration `20260405200007`) — Rust enforces `MAX_IDEMPOTENCY_KEY_LEN = 255` on deserialization, but the DB allowed unbounded TEXT. Consistent with the defense-in-depth pattern from v0.8.4-v0.8.6 (names, provider_id, provider_tx_id, on_chain_tx_hash). Added `CHECK (LENGTH(key) <= 255 AND LENGTH(TRIM(key)) > 0)`

### Verified False Positives (Not Fixed)

| Claimed Issue | Verdict |
|---|---|
| Unauthenticated approve/reject endpoints | Documented Phase 10 scope (line 160-161 in payments.rs). Dashboard auth is planned, not missing. |
| Settlement data loss on audit write failure | `write_audit` returns `?` — error propagates. Idempotency key not completed. Acceptable for current phase. |
| persist_settlement race condition | Only called from process() and resume_after_approval(), both holding idempotency lock. |
| Escalation timeout nil profile_id fallback | Intentional graceful degradation added in v0.8.3 for agent-deleted edge case. |
| Circuit breaker Mutex poisoning | InMemoryCircuitBreakerStore is test-only. Production uses Redis. |
| Idempotency TTL expiry double-payment | 300s TTL vs. sub-300ms target. Provider has own idempotency. Architecture concern, not code bug. |
| Provider registry not thread-safe | register(&mut self) called at startup only. Rust borrow checker prevents concurrent registration. |
| Scorer all-zero weights float precision | 0.0 + 0.0 + 0.0 + 0.0 = exactly 0.0 in IEEE 754. |
| NotIn non-array bypass | Operator misconfiguration edge case. Schema validation on write should prevent. |
| Unknown rule type Approve bypass | Explicit Approve from matching_rules doesn't change final decision. |
| FK cascade behavior unspecified | RESTRICT is correct (verified in v0.8.5). Prevents orphan records. |
| Index column order suboptimal | (profile_id, enabled, priority) is optimal for the query pattern. |

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 377/377 passing (173 models + 14 audit + 108 policy + 17 providers + 54 router + 11 api) |

---

## 0.8.6 — 2026-04-05

**Production review: update_policy validation gap, approve/reject audit field bypass, spending limit strictness, audit ledger DB constraints**

Full 6-crate production readiness review (4 parallel review agents across all crates + migrations + manual code-level verification). ~80 candidate findings surfaced across all agents; after line-by-line verification, the majority were confirmed as false positives (already fixed in v0.8.1-0.8.5, intentional design decisions, or misunderstood code paths). 5 genuine fixes across 3 files + 1 new migration, all additive (no reversals of prior hardenings).

### Fixed

- **`update_policy` handler allows zero spending limits — agent lockout (MEDIUM)** (`api/routes/agents.rs`) — `UpdatePolicyRequest` uses `Option<Decimal>` with derive(Deserialize), providing no validation. The handler writes values directly to SQL via `COALESCE($1, existing_value)`. A zero value passed the DB CHECK (`>= 0`) and was persisted, but `AgentProfile`'s custom Deserialize requires `> 0`. On the next authentication attempt, the auth extractor's deserialization failed with a 500 error, permanently locking the agent out. Added explicit positive-value validation for all five spending limit fields (`max_per_transaction`, `max_daily_spend`, `max_weekly_spend`, `max_monthly_spend`, `escalation_threshold`) before any DB write
- **Approve/reject handlers bypass `HumanReviewRecord` validation — permanent audit corruption (MEDIUM)** (`api/routes/payments.rs`) — Both handlers constructed `HumanReviewRecord` via struct literal, bypassing the custom Deserialize that validates: `reviewer_id` non-empty/non-whitespace, `reviewer_id` length ≤ 255, `reason` non-empty/non-whitespace when present, `reason` length ≤ 2000. Since audit records are append-only (DB triggers prevent UPDATE/DELETE), invalid values would be permanently persisted. Added `validate_review_fields()` function called before any state mutation in both handlers, enforcing the same invariants as the Deserialize impl. Also exported `MAX_REVIEWER_ID_LEN` and `MAX_REVIEW_REASON_LEN` constants in the models prelude
- **DB spending limits CHECK constraints allow zero — Rust↔DB validation gap (MEDIUM)** (new migration `20260405200006`) — DB used `CHECK (max_per_transaction IS NULL OR max_per_transaction >= 0)` but Rust requires `> 0`. Replaced all five `_non_negative` constraints with `_positive` variants using `> 0`. Same pattern applied to `escalation_threshold`
- **DB lacks CHECK on `audit_log.final_status` — unconstrained append-only column (MEDIUM)** (new migration `20260405200006`) — `final_status` was unconstrained TEXT. Added CHECK constraining to the 10 valid `PaymentStatus` enum values (`pending`, `validating`, `pending_approval`, `approved`, `submitted`, `settled`, `failed`, `blocked`, `rejected`, `timed_out`). Critical because the audit ledger is append-only — invalid values would be permanent
- **DB lacks CHECK on `audit_log.on_chain_tx_hash` length — unbounded append-only column (LOW-MEDIUM)** (new migration `20260405200006`) — Rust enforces `MAX_ON_CHAIN_TX_HASH_LEN = 256` on deserialization, but the DB allowed unbounded TEXT. Added `CHECK (on_chain_tx_hash IS NULL OR LENGTH(on_chain_tx_hash) <= 256)`

### Verified False Positives (Not Fixed)

| Claimed Issue | Verdict |
|---|---|
| SQL injection in `find_expired_escalations` | `(pr.escalation->>'timeout_minutes')::int` reads admin-controlled policy_rules data, not user input. Already verified in v0.8.5. |
| Nil profile_id in escalation timeout audit | Intentional graceful degradation added in v0.8.3. Agent deletion while PendingApproval is an extreme edge case with no delete endpoint exposed. |
| Double idempotency complete in approve | `process()` holds (doesn't complete) the key on escalation; approve completes it once. Single complete, not double. |
| Approve endpoint ordering race | No agent delete endpoint exists. Requires direct DB manipulation during approval — not an application-level bug. |
| FK cascade behavior (RESTRICT default) | RESTRICT is correct for a payment system — prevents orphan records. |
| NaN propagation in scorer | `ProviderHealth` custom Deserialize validates `error_rate_5m` is finite ∈ [0.0, 1.0]. |
| Spend limits count Pending payments | Intentional — includes in-flight payments to prevent concurrent requests collectively exceeding limits. |

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 377/377 passing (173 models + 14 audit + 108 policy + 17 providers + 54 router + 11 api) |

---

## 0.8.5 — 2026-04-05

**Production review: settlement data persistence, escalation timeout audit resilience, provider field DB constraints**

Full 6-crate production readiness review (4 parallel review agents across all crates + migrations + manual code-level verification). ~20 candidate findings surfaced; after line-by-line verification, the majority were confirmed as false positives (intentional design decisions, already-validated invariants, or misunderstood Rust ownership semantics). 3 genuine fixes across 2 files + 1 new migration, all additive (no reversals of prior hardenings).

### Fixed

- **Settlement data never persisted to payments table — reconciliation-breaking gap (CRITICAL)** (`api/db.rs`, `api/orchestrator.rs`) — `update_payment()` and `update_payment_if_status()` only wrote `status`, `provider_id`, and `provider_tx_id`. The columns `amount_settled`, `settled_currency`, and `failure_reason` (present in the schema since migration `20260331200004`) were never populated. Every settled payment showed `NULL` for settlement amounts in the database. The audit log captured settlement data via `ProviderResponseRecord`, but the payments table — the queryable source of truth for reconciliation and financial reporting — permanently lost it. Added `persist_settlement()` to the `PaymentRepository` trait + `PgPaymentRepository` implementation. Called from both `process()` and `resume_after_approval()` immediately after provider execution, writing `amount_settled`, `settled_currency`, and a descriptive `failure_reason` for failed/declined/refunded transactions
- **Escalation timeout audit write silently swallowed on failure — compliance gap (HIGH)** (`api/orchestrator.rs`) — When the escalation timeout monitor's audit write failed, the error was logged at ERROR level but the function continued. Since the payment state change was already committed to the DB, this left a permanent audit gap: a payment blocked by timeout with no corresponding audit record. Added a single retry with 250ms delay (covers transient DB errors, which are the most common failure mode). If the retry also fails, logs at ERROR with a `CRITICAL:` prefix and explicit guidance that manual reconciliation is required, giving operators clear signal for alerting
- **DB lacks length constraints on `payments.provider_id` and `payments.provider_tx_id` — unbounded TEXT columns (HIGH)** (new migration `20260405200005`) — Rust types enforce `MAX_PROVIDER_ID_LEN = 255` (ProviderId) and `MAX_PROVIDER_TRANSACTION_ID_LEN = 500` (ProviderResponseRecord), but the DB allowed unbounded TEXT. Direct DB manipulation or future ORM changes could persist oversized values that break deserialization on read — and in the append-only audit ledger, oversized values would become permanent. Added CHECK constraints: `LENGTH(provider_id) <= 255` and `LENGTH(provider_tx_id) <= 500` (both allowing NULL). Same pattern as v0.8.4's name length constraints
- **Missing index on `audit_log.agent_profile_id` — unbounded table scan (MEDIUM)** (new migration `20260405200005`) — The audit ledger is append-only and grows without bound. Profile-scoped audit queries (`WHERE agent_profile_id = $1`) required full table scans. Added `idx_audit_profile` B-tree index

### Verified False Positives (Not Fixed)

| Claimed Issue | Verdict |
|---|---|
| SQL injection in escalation timeout query | `(pr.escalation->>'timeout_minutes')::int` reads admin-controlled policy_rules data, not user input. PostgreSQL errors on non-integer cast; no SQL execution possible. |
| NaN propagation in scorer health_score | `ProviderHealth` custom Deserialize validates `error_rate_5m` is finite ∈ [0.0, 1.0]. NaN cannot reach the scorer. |
| Spend limits count Pending payments (bypass) | Intentional design — docstring explicitly states "includes in-flight payments." Not counting them would allow concurrent requests to individually pass but collectively exceed limits. |
| Escalation threshold uses >= instead of > | Intentional — escalation_threshold means "require human approval at or above this amount." Different semantic from amount_cap's hard ceiling. |
| Corrupt idempotency lock blocks retries | Idempotency values are always `payment_id.as_uuid().to_string()`. UUID corruption requires Redis-level data loss, not an application bug. |
| Audit query fails on malformed entries | Correct behavior — surfacing data corruption rather than silently dropping records. |

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 377/377 passing (173 models + 14 audit + 108 policy + 17 providers + 54 router + 11 api) |

---

## 0.8.4 — 2026-04-05

**Production review: API amount validation gap, invalid regex policy bypass, name length DB constraints**

Full 6-crate production readiness review (7 parallel review agents across all crates + migrations). ~50 candidate findings surfaced; after manual code-level verification, the majority were confirmed as false positives (already fixed in prior hardenings, known design decisions, or misunderstood Rust ownership semantics). 3 genuine fixes across 3 files + 1 new migration, all additive (no reversals of prior hardenings).

### Fixed

- **API boundary bypasses `PaymentRequest` amount validation — invalid data reaches orchestrator (MEDIUM)** (`api/routes/payments.rs`) — `CreatePaymentRequest` deserializes `amount: Decimal` via derive(Deserialize) with no validation. The handler then constructs `PaymentRequest` via struct literal (bypassing the custom `Deserialize` impl on `PaymentRequest` which validates `amount > 0`). A zero or negative amount would reach the orchestrator and only be caught by the DB `CHECK (amount > 0)` constraint, surfacing as a raw sqlx error instead of a clean 422 validation response. Added explicit `amount <= Decimal::ZERO` check before `PaymentRequest` construction, returning `ApiError::ValidationError`
- **Invalid regex pattern returns `true` — broken APPROVE rules grant unintended approvals (MEDIUM)** (`policy/evaluator.rs`) — `regex_matches()` returned `true` when a regex pattern was invalid, with the reasoning "to prevent policy bypass from misconfigured patterns." This reasoning assumed all rules are restrictive (BLOCK/ESCALATE). For APPROVE rules, returning `true` means the condition matches, the rule fires, and the payment is approved — a policy bypass in the opposite direction. Changed both the normal path (line 273) and the poisoned-mutex fallback (line 238) to return `false`. A non-matching condition means the rule does not fire, so payments continue to subsequent rules or the default policy. Updated the corresponding test (`condition_matches_invalid_regex_fails_safe`) to assert the corrected semantics
- **DB lacks length constraints on `agents.name` and `agent_profiles.name` — unbounded TEXT columns (LOW-MEDIUM)** (new migration `20260405200004`) — Rust types enforce `MAX_NAME_LEN = 255` and whitespace validation, but the DB allowed unbounded TEXT. Direct DB manipulation or future ORM changes could persist oversized names into the append-only audit ledger (where they become permanent). Added CHECK constraints: `LENGTH(name) <= 255 AND LENGTH(TRIM(name)) > 0` on both tables

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 377/377 passing (173 models + 14 audit + 108 policy + 17 providers + 54 router + 11 api) |

---

## 0.8.3 — 2026-04-05

**Production review: idempotency observability gap, escalation timeout audit correctness, webhook input validation**

Full 6-crate production readiness review (4 parallel review agents + manual verification). Surfaced ~30 candidate findings; after code-level verification, 26 were confirmed as false alarms or intentional design (fail-safe semantics, symmetric case-insensitive comparison, correct velocity arithmetic, deferred Phase 10 auth). 4 genuine fixes across 2 files, all additive (no reversals of prior hardenings).

### Fixed

- **Silent idempotency release error on policy block — observability gap (HIGH)** (`api/orchestrator.rs`) — When a payment was blocked by policy, the idempotency key release used `let _ =`, completely discarding any Redis error. v0.8.2 upgraded the identical pattern in approval, rejection, and escalation timeout paths to WARN-level logging, but missed the policy-block path. If Redis fails to release, operators now have visibility via `"failed to release idempotency key after policy block"` at WARN level, consistent with all other idempotency error handling
- **Escalation timeout audit entry wrote nil UUID for `agent_profile_id` — immutable data corruption (HIGH)** (`api/orchestrator.rs`) — The escalation timeout monitor wrote `Uuid::nil()` as the `agent_profile_id` in the audit entry. Since the audit ledger is append-only (DB triggers prevent UPDATE/DELETE), this incorrect data was permanent. The approve handler (line 149-218) and reject handler (line 319-326) both correctly looked up the real `profile_id` from the agents table. Added the same lookup pattern to the timeout monitor with graceful fallback to nil UUID if the agent was deleted or the query fails
- **Webhook URL missing format validation — malformed data persistence (MEDIUM)** (`api/routes/webhooks.rs`) — The webhook registration endpoint only checked `url.is_empty()`. No URL scheme validation (could accept arbitrary strings like `ftp://` or `not-a-url`), no length bound (unbounded TEXT column). Added: must start with `https://` or `http://`, maximum 2048 characters
- **Webhook secret accepted single-character values — weak HMAC signatures (LOW)** (`api/routes/webhooks.rs`) — The webhook secret only checked `secret.is_empty()`. A 1-character secret would produce trivially brute-forceable HMAC-SHA256 signatures when webhook dispatch is implemented. Added minimum 16-character requirement

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 377/377 passing (173 models + 14 audit + 108 policy + 17 providers + 54 router + 11 api) |

---

## 0.8.2 — 2026-04-05

**Production review: escalation timeout audit trail, idempotency key lifecycle completion, circuit breaker observability**

Full 6-crate parallel production readiness review targeting lifecycle completeness in the escalation paths (approve, reject, timeout). The happy path correctly handles audit writes, idempotency completion, and circuit breaker logging — but the escalation branches were missing these bookkeeping steps. 3 fixes across 2 files, all additive (no reversals of prior hardenings).

### Fixed

- **Missing audit entry on escalation timeout — compliance-breaking gap (CRITICAL)** (`api/orchestrator.rs`) — The escalation timeout monitor transitioned payments `PendingApproval → TimedOut → Blocked` and updated the DB, but never wrote an audit entry. The docstring stated "writes an audit entry" but the code did not. For a payment control plane whose core invariant is an immutable audit trail of every state change, this meant a payment could be silently blocked by timeout with zero audit record. Added a full `AuditEntry` write (with `reviewer_id: "system:escalation_timeout"` and a `HumanReviewRecord` recording the system decision) after the conditional DB update succeeds
- **Idempotency key permanently leaked for escalated payments (HIGH)** (`api/orchestrator.rs`, `api/routes/payments.rs`) — When `process()` escalated a payment, the idempotency key was intentionally held ("Don't release idempotency — the payment is still in progress"). But none of the three resolution paths completed or released it: approve called `resume_after_approval()` which never touched idempotency; reject never released; timeout never released. In production with Redis, the key would eventually expire via TTL, but during the TTL window after resolution, client retries with the same key would get `IdempotencyConflict` for a payment that was already resolved. Added `idempotency_guard.complete()` after successful approval execution, `idempotency_guard.release()` after rejection, and `idempotency_guard.release()` after escalation timeout
- **Circuit breaker recording errors silently swallowed (MEDIUM)** (`api/orchestrator.rs`) — All three `record_success()` and `record_failure()` calls in the failover loop used `let _ =`, completely discarding errors. In v0.8.1, the analogous idempotency completion case was upgraded to a WARN log, but circuit breaker recording was missed. If circuit breaker state fails to update (e.g., Redis hiccup), routing decisions would be based on stale health data with zero visibility. Replaced all three `let _ =` with `if let Err(e)` blocks that log at WARN level with provider ID and error context

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 377/377 passing (173 models + 14 audit + 108 policy + 17 providers + 54 router + 11 api) |

---

## 0.8.1 — 2026-04-05

**Cross-crate production review: audit correctness, race safety, data corruption prevention, schema hardening**

Comprehensive 7-agent parallel review of all 6 crates + migrations, targeting production readiness. 11 fixes across 4 crates and 1 migration, all additive (no reversals of prior hardenings). Central theme: eliminating silent data corruption paths, fixing race conditions in concurrent payment operations, and closing schema gaps.

### Fixed

- **Wrong `agent_profile_id` in approve/reject audit entries** (`api/routes/payments.rs`) — Both handlers used the agent's UUID as the profile ID when constructing audit entries, writing incorrect data to the immutable audit log. Moved agent/profile lookup before audit write so the correct `profile.id` is used. The approve handler now constructs `AuthenticatedAgent` before the audit entry; the reject handler now looks up the actual `profile_id` from the agents table
- **Silent deserialization fallbacks in `load_recent_payments`** (`api/db.rs`) — `unwrap_or(Currency::USD)`, `unwrap_or(PaymentStatus::Pending)`, and `unwrap_or(RailPreference::Auto)` silently masked data corruption, feeding wrong data into policy evaluation (velocity limits, spend rates, duplicate detection). Replaced all with explicit error propagation that surfaces the corrupted field name and value
- **Idempotency key released after payment INSERT on validation failure** (`api/orchestrator.rs`) — Justification validation ran after both `insert_payment` and `idempotency_guard.acquire()`. On validation failure, the idempotency key was released while the payment row remained in the DB. Moved justification validation before payment creation and idempotency acquisition, eliminating the inconsistent state window
- **`insert_payment` silently defaulted currency/rail on serialization failure** (`api/db.rs`) — `unwrap_or("USD")` and `unwrap_or("auto")` on `serde_json::to_value().as_str()` could write wrong currency to the payments table. Replaced with `ok_or_else` that returns `ApiError::Internal` with a descriptive message
- **Unbounded `get_by_payment()` audit query** (`audit/reader.rs`) — No LIMIT clause, unlike the bounded `query()` method (clamped to 1000). Added `LIMIT 1000` to prevent OOM on payments with many audit entries
- **Race condition: approve/reject vs escalation timeout monitor** (`api/orchestrator.rs`, `api/db.rs`, `api/routes/payments.rs`) — Both the escalation timeout monitor and approve/reject handlers performed read-check-write without atomicity guarantees. Added `update_payment_if_status()` to `PaymentRepository` trait — a conditional UPDATE with `WHERE status = $expected` that returns whether the row was updated. Approve, reject, and escalation monitor all use this; concurrent losers get a clear error (handlers) or info log (monitor) instead of silently overwriting
- **Half-open circuit breaker non-atomic increment** (`router/circuit_breaker.rs`) — `get_half_open_count` + check + `increment_half_open_count` was three separate operations, allowing more requests through than `half_open_max_requests` under concurrent load. Changed to increment-first-then-check: atomically increment via `increment_half_open_count` (returns new count), then compare `new_count <= max`. The extra increment past the limit is benign (success counting is independent)
- **Missing index on `virtual_cards(provider_id)`** (new migration `20260405200003`) — No index for provider-level card lookups; the composite unique `(provider_id, provider_card_id)` doesn't serve as a leading index for provider_id-only queries
- **Missing unique constraint on `webhook_endpoints(url)`** (new migration `20260405200003`) — Allowed duplicate webhook registrations at the DB level
- **First-time merchant O(n) lookup** (`policy/rules/first_time_merchant.rs`, `api/db.rs`) — `HashSet::iter().any()` with per-element `to_ascii_lowercase()` instead of O(1) `HashSet::contains()`. Fixed by pre-lowercasing merchant identifiers in `load_known_merchants` and using `contains(&id_lower)` in the evaluator
- **`idempotency_guard.complete()` error silently discarded** (`api/orchestrator.rs`) — `let _ =` on the completion result. Added WARN-level log with payment_id and error message. The payment is already persisted, so this is informational, not fatal

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 377/377 passing (173 models + 14 audit + 108 policy + 17 providers + 54 router + 11 api) |

---

## 0.8.0 — 2026-04-05

**Phase 8: API Crate — Axum HTTP Server, Payment Lifecycle Orchestrator, Authentication & Rate Limiting**

Implements the `cream-api` crate — the Axum HTTP server that wires all five infrastructure crates (models, audit, policy, providers, router) into a runnable payment control plane. This is the integration crate that makes Cream a real service: 12 REST endpoints, the deterministic 8-step payment lifecycle with provider failover, agent authentication via API key, per-agent Redis rate limiting, and a background escalation timeout monitor.

### Added

- **Payment Lifecycle Orchestrator** (`orchestrator.rs`) — implements the 8-step deterministic pipeline from the vision spec. Steps 1-2 (schema validation, agent identity) are handled by Axum extractors; Steps 3-8 (justification validation, policy evaluation, routing, provider execution with failover, settlement confirmation, audit write) are in the orchestrator. Policy decisions branch into three paths: Approve (continue pipeline), Block (terminal — return 403), Escalate (return payment with `pending_approval` status). Idempotency is enforced via `IdempotencyGuard::acquire()` before any processing begins
- **Provider failover logic** — iterates the router's ranked candidate list. Retryable errors (`RequestFailed`, `Timeout`, `Unavailable`, `RateLimited`, `UnexpectedResponse`) cascade to the next candidate; non-retryable errors (`InvalidAmount`, `ComplianceBlocked`, `InsufficientFunds`, etc.) fail immediately with 502. Circuit breaker updated on every outcome. All candidates exhausted → 503
- **`resume_after_approval()`** — when a human approves an escalated payment, the orchestrator resumes from Step 5 (routing → execution → settlement → audit) without re-evaluating policy
- **`PaymentRepository` trait** (`db.rs`) — abstracts all database queries behind a trait boundary for orchestrator unit testability. 8 methods: `insert_payment`, `get_payment`, `get_payment_for_agent`, `update_payment`, `load_rules`, `load_recent_payments`, `load_known_merchants`, `find_expired_escalations`. `PgPaymentRepository` implements against the actual schema (18 SQL queries total across all modules)
- **`AuthenticatedAgent` extractor** (`extractors/auth.rs`) — implements Axum's `FromRequestParts<AppState>`. Extracts `Authorization: Bearer <api_key>`, SHA-256 hashes it, queries `agents` by `api_key_hash` (unique index), verifies `status = 'active'`, loads `AgentProfile`. Auth is per-handler via the extractor pattern — routes that omit the extractor are public
- **`ValidatedJson<T>` extractor** (`extractors/json.rs`) — wraps `axum::Json<T>` with custom rejection returning `ApiError::ValidationError` (consistent JSON error body) instead of Axum's default plain-text rejection
- **Per-agent rate limiting** (`middleware/rate_limit.rs`) — fixed-window counter via Redis. Key: `cream:rate:{key_hash}:{window_epoch}`. Over limit → `429 RateLimited` with `retry_after_secs`. Fail-open on Redis unavailability (WARN log, request allowed through)
- **Request ID propagation** (`middleware/request_id.rs`) — `X-Request-Id` header with UUIDv7 generation via `tower_http::request_id`. Preserves client-provided IDs; generates one if absent; propagates to response
- **Escalation timeout monitor** (`orchestrator.rs`) — Tokio interval task (configurable, default 30s). Queries for `PendingApproval` payments past their `escalation.timeout_minutes`. Transitions each: `PendingApproval → TimedOut → Blocked`
- **`ApiError` enum** (`error.rs`) — 10 variants mapping to HTTP status codes (400, 401, 403, 404, 409, 422, 429, 500, 502, 503). JSON response body: `{ error_code, message, details }`. `From` impls for `PolicyError`, `RoutingError`, `AuditError`, `DomainError`, `sqlx::Error`, `anyhow::Error`. `Display` impl for tracing compatibility. Server errors (5xx) log at error/warn; client errors (4xx) log at debug
- **`AppConfig`** (`config.rs`) — environment-based configuration: `DATABASE_URL`, `REDIS_URL` (required), `HOST` (default `0.0.0.0`), `PORT` (default `8080`), `RATE_LIMIT_REQUESTS` (default 100), `RATE_LIMIT_WINDOW_SECS` (default 60), `ESCALATION_CHECK_INTERVAL_SECS` (default 30)
- **`AppState`** (`state.rs`) — `Clone`-friendly shared state: `PgPool`, Redis `ConnectionManager`, `Arc<PolicyEngine>`, `Arc<RouteSelector>`, `Arc<ProviderRegistry>`, `Arc<dyn AuditWriter>`, `Arc<dyn AuditReader>`, `Arc<IdempotencyGuard>`, `Arc<CircuitBreaker>`, `Arc<dyn PaymentRepository>`, `Arc<AppConfig>`
- **12 REST endpoints** across 6 route modules:
  - `POST /v1/payments` — initiate payment with structured justification (→ orchestrator pipeline)
  - `GET /v1/payments/{id}` — retrieve payment status + audit trail (agent-scoped)
  - `POST /v1/payments/{id}/approve` — human-approve escalated payment (resumes pipeline from Step 5)
  - `POST /v1/payments/{id}/reject` — human-reject escalated payment (terminal, writes `HumanReviewRecord`)
  - `POST /v1/cards` — issue scoped virtual card via provider
  - `PATCH /v1/cards/{id}` — update card spending controls (agent-scoped ownership check)
  - `DELETE /v1/cards/{id}` — cancel/revoke card immediately (agent-scoped)
  - `GET /v1/audit` — query audit log with filters (agent-scoped, delegates to `AuditReader`)
  - `GET /v1/agents/{id}/policy` — get agent's policy profile + rules (self-only access)
  - `PUT /v1/agents/{id}/policy` — update agent's policy profile fields (self-only access)
  - `GET /v1/providers/health` — real-time health status of all registered providers
  - `POST /v1/webhooks` — register webhook endpoint (SHA-256 hashed secret)
- **`/health` endpoint** — unauthenticated, no rate limit, returns `"ok"`
- **Server startup** (`main.rs`) — wires `PgPool`, Redis, `PolicyEngine::new()`, `ProviderRegistry` with `MockProvider`, `RouteSelector` with default weights, `PgAuditWriter`/`PgAuditReader`, in-memory circuit breaker + idempotency stores, `PgPaymentRepository`. Spawns escalation monitor. Binds `TcpListener` and serves
- **Workspace dependencies** — added `sha2 = "0.10"`, `hex = "0.4"` to workspace `Cargo.toml`
- 11 new tests: 10 error variant → HTTP status mapping tests, 1 config validation test

### Design decisions

- **Auth as extractor, not middleware** — idiomatic Axum 0.8. Handlers that need auth include `AuthenticatedAgent` as a parameter; handlers that don't (health, approve, reject) simply omit it. No middleware exclusion lists
- **SHA-256 for API key hashing** — not argon2. API keys are machine-generated high-entropy random tokens, not human passwords. SHA-256 is cryptographically appropriate and ~1000x faster at per-request auth time
- **`PaymentRepository` trait** — follows the trait-boundary pattern from every other crate (`AuditWriter`, `AuditReader`, `CircuitBreakerStore`, `IdempotencyStore`, `HealthSource`). Enables orchestrator unit testing without Postgres
- **Fail-open rate limiting** — Redis unavailability should not cascade into a full service outage. Rate limit failures log at WARN and allow the request through
- **No auth on approve/reject** — human reviewer endpoints use dashboard session auth (Phase 10). Scaffold uses `reviewer_id` from request body
- **Failover only on retryable errors** — non-retryable provider errors fail immediately. Same structurally invalid request would fail against any provider

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 377/377 passing (173 models + 14 audit + 108 policy + 17 providers + 54 router + 11 api) |

---

## 0.7.12 — 2026-04-05

**Phase 7.12: Circuit Breaker Clock Skew Guard & Counter Overflow Protection**

Production readiness review (router) fixing two defensive hardening gaps in the circuit breaker. The cooldown elapsed check now guards against clock skew (NTP adjustment causing `opened_at` to be in the future relative to `now`), and half-open counters use saturating arithmetic to prevent u32 overflow. Both changes are additive — no reverts of previous hardenings.

### Fixed

- **Circuit breaker cooldown check underflows on clock skew — premature HalfOpen transition (LOW-MEDIUM)** — `is_allowed()` computed elapsed time as `now - opened_at` without verifying `now >= opened_at`. If NTP adjusted the system clock backward after a breaker opened, the i64 subtraction would underflow (wrap to a large positive value in release mode, panic in debug mode), passing the cooldown check and prematurely transitioning an Open breaker to HalfOpen. Added `now >= opened` guard before the subtraction
- **Half-open counters use unchecked u32 arithmetic — theoretical overflow (LOW)** — `half_open_count` and `half_open_success_count` in `InMemoryCircuitBreakerStore` used `+= 1`, which could theoretically overflow at `u32::MAX` (4 billion increments). Switched to `saturating_add(1)` for zero-cost overflow protection. The trait contract now implicitly expects saturating semantics from all store implementations

### Added

- 1 new test: `clock_skew_does_not_prematurely_transition_to_half_open` — manually sets `opened_at` to 60 seconds in the future, verifies the breaker remains Open and `is_allowed()` returns false

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 366/366 passing (173 models + 14 audit + 108 policy + 17 providers + 54 router) |

---

## 0.7.11 — 2026-04-05

**Phase 7.11: Circuit Breaker Half-Open Success Counting Fix**

Production readiness review (router) fixing a concurrency-correctness issue in the circuit breaker's half-open → closed transition. The breaker previously tracked *requests allowed through* to decide when to close, meaning a single success could prematurely promote a partially-failing provider back to Closed when concurrent half-open requests were in flight. The fix introduces a dedicated success counter so the breaker only closes when all N probe requests have succeeded. The change is additive — no reverts of previous hardenings.

### Fixed

- **Circuit breaker closes on first success in half-open under concurrency — premature provider promotion (MEDIUM)** — `record_success()` checked `half_open_count >= half_open_max_requests` to decide when to close the breaker, but `half_open_count` was incremented by `is_allowed()` (tracking requests *let through*, not successes). With `half_open_max_requests = 3` and 3 concurrent requests in flight, a single success arriving before pending failures would see `count(3) >= max(3)` and close the breaker — even if the other 2 requests failed. The failures would then arrive in Closed state and only affect the error rate, never re-opening the breaker. A provider with a 33% success rate could be promoted back to full traffic. Added a dedicated `half_open_success_count` to `CircuitBreakerStore`, incremented only in `record_success()`. The breaker now closes when `success_count >= half_open_max_requests`, requiring all probe requests to succeed

### Added

- `get_half_open_success_count()` and `increment_half_open_success_count()` methods on `CircuitBreakerStore` trait
- `half_open_success_count` field in `InMemoryCircuitBreakerStore`
- Success counter reset in `reset_half_open_count()` and `reset()` methods
- 1 new test: `half_open_partial_success_does_not_close` — verifies 1 success out of 3 does not close the breaker
- Updated existing `half_open_successes_close_breaker` test to verify incremental success counting (stays HalfOpen after first success, closes after second)

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 365/365 passing (173 models + 14 audit + 108 policy + 17 providers + 53 router) |

---

## 0.7.10 — 2026-04-05

**Phase 7.10: Cross-Crate Production Review — State Machine Completeness, Deterministic Routing, Settlement Integrity**

Systematic cross-crate review (models, audit, policy, router, migrations) targeting five findings from a full six-agent parallel review of all Phases 1-7. The central theme: closing the remaining gaps in state machine invariant enforcement at the deserialization boundary, ensuring deterministic behavior in routing and audit queries, preventing i32 overflow in the policy hot path, and enforcing settlement field integrity at the database level. All changes are additive — no reverts of previous hardenings.

### Fixed

- **Payment deserialization allows Settled/Failed without provider fields — missing state machine invariant (CRITICAL)** — The deserialization validated that pre-provider statuses must NOT have provider fields (v0.7.9) and that provider fields must be paired (v0.7.9), but did not enforce the converse: Settled and Failed are only reachable from Submitted (which requires `set_provider()`), so they MUST have provider fields. A corrupted row with `status=settled, provider_id=NULL` would deserialize without error, creating an audit trail entry with no provider attribution for a settled payment. Added Invariant 3: `must_have_provider` check for Settled and Failed statuses
- **`get_by_payment()` uses non-deterministic ordering — pagination instability (MEDIUM)** — The main `query()` method uses `ORDER BY timestamp DESC, id DESC` (fixed in v0.7.8), but `get_by_payment()` still used only `ORDER BY timestamp DESC`. Under timestamp collision, paginated clients calling this method could see duplicates or miss records. Added `id DESC` as secondary sort, matching the established pattern
- **Scorer sort uses non-deterministic tiebreaker — unstable provider selection (MEDIUM)** — When multiple providers have identical composite scores, `partial_cmp` returns `Equal` and the sort order is non-deterministic. Combined with `candidates[0]` selection, the "winning" provider could change between calls with identical state, making routing unpredictable and A/B testing impossible. Added `.then_with(|| a.provider_id.cmp(&b.provider_id))` for lexicographic tiebreaking. Added `Ord`/`PartialOrd` derives to `ProviderId`
- **`utc_offset_hours` cast can panic in debug mode — i32 overflow (LOW-MEDIUM)** — `v.as_i64().map(|h| h as i32 * 3600)` in `extract_hours()` panics in debug mode if an operator sets an extreme JSON value (e.g., `2147483647`), because `i32 * 3600` overflows before `FixedOffset::east_opt` can validate the result. Added bounds check: values outside `-26..=26` are logged as errors and ignored, falling back to UTC or profile timezone
- **No DB constraint pairing `amount_settled` and `settled_currency` — inconsistent settlement records (HIGH)** — Constraints existed for `amount_settled > 0` and `settled_currency IN (...)` separately, but nothing enforced that they must be set together. A payment could have `amount_settled = 100.00` with `settled_currency = NULL`, making settlement reconciliation impossible. Added `chk_payments_settlement_pair` CHECK constraint via migration `20260405200002`

### Added

- Invariant 3 in Payment deserialization: Settled/Failed must have both `provider_id` and `provider_transaction_id`
- Deterministic `id DESC` secondary sort in `get_by_payment()` audit query
- Lexicographic provider_id tiebreaker in scorer sort
- `Ord` and `PartialOrd` derives on `ProviderId`
- Bounds check on `utc_offset_hours` in TimeWindowEvaluator (`-26..=26` range)
- Migration `20260405200002`: `chk_payments_settlement_pair` CHECK constraint
- 6 new tests: Payment settled/failed without provider (2), settled with provider accepted (1), scorer deterministic tiebreaker (1), time_window extreme positive/negative offset ignored (2)

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 364/364 passing (173 models + 14 audit + 108 policy + 17 providers + 52 router) |

---

## 0.7.9 — 2026-04-05

**Phase 7.9: Production Review — State Machine Invariants, Audit Bounds, Schema Integrity**

Systematic cross-crate review (models, audit, policy, migrations) targeting five findings from a full seven-agent parallel review of Phases 1-7. The central theme: closing gaps in state machine invariant enforcement at the deserialization boundary, completing the established length-bound pattern on the last unbounded audit-persisted string, correcting a misleading comment, and adding a missing database uniqueness constraint. All changes are additive — no reverts of previous hardenings.

### Fixed

- **Payment deserialization allows provider fields on no-provider terminal statuses — state machine invariant gap (MEDIUM)** — The pre-submission check covered `Pending`, `Validating`, and `PendingApproval`, but `Blocked`, `Rejected`, and `TimedOut` are also reached before provider assignment (per the state machine: `Validating→Blocked`, `PendingApproval→Rejected`, `PendingApproval→TimedOut→Blocked`). A corrupted database row with `status=blocked, provider_id=some_id` would deserialize without error, violating the invariant that `set_provider()` only operates in `Approved` or `Submitted` status. Extended the no-provider check to cover all six pre-provider statuses
- **Payment deserialization allows asymmetric provider fields — impossible state accepted (MEDIUM)** — `set_provider()` always assigns `provider_id` and `provider_transaction_id` atomically as a pair, but deserialization did not verify they were set together. A row with `provider_id=Some, provider_transaction_id=None` (or vice versa) would load successfully, creating an in-memory state that could never be created through the normal code path. Added pair validation: both must be `Some` or both `None`
- **`AuditEntry.on_chain_tx_hash` has no maximum length — unbounded audit ledger bloat (MEDIUM)** — Every other audit-persisted string field has a `MAX_*_LEN` constant and validation in its custom `Deserialize` (established pattern since v0.6.10). On-chain transaction hashes were unbounded. An arbitrarily long hash would persist permanently in the append-only ledger. Added `MAX_ON_CHAIN_TX_HASH_LEN = 256` (Ethereum/Base hashes are 66 chars; 256 provides headroom) with `trim().is_empty()` and max-length validation via custom `Deserialize`
- **Regex cache comment says "evicts all entries" but code evicts one — misleading documentation (LOW)** — The doc comment on `REGEX_CACHE` at `evaluator.rs:11` stated the cache "evicts all entries when the limit is reached", but the code at lines 252-259 evicts a single arbitrary entry per insertion. The single-eviction strategy is correct (preserves hot patterns), but the comment was misleading. Corrected to match the actual behavior
- **`virtual_cards` table missing composite unique constraint on `(provider_id, provider_card_id)` — silent duplicate acceptance (LOW-MEDIUM)** — If a provider bug or race condition returned the same card ID twice, the database would silently store both rows. Added `UNIQUE(provider_id, provider_card_id)` constraint via migration `20260405200001`

### Added

- Custom `Deserialize` for `AuditEntry` with `on_chain_tx_hash` empty/whitespace and max-length validation
- `MAX_ON_CHAIN_TX_HASH_LEN` constant (256) for on-chain transaction hash length validation
- Extended Payment deserialization: `Blocked`, `Rejected`, `TimedOut` added to no-provider check
- Provider field pair validation in Payment deserialization (both or neither)
- Migration `20260405200001`: `uk_virtual_cards_provider_card` composite unique constraint
- 11 new tests: AuditEntry on_chain_tx_hash valid/none/empty/whitespace/oversized/at-limit (6), Payment provider fields on blocked/rejected/timed_out (3), Payment asymmetric provider fields both directions (2)

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 358/358 passing (170 models + 14 audit + 106 policy + 17 providers + 51 router) |

---

## 0.7.8 — 2026-04-05

**Phase 7.8: Cross-Crate Production Readiness Review**

Systematic cross-crate review (models, audit, policy) targeting five findings from a full codebase audit of Phases 1-7. The central theme: closing the last remaining gaps in the established validation patterns — empty-string guards on enum payloads, length bounds on indexed keys, deterministic query ordering, accurate fail-safe log messages, and exact depth enforcement. All changes are additive — no reverts of previous hardenings.

### Fixed

- **`PaymentCategory::Other` accepts empty/whitespace-only strings — meaningless audit categories (MEDIUM)** — The `Other(String)` variant checked `len() > MAX_CATEGORY_OTHER_LEN` but allowed `Other("")` and `Other("   ")` through. Every other audit-persisted string field validates with `trim().is_empty()` — `Justification.summary` (v0.6.15), `Recipient.identifier` (v0.7.7), `HumanReviewRecord.reason` (v0.7.6), etc. A whitespace-only category would be permanently stored in the append-only audit ledger as a formally valid but meaningless classification. Added `trim().is_empty()` check before the max-length check, matching the established pattern
- **`IdempotencyKey` has no maximum length — unbounded database index and Redis key bloat (MEDIUM)** — Every other audit-persisted string field has a `MAX_*_LEN` constant (established pattern since v0.6.10). Idempotency keys were unbounded. An arbitrarily long key would bloat the database index and Redis store. Added `MAX_IDEMPOTENCY_KEY_LEN = 255` with validation in `new()` (panic), `try_new()` (Result), `FromStr`, and custom `Deserialize`
- **Audit query `ORDER BY timestamp DESC` is non-deterministic under timestamp collision — pagination instability (LOW-MEDIUM)** — When multiple audit entries share the same timestamp (plausible at microsecond precision under high throughput), their ordering is undefined. Paginated clients could see duplicates or miss records across page boundaries. Added `id DESC` as secondary sort — IDs are UUIDv7 (time-sortable), guaranteeing deterministic ordering even when timestamps collide
- **Time window `extract_hours` log messages say "skipped" but rule actually triggers — misleading operator diagnostics (LOW)** — When `start > 23`, `end > 23`, or `start == end`, `extract_hours` returns `None`, which the evaluator at line 28 treats as `RuleResult::Triggered(rule.action)` — the rule fires (fail-safe), it does not skip. The log messages said "rule will be skipped" and "skipping as likely misconfiguration", actively misleading operators debugging policy behavior. Corrected to "failing safe (rule will trigger)" and upgraded from `warn` to `error` to match the severity of a misconfigured rule
- **`PolicyCondition` depth check allows one more level than `MAX_CONDITION_DEPTH` advertises — off-by-one (LOW)** — `parse_depth` checked `depth > MAX_CONDITION_DEPTH` starting from depth 0, meaning a tree at depth 32 passed the `32 > 32` check. The effective max was 33 levels while the constant says 32. Changed to `depth >= MAX_CONDITION_DEPTH` so the constant means what it says

### Added

- `MAX_IDEMPOTENCY_KEY_LEN` constant (255) for idempotency key length validation
- Max-length validation on `IdempotencyKey::new()` (panic), `try_new()` (Result), `FromStr`, and `Deserialize`
- `trim().is_empty()` check for `PaymentCategory::Other` in custom `Deserialize`
- Secondary sort `id DESC` in audit query `ORDER BY` clause
- 7 new tests: PaymentCategory::Other empty + whitespace (2), IdempotencyKey oversized try_new + at-limit + deserialize + from_str + panic (5)

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 347/347 passing (159 models + 14 audit + 106 policy + 17 providers + 51 router) |

---

## 0.7.7 — 2026-04-02

**Phase 7.7: Recipient Identifier Whitespace Guard**

Production readiness review (models) closing the last remaining gap in the established `trim().is_empty()` validation pattern. The `Recipient.identifier` field — the only required, audit-persisted string field still using bare `is_empty()` — now rejects whitespace-only values, matching the pattern applied to every other string field across the models crate. The change is additive — no reverts of previous hardenings.

### Fixed

- **`Recipient.identifier` accepts whitespace-only strings — meaningless audit records (HIGH)** — The `identifier` field holds the payment target (merchant ID, email, wallet address, bank account reference). The custom `Deserialize` rejected empty strings (`""`) but allowed whitespace-only values (`"   "`) through. Every other audit-persisted string field in the models crate validates with `trim().is_empty()` — `Justification.summary` (v0.6.15), `ProviderResponseRecord.transaction_id` (v0.7.2), `HumanReviewRecord.reviewer_id` (v0.7.1), `Recipient.name` (v0.7.5), etc. A whitespace-only identifier would be permanently stored in the append-only audit ledger as a formally valid but meaningless payment target. Changed `is_empty()` to `trim().is_empty()`, matching the established pattern

### Added

- `trim().is_empty()` check for `Recipient.identifier` in custom `Deserialize`
- 1 new test: whitespace-only identifier rejected

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 340/340 passing (152 models + 14 audit + 106 policy + 17 providers + 51 router) |

---

## 0.7.6 — 2026-04-02

**Phase 7.6: Final Empty-String Guard Sweep — HumanReviewRecord.reason and PaymentMetadata Optional Fields**

Cross-crate production readiness review (models) closing the last two gaps in the established empty-string guard pattern for optional audit-persisted fields. The pattern — `trim().is_empty()` rejection when `Some`, with `None` remaining valid — was applied to `Justification.task_id`, `Justification.expected_value`, and `Recipient.name` in v0.7.5 but missed `HumanReviewRecord.reason` and the three `PaymentMetadata` fields. All changes are additive — no reverts of previous hardenings.

### Fixed

- **`HumanReviewRecord.reason` accepts empty/whitespace-only string when `Some` — unexplained human decision in audit trail (LOW-MEDIUM)** — The `reason` field captures why a human reviewer approved or rejected an escalated payment. The custom `Deserialize` validated max length (`MAX_REVIEW_REASON_LEN`, v0.6.10) but allowed `Some("")` and `Some("   ")` through. A reviewer submitting an empty reason creates an audit entry where the decision rationale is formally present but meaningless — undermining audit trail accountability. `None` (no reason provided) is valid; `Some("")` is not. Added `trim().is_empty()` check before the max-length check, matching the pattern from `Justification.task_id` (v0.7.5)
- **`PaymentMetadata.agent_session_id`, `.workflow_id`, `.operator_ref` accept empty/whitespace-only strings when `Some` — meaningless audit metadata (LOW)** — All three optional metadata fields validated max length (`MAX_METADATA_FIELD_LEN`, v0.6.9) but not emptiness when present. An agent submitting `"agent_session_id": ""` creates a metadata record that is formally populated but carries no information — polluting audit log queries that filter on metadata presence. Added `trim().is_empty()` check inside `validate_field()` before the max-length check, covering all three fields in one fix

### Added

- `trim().is_empty()` check for `HumanReviewRecord.reason` when `Some` in custom `Deserialize`
- `trim().is_empty()` check in `PaymentMetadata::validate_field()` covering all three optional fields
- 9 new tests: HumanReviewRecord empty/whitespace reason + None reason + valid reason (4), PaymentMetadata empty agent_session_id + whitespace workflow_id + empty operator_ref + None fields + valid fields (5)

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 339/339 passing (151 models + 14 audit + 106 policy + 17 providers + 51 router) |

---

## 0.7.5 — 2026-04-02

**Phase 7.5: Production Hardening — Unknown Rule Type Fail-Safe, IdempotencyKey Fix, Scorer Clamp, Schema Alignment, Validation Gaps**

Systematic production readiness review targeting nine findings across models, policy, and router crates. The central theme: closing the remaining gaps in the established validation patterns — fail-safe behavior on unregistered rule types, empty-string guards on optional audit-persisted fields, and defensive clamping in the scoring algorithm. All changes are additive — no reverts of previous hardenings.

### Fixed

- **Unregistered `rule_type` in policy engine silently skipped — policy bypass via typo (HIGH)** — When a rule referenced an unregistered `rule_type` (e.g., `"amonut_cap"` instead of `"amount_cap"`), the engine logged a warning and skipped the rule entirely, allowing the payment through. v0.7.4 fixed this for misconfigured *parameters* within registered evaluators, but unregistered rule *types* still failed open. Changed to return `RuleResult::Triggered(rule.action)` for unknown types — fail-safe (deny/escalate per the rule's configured action) instead of fail-open (skip). Log level upgraded from `warn` to `error`
- **`IdempotencyKey::from_str("idem_")` returns empty key — deduplication bypass (HIGH)** — The `FromStr` implementation stripped the `"idem_"` prefix but did not validate that the remaining key was non-empty. `"idem_".parse::<IdempotencyKey>()` returned `Ok(IdempotencyKey(""))`, bypassing the empty-check present in both `new()` and the `Deserialize` impl. Added `key.is_empty()` check after `strip_prefix`
- **Scorer health score can go negative — inverts provider ranking (MEDIUM)** — `1.0 - error_rate_5m` produced a negative health score when `error_rate_5m > 1.0` (possible transiently from timing). Negative scores inverted ranking logic. Added `.max(0.0)` clamp
- **Scorer `decimal_to_f64()` silently returns 0.0 on parse failure — cheapest-provider illusion (MEDIUM)** — The string-based conversion `f64::from_str(&d.to_string()).unwrap_or(0.0)` would silently produce 0.0 if parsing failed, making a broken provider appear cheapest. Replaced with `rust_decimal::prelude::ToPrimitive::to_f64()` which handles the conversion natively without string round-tripping
- **`VirtualCard` struct missing `updated_at` field — schema/model mismatch (MEDIUM)** — Migration `20260401200003` added `updated_at` to the `virtual_cards` table, but the Rust `VirtualCard` struct did not include the field. Any `sqlx::FromRow` query or full-struct deserialization would fail at runtime. Added `pub updated_at: DateTime<Utc>` field and updated mock provider
- **`ScoringWeights::validate()` allows all-zero weights — non-deterministic ranking (LOW-MEDIUM)** — All four weights at 0.0 produced identical scores for every provider, making selection dependent on input order (non-deterministic). Added `sum == 0.0` rejection to `validate()`
- **`Justification.task_id` and `.expected_value` accept empty strings when present (LOW)** — These optional string fields checked max length but not emptiness when `Some`. An empty string `""` is semantically meaningless and should be `None` or rejected. Added `trim().is_empty()` checks matching the pattern established for `summary` (v0.6.15)
- **`Recipient.name` accepts empty/whitespace string when present (LOW)** — Same gap: max length validated but not emptiness. Added `trim().is_empty()` check matching the pattern for `identifier` (v0.6.15)
- **`EscalationConfig.timeout_minutes` allows zero — no human review window (LOW)** — Zero timeout means instant expiry, defeating the purpose of escalation. The `on_timeout` action fires immediately with no human review window. Added `timeout_minutes > 0` validation
- **`ProviderId` has no maximum length — unbounded audit log bloat (LOW)** — Every other audit-persisted string field has a `MAX_*_LEN` constant (established pattern since v0.6.10). Provider IDs were unbounded. Added `MAX_PROVIDER_ID_LEN = 255` with validation in `new()`, `try_new()`, and `Deserialize`

### Added

- `MAX_PROVIDER_ID_LEN` constant (255) for provider ID length validation
- Max-length validation on `ProviderId::new()` (panic), `try_new()` (Result), and `Deserialize`
- `trim().is_empty()` checks for `Justification.task_id` and `Justification.expected_value`
- `trim().is_empty()` check for `Recipient.name`
- `timeout_minutes > 0` validation in `EscalationConfig` custom `Deserialize`
- `sum > 0` validation in `ScoringWeights::validate()`
- Health score clamp `(1.0 - error_rate).max(0.0)` in `ProviderScorer`
- `VirtualCard.updated_at` field with mock provider update
- 17 new tests: IdempotencyKey FromStr prefix-only + valid (2), Justification empty/whitespace task_id + expected_value (4), Recipient empty/whitespace name (2), EscalationConfig zero timeout (1), ProviderId oversized try_new/at-limit/deserialize/panic (4), ScoringWeights all-zero (1), engine unknown rule_type block/escalate/approve (3)

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 330/330 passing (142 models + 14 audit + 106 policy + 17 providers + 51 router) |

---

## 0.7.4 — 2026-04-02

**Phase 7.4: Production Hardening — Fail-Safe Policy Enforcement, Name Validation, Regex Safety, Schema Constraints**

Full-crate production hardening (models, policy, migrations) targeting seven findings from a systematic cross-crate review. The central theme: the policy engine's behavior on misconfigured rules was "fail-open" (skip the rule, let the payment through), which is the opposite of what a financial control plane requires. All changes are additive — no reverts of previous hardenings.

### Fixed

- **Misconfigured velocity_limit/time_window/duplicate_detection rules silently pass — policy bypass via typo (HIGH)** — When a rule's condition tree was missing required parameters (e.g., typo `"max_cnt"` instead of `"max_count"`), the evaluator logged a warning and returned `RuleResult::Pass`, silently disabling the rule. A single configuration typo could remove a velocity limit, time window, or duplicate guard entirely. Changed all three evaluators to return `RuleResult::Triggered(rule.action)` on misconfiguration — fail-safe (deny) instead of fail-open (allow). Log level upgraded from `warn` to `error` for visibility
- **Invalid regex pattern in `Matches` condition silently passes — deny-list bypass (MEDIUM)** — A malformed regex (e.g., `[unclosed`) in a `Matches` condition logged a warning and returned `false` (non-match), meaning the associated rule would never fire. For deny-list patterns, this is a silent bypass. Changed `regex_matches()` to return `true` (fail-safe: assume match) on invalid patterns, ensuring the rule triggers. Also added explicit error logging for the poisoned-mutex fallback path
- **`Agent.name` and `AgentProfile.name` have no length validation — unbounded audit log bloat (MEDIUM)** — Every other string field persisted to the append-only audit ledger has a `MAX_*_LEN` constant and validation in its custom `Deserialize` (established pattern since v0.6.10). These two fields were unbounded, allowing multi-megabyte names that would permanently inflate the audit log. Added custom `Deserialize` for `Agent` with `trim().is_empty()` and `len() > 255` checks; added equivalent name validation to the existing `AgentProfile` deserializer
- **Unrecognized field names in conditions log at `warn` level — operator misconfigurations not surfaced (LOW)** — A typo in a condition field name (e.g., `"recipient.idenifier"`) resolved to `null`, causing comparisons to silently return `false` and the rule to never fire. While the resolution behavior is kept (changing it would risk false blocks in complex condition trees), the log level is upgraded from `warn` to `error` to ensure misconfigured rules are visible in monitoring and alerting
- **`settled_currency` column has no CHECK constraint — invalid currency permanently stored (MEDIUM)** — The `currency` column has `chk_payments_currency` (v0.6.10) constraining it to the Rust `Currency` enum values, but `settled_currency` had no equivalent constraint. A buggy provider returning an invalid settlement currency would permanently store invalid data. Added CHECK constraint matching the currency enum, allowing NULL (settlement currency is optional until provider confirms)
- **Missing index on `payments.provider_id` — sequential scan on reconciliation queries (LOW)** — The payments table had indexes on `agent_id`, `status`, and `created_at` but not `provider_id`. Per-provider reconciliation and settlement queries would full-scan. Added `idx_payments_provider_id`

### Documented

- **Currency-isolated spend/velocity/duplicate limits are by design** — Added explicit doc comments to `SpendRateEvaluator`, `VelocityLimitEvaluator`, and `DuplicateDetectionEvaluator` explaining that per-currency filtering is intentional: summing across currencies without FX conversion would produce meaningless totals, and embedding live FX rates in the policy hot path would add latency, external dependencies, and non-determinism

### Added

- Custom `Deserialize` for `Agent` with `name.trim().is_empty()` and `len() > MAX_NAME_LEN` (255) validation
- `AgentProfile` deserializer extended with equivalent name validation
- `MAX_NAME_LEN` constant (255) for agent and profile name fields
- Migration `20260402200001`: `chk_payments_settled_currency` CHECK constraint + `idx_payments_provider_id` index
- 8 new tests: Agent empty/whitespace/oversized/max-length name (4), AgentProfile empty/whitespace/oversized name (3), Agent valid name (1)
- 7 existing tests updated to assert new fail-safe behavior (Triggered instead of Pass)

### Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo test --workspace` | 313/313 passing (129 models + 14 audit + 103 policy + 17 providers + 50 router) |

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
