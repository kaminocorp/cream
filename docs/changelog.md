# Changelog

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
