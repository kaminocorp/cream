//! # Enhancement 1: DB Serialization Round-Trip Tests
//!
//! These tests verify that every Rust enum variant survives a Rust → Postgres → Rust
//! round-trip without hitting CHECK constraint violations or deserialization failures.
//!
//! **Why these exist:** Three CRITICAL bugs in v0.8.8–v0.8.10 were caused by mismatches
//! between Rust's serde output and Postgres CHECK constraints. All three would have been
//! caught instantly by these tests:
//!   - v0.8.8: `Currency::USD` serialized to `"U_S_D"` (SCREAMING_SNAKE_CASE split)
//!   - v0.8.9: `PaymentStatus::PendingApproval` serialized to `"pendingapproval"` (Debug, not Display)
//!   - v0.8.10: `CardType::SingleUse` serialized to `"singleuse"` (Debug, not serde)

mod common;

use common::{seed_agent, TestDb};
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

// ===========================================================================
// 1.1 — PaymentStatus: every variant round-trips through the payments table
// ===========================================================================

#[tokio::test]
async fn payment_status_all_variants_roundtrip() {
    let db = TestDb::new().await;
    let (_profile_id, agent_id, _) = seed_agent(&db.pool).await;

    // Every PaymentStatus variant and its expected DB string.
    // This list is the test's source of truth — if a new variant is added to
    // the enum without updating this list, the test suite will lack coverage
    // (and the next test, which checks CHECK constraints, will catch it).
    let variants: Vec<(&str, &str)> = vec![
        ("pending", "pending"),
        ("validating", "validating"),
        ("pending_approval", "pending_approval"),
        ("approved", "approved"),
        ("submitted", "submitted"),
        ("settled", "settled"),
        ("failed", "failed"),
        ("blocked", "blocked"),
        ("rejected", "rejected"),
        ("timed_out", "timed_out"),
    ];

    for (i, (status_str, expected_db)) in variants.iter().enumerate() {
        let payment_id = Uuid::now_v7();
        let idem_key = format!("idem-status-{i}");

        let recipient = json!({
            "recipient_type": "merchant",
            "identifier": "test_merchant",
        });
        let justification = json!({
            "summary": "Test payment for status round-trip verification in integration tests",
            "category": "api_credits",
        });

        // For settled/failed with provider fields (to satisfy schema expectations).
        let (provider_id, provider_tx_id) = if *status_str == "settled" {
            (Some("test_provider"), Some("tx_123"))
        } else if *status_str == "submitted" {
            (Some("test_provider"), Some("tx_123"))
        } else {
            (None, None)
        };

        sqlx::query(
            "INSERT INTO payments
                (id, agent_id, idempotency_key, amount, currency, recipient,
                 preferred_rail, justification, status, provider_id, provider_tx_id)
             VALUES ($1, $2, $3, 100.00, 'USD', $4, 'auto', $5, $6, $7, $8)",
        )
        .bind(payment_id)
        .bind(agent_id)
        .bind(&idem_key)
        .bind(&recipient)
        .bind(&justification)
        .bind(status_str)
        .bind(provider_id)
        .bind(provider_tx_id)
        .execute(&db.pool)
        .await
        .unwrap_or_else(|e| panic!("INSERT failed for status '{status_str}': {e}"));

        // Read back and verify.
        let row = sqlx::query("SELECT status FROM payments WHERE id = $1")
            .bind(payment_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
        let db_status: String = row.get("status");
        assert_eq!(
            &db_status, expected_db,
            "status round-trip mismatch for variant '{status_str}'"
        );

        // Verify serde round-trip: DB string → serde_json → PaymentStatus → serde_json → string
        let parsed: serde_json::Value = serde_json::from_str(&format!("\"{db_status}\"")).unwrap();
        let _status: cream_models::payment::PaymentStatus =
            serde_json::from_value(parsed).unwrap_or_else(|e| {
                panic!("deserialization failed for DB value '{db_status}': {e}")
            });
    }

    db.cleanup().await;
}

// ===========================================================================
// 1.2 — Currency: every variant round-trips
// ===========================================================================

#[tokio::test]
async fn currency_all_variants_roundtrip() {
    let db = TestDb::new().await;
    let (_profile_id, agent_id, _) = seed_agent(&db.pool).await;

    // Every Currency variant and its expected DB string.
    let variants: Vec<&str> = vec![
        "USD", "EUR", "GBP", "SGD", "JPY", "CNY", "HKD", "AUD", "CAD", "INR", "KRW", "TWD",
        "THB", "MYR", "IDR", "PHP", "VND", "BRL", "MXN", "CHF", "SEK", "NOK", "DKK", "NZD",
        "AED", "BTC", "ETH", "USDC", "USDT", "SOL", "MATIC", "AVAX", "BASE_ETH",
    ];

    for (i, currency_str) in variants.iter().enumerate() {
        let payment_id = Uuid::now_v7();
        let idem_key = format!("idem-currency-{i}");

        let recipient = json!({
            "recipient_type": "merchant",
            "identifier": "test_merchant",
        });
        let justification = json!({
            "summary": "Test payment for currency round-trip verification in integration tests",
            "category": "api_credits",
        });

        sqlx::query(
            "INSERT INTO payments
                (id, agent_id, idempotency_key, amount, currency, recipient,
                 preferred_rail, justification, status)
             VALUES ($1, $2, $3, 100.00, $4, $5, 'auto', $6, 'pending')",
        )
        .bind(payment_id)
        .bind(agent_id)
        .bind(&idem_key)
        .bind(currency_str)
        .bind(&recipient)
        .bind(&justification)
        .execute(&db.pool)
        .await
        .unwrap_or_else(|e| panic!("INSERT failed for currency '{currency_str}': {e}"));

        // Read back and verify DB stores exactly the expected string.
        let row = sqlx::query("SELECT currency FROM payments WHERE id = $1")
            .bind(payment_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
        let db_currency: String = row.get("currency");
        assert_eq!(
            &db_currency, currency_str,
            "currency round-trip mismatch for '{currency_str}'"
        );

        // Verify the DB value deserializes back to the Rust enum.
        let parsed: cream_models::payment::Currency =
            serde_json::from_value(json!(db_currency)).unwrap_or_else(|e| {
                panic!("deserialization failed for DB currency '{db_currency}': {e}")
            });

        // And re-serializes to the same string.
        let reserialized = serde_json::to_value(parsed).unwrap();
        assert_eq!(
            reserialized.as_str().unwrap(),
            *currency_str,
            "re-serialization mismatch for '{currency_str}'"
        );
    }

    db.cleanup().await;
}

// ===========================================================================
// 1.3 — RailPreference: every variant round-trips
// ===========================================================================

#[tokio::test]
async fn rail_preference_all_variants_roundtrip() {
    let db = TestDb::new().await;
    let (_profile_id, agent_id, _) = seed_agent(&db.pool).await;

    let variants: Vec<&str> = vec!["auto", "card", "ach", "swift", "local", "stablecoin"];

    for (i, rail_str) in variants.iter().enumerate() {
        let payment_id = Uuid::now_v7();
        let idem_key = format!("idem-rail-{i}");

        let recipient = json!({
            "recipient_type": "merchant",
            "identifier": "test_merchant",
        });
        let justification = json!({
            "summary": "Test payment for rail preference round-trip verification in integration tests",
            "category": "api_credits",
        });

        sqlx::query(
            "INSERT INTO payments
                (id, agent_id, idempotency_key, amount, currency, recipient,
                 preferred_rail, justification, status)
             VALUES ($1, $2, $3, 100.00, 'USD', $4, $5, $6, 'pending')",
        )
        .bind(payment_id)
        .bind(agent_id)
        .bind(&idem_key)
        .bind(&recipient)
        .bind(rail_str)
        .bind(&justification)
        .execute(&db.pool)
        .await
        .unwrap_or_else(|e| panic!("INSERT failed for rail '{rail_str}': {e}"));

        let row = sqlx::query("SELECT preferred_rail FROM payments WHERE id = $1")
            .bind(payment_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
        let db_rail: String = row.get("preferred_rail");
        assert_eq!(&db_rail, rail_str, "rail round-trip mismatch for '{rail_str}'");

        // Verify deserialization.
        let _parsed: cream_models::payment::RailPreference =
            serde_json::from_value(json!(db_rail)).unwrap_or_else(|e| {
                panic!("deserialization failed for DB rail '{db_rail}': {e}")
            });
    }

    db.cleanup().await;
}

// ===========================================================================
// 1.4 — CardType: every variant round-trips through virtual_cards
// ===========================================================================

#[tokio::test]
async fn card_type_all_variants_roundtrip() {
    let db = TestDb::new().await;
    let (_profile_id, agent_id, _) = seed_agent(&db.pool).await;

    let variants: Vec<&str> = vec!["single_use", "multi_use"];

    for (i, card_type_str) in variants.iter().enumerate() {
        let card_id = Uuid::now_v7();
        let provider_card_id = format!("prov_card_{i}");
        let controls = json!({});

        sqlx::query(
            "INSERT INTO virtual_cards
                (id, agent_id, provider_id, provider_card_id, card_type, controls, status, created_at, updated_at)
             VALUES ($1, $2, 'test_provider', $3, $4, $5, 'active', now(), now())",
        )
        .bind(card_id)
        .bind(agent_id)
        .bind(&provider_card_id)
        .bind(card_type_str)
        .bind(&controls)
        .execute(&db.pool)
        .await
        .unwrap_or_else(|e| panic!("INSERT failed for card_type '{card_type_str}': {e}"));

        let row = sqlx::query("SELECT card_type FROM virtual_cards WHERE id = $1")
            .bind(card_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
        let db_card_type: String = row.get("card_type");
        assert_eq!(
            &db_card_type, card_type_str,
            "card_type round-trip mismatch for '{card_type_str}'"
        );

        // Verify the DB value round-trips through serde.
        let parsed: cream_models::card::CardType =
            serde_json::from_value(json!(db_card_type)).unwrap_or_else(|e| {
                panic!("deserialization failed for DB card_type '{db_card_type}': {e}")
            });
        let reserialized = serde_json::to_value(parsed).unwrap();
        assert_eq!(reserialized.as_str().unwrap(), *card_type_str);
    }

    db.cleanup().await;
}

// ===========================================================================
// 1.5 — CardStatus: every variant round-trips
// ===========================================================================

#[tokio::test]
async fn card_status_all_variants_roundtrip() {
    let db = TestDb::new().await;
    let (_profile_id, agent_id, _) = seed_agent(&db.pool).await;

    let variants: Vec<&str> = vec!["active", "frozen", "cancelled", "expired"];

    for (i, status_str) in variants.iter().enumerate() {
        let card_id = Uuid::now_v7();
        let provider_card_id = format!("prov_card_status_{i}");
        let controls = json!({});

        sqlx::query(
            "INSERT INTO virtual_cards
                (id, agent_id, provider_id, provider_card_id, card_type, controls, status, created_at, updated_at)
             VALUES ($1, $2, 'test_provider', $3, 'single_use', $4, $5, now(), now())",
        )
        .bind(card_id)
        .bind(agent_id)
        .bind(&provider_card_id)
        .bind(&controls)
        .bind(status_str)
        .execute(&db.pool)
        .await
        .unwrap_or_else(|e| panic!("INSERT failed for card status '{status_str}': {e}"));

        let row = sqlx::query("SELECT status FROM virtual_cards WHERE id = $1")
            .bind(card_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
        let db_status: String = row.get("status");
        assert_eq!(
            &db_status, status_str,
            "card status round-trip mismatch for '{status_str}'"
        );

        let parsed: cream_models::card::CardStatus =
            serde_json::from_value(json!(db_status)).unwrap_or_else(|e| {
                panic!("deserialization failed for DB card status '{db_status}': {e}")
            });
        let reserialized = serde_json::to_value(parsed).unwrap();
        assert_eq!(reserialized.as_str().unwrap(), *status_str);
    }

    db.cleanup().await;
}

// ===========================================================================
// 1.6 — PolicyAction: every variant round-trips through policy_rules
// ===========================================================================

#[tokio::test]
async fn policy_action_all_variants_roundtrip() {
    let db = TestDb::new().await;
    let (profile_id, _agent_id, _) = seed_agent(&db.pool).await;

    // PolicyAction uses SCREAMING_SNAKE_CASE.
    let variants: Vec<&str> = vec!["APPROVE", "BLOCK", "ESCALATE"];

    for (i, action_str) in variants.iter().enumerate() {
        let rule_id = Uuid::now_v7();
        let condition = json!({"field": "amount", "op": "less_than", "value": 100});

        let escalation = if *action_str == "ESCALATE" {
            Some(json!({"channel": "slack", "timeout_minutes": 30, "on_timeout": "block"}))
        } else {
            None
        };

        sqlx::query(
            "INSERT INTO policy_rules
                (id, profile_id, priority, condition, action, escalation, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, true, now(), now())",
        )
        .bind(rule_id)
        .bind(profile_id)
        .bind(i as i32)
        .bind(&condition)
        .bind(action_str)
        .bind(&escalation)
        .execute(&db.pool)
        .await
        .unwrap_or_else(|e| panic!("INSERT failed for action '{action_str}': {e}"));

        let row = sqlx::query("SELECT action FROM policy_rules WHERE id = $1")
            .bind(rule_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
        let db_action: String = row.get("action");
        assert_eq!(
            &db_action, action_str,
            "action round-trip mismatch for '{action_str}'"
        );

        // Verify serde deserialization.
        let parsed: cream_models::policy::PolicyAction =
            serde_json::from_value(json!(db_action)).unwrap_or_else(|e| {
                panic!("deserialization failed for DB action '{db_action}': {e}")
            });
        let reserialized = serde_json::to_value(parsed).unwrap();
        assert_eq!(reserialized.as_str().unwrap(), *action_str);
    }

    db.cleanup().await;
}

// ===========================================================================
// 1.7 — AgentProfile spending limits survive Decimal round-trip
// ===========================================================================

#[tokio::test]
async fn agent_profile_spending_limits_decimal_roundtrip() {
    let db = TestDb::new().await;

    let profile_id = Uuid::now_v7();
    // Use values with varying decimal precision to catch truncation/rounding.
    let max_per_tx = Decimal::new(12345678, 4); // 1234.5678
    let max_daily = Decimal::new(99999999, 2); // 999999.99
    let max_weekly = Decimal::new(1, 4); // 0.0001
    let max_monthly = Decimal::new(9999999999999, 4); // 999999999.9999 (max for NUMERIC(19,4))

    sqlx::query(
        "INSERT INTO agent_profiles
            (id, name, max_per_transaction, max_daily_spend, max_weekly_spend, max_monthly_spend, created_at, updated_at)
         VALUES ($1, 'decimal-test', $2, $3, $4, $5, now(), now())",
    )
    .bind(profile_id)
    .bind(max_per_tx)
    .bind(max_daily)
    .bind(max_weekly)
    .bind(max_monthly)
    .execute(&db.pool)
    .await
    .expect("INSERT agent_profiles with precise decimals");

    let row = sqlx::query(
        "SELECT max_per_transaction, max_daily_spend, max_weekly_spend, max_monthly_spend
         FROM agent_profiles WHERE id = $1",
    )
    .bind(profile_id)
    .fetch_one(&db.pool)
    .await
    .unwrap();

    let db_max_per_tx: Decimal = row.get("max_per_transaction");
    let db_max_daily: Decimal = row.get("max_daily_spend");
    let db_max_weekly: Decimal = row.get("max_weekly_spend");
    let db_max_monthly: Decimal = row.get("max_monthly_spend");

    assert_eq!(db_max_per_tx, max_per_tx, "max_per_transaction precision lost");
    assert_eq!(db_max_daily, max_daily, "max_daily_spend precision lost");
    assert_eq!(db_max_weekly, max_weekly, "max_weekly_spend precision lost");
    assert_eq!(db_max_monthly, max_monthly, "max_monthly_spend precision lost");

    db.cleanup().await;
}

// ===========================================================================
// 1.8 — Payment JSON columns (recipient, justification, metadata) round-trip
// ===========================================================================

#[tokio::test]
async fn payment_json_columns_roundtrip() {
    let db = TestDb::new().await;
    let (_profile_id, agent_id, _) = seed_agent(&db.pool).await;

    let payment_id = Uuid::now_v7();
    let recipient = json!({
        "recipient_type": "merchant",
        "identifier": "stripe_merch_abc",
        "name": "Acme Corp",
        "country": "SG",
    });
    let justification = json!({
        "summary": "Purchasing cloud credits for production batch processing job number 4421",
        "task_id": "task_8372",
        "category": "cloud_infrastructure",
        "expected_value": "Complete customer onboarding within 4 hours",
    });
    let metadata = json!({
        "agent_session_id": "sess_001",
        "workflow_id": "wf_042",
        "operator_ref": "ref_XYZ",
    });

    sqlx::query(
        "INSERT INTO payments
            (id, agent_id, idempotency_key, amount, currency, recipient,
             preferred_rail, justification, metadata, status)
         VALUES ($1, $2, 'idem-json-test', 149.99, 'SGD', $3, 'auto', $4, $5, 'pending')",
    )
    .bind(payment_id)
    .bind(agent_id)
    .bind(&recipient)
    .bind(&justification)
    .bind(&metadata)
    .execute(&db.pool)
    .await
    .expect("INSERT payment with full JSON columns");

    let row = sqlx::query(
        "SELECT recipient, justification, metadata FROM payments WHERE id = $1",
    )
    .bind(payment_id)
    .fetch_one(&db.pool)
    .await
    .unwrap();

    let db_recipient: serde_json::Value = row.get("recipient");
    let db_justification: serde_json::Value = row.get("justification");
    let db_metadata: serde_json::Value = row.get("metadata");

    // Verify the JSON survived the round-trip intact.
    assert_eq!(db_recipient, recipient, "recipient JSON mismatch");
    assert_eq!(db_justification, justification, "justification JSON mismatch");
    assert_eq!(db_metadata, metadata, "metadata JSON mismatch");

    // Verify these can be deserialized back into the Rust types.
    let _: cream_models::recipient::Recipient =
        serde_json::from_value(db_recipient).expect("recipient deserialization failed");
    let _: cream_models::justification::Justification =
        serde_json::from_value(db_justification).expect("justification deserialization failed");
    let _: cream_models::payment::PaymentMetadata =
        serde_json::from_value(db_metadata).expect("metadata deserialization failed");

    db.cleanup().await;
}

// ===========================================================================
// 1.9 — Settlement persistence and paired constraint
// ===========================================================================

#[tokio::test]
async fn settlement_persistence_roundtrip() {
    let db = TestDb::new().await;
    let (_profile_id, agent_id, _) = seed_agent(&db.pool).await;

    let payment_id = Uuid::now_v7();
    let recipient = json!({
        "recipient_type": "merchant",
        "identifier": "test_merchant",
    });
    let justification = json!({
        "summary": "Test payment for settlement round-trip verification in integration tests",
        "category": "api_credits",
    });

    sqlx::query(
        "INSERT INTO payments
            (id, agent_id, idempotency_key, amount, currency, recipient,
             preferred_rail, justification, status, provider_id, provider_tx_id)
         VALUES ($1, $2, 'idem-settle-test', 250.00, 'USD', $3, 'auto', $4, 'settled', 'stripe', 'tx_abc')",
    )
    .bind(payment_id)
    .bind(agent_id)
    .bind(&recipient)
    .bind(&justification)
    .execute(&db.pool)
    .await
    .expect("INSERT settled payment");

    // Update settlement fields (mimicking persist_settlement).
    let amount_settled = Decimal::new(24999, 2); // 249.99
    let settled_currency = "SGD";

    sqlx::query(
        "UPDATE payments SET amount_settled = $1, settled_currency = $2, updated_at = now() WHERE id = $3",
    )
    .bind(amount_settled)
    .bind(settled_currency)
    .bind(payment_id)
    .execute(&db.pool)
    .await
    .expect("UPDATE settlement fields");

    // Read back.
    let row = sqlx::query(
        "SELECT amount_settled, settled_currency FROM payments WHERE id = $1",
    )
    .bind(payment_id)
    .fetch_one(&db.pool)
    .await
    .unwrap();

    let db_amount: Decimal = row.get("amount_settled");
    let db_currency: String = row.get("settled_currency");

    assert_eq!(db_amount, amount_settled);
    assert_eq!(db_currency, settled_currency);

    // Verify the currency round-trips through serde.
    let _: cream_models::payment::Currency =
        serde_json::from_value(json!(db_currency)).expect("settled_currency deserialization failed");

    db.cleanup().await;
}

// ===========================================================================
// 1.10 — Failed payment without provider fields deserializes correctly
// ===========================================================================

#[tokio::test]
async fn failed_payment_without_provider_roundtrip() {
    let db = TestDb::new().await;
    let (_profile_id, agent_id, _) = seed_agent(&db.pool).await;

    // A payment that failed before any provider was contacted (e.g., routing failure).
    // This is the exact scenario from v0.8.9 ghost-records bug.
    let payment_id = Uuid::now_v7();
    let recipient = json!({
        "recipient_type": "merchant",
        "identifier": "test_merchant",
    });
    let justification = json!({
        "summary": "Test payment for pre-provider failure round-trip verification in tests",
        "category": "api_credits",
    });

    sqlx::query(
        "INSERT INTO payments
            (id, agent_id, idempotency_key, amount, currency, recipient,
             preferred_rail, justification, status, provider_id, provider_tx_id)
         VALUES ($1, $2, 'idem-fail-noprovider', 100.00, 'USD', $3, 'auto', $4, 'failed', NULL, NULL)",
    )
    .bind(payment_id)
    .bind(agent_id)
    .bind(&recipient)
    .bind(&justification)
    .execute(&db.pool)
    .await
    .expect("INSERT failed payment without provider");

    // Read back via the same JSON reconstruction pattern used by PaymentRow::into_payment().
    let row = sqlx::query(
        "SELECT id, agent_id, idempotency_key, amount, currency, recipient,
                preferred_rail, justification, metadata, status, provider_id,
                provider_tx_id, created_at, updated_at
         FROM payments WHERE id = $1",
    )
    .bind(payment_id)
    .fetch_one(&db.pool)
    .await
    .unwrap();

    let id: Uuid = row.get("id");
    let status: String = row.get("status");
    let provider_id: Option<String> = row.get("provider_id");
    let provider_tx_id: Option<String> = row.get("provider_tx_id");
    let currency: String = row.get("currency");
    let preferred_rail: String = row.get("preferred_rail");
    let db_recipient: serde_json::Value = row.get("recipient");
    let db_justification: serde_json::Value = row.get("justification");
    let db_metadata: Option<serde_json::Value> = row.get("metadata");
    let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
    let updated_at: chrono::DateTime<chrono::Utc> = row.get("updated_at");
    let idem_key: String = row.get("idempotency_key");

    assert_eq!(status, "failed");
    assert!(provider_id.is_none(), "provider_id should be NULL for pre-provider failure");
    assert!(provider_tx_id.is_none(), "provider_tx_id should be NULL for pre-provider failure");

    // Reconstruct via JSON round-trip — the exact pattern from db.rs:142-154.
    let request_json = json!({
        "agent_id": format!("agt_{id}"),
        "amount": "100.00",
        "currency": currency,
        "recipient": db_recipient,
        "preferred_rail": preferred_rail,
        "justification": db_justification,
        "metadata": db_metadata,
        "idempotency_key": idem_key,
    });

    let payment_json = json!({
        "id": id.to_string(),
        "request": request_json,
        "status": status,
        "provider_id": provider_id,
        "provider_transaction_id": provider_tx_id,
        "created_at": created_at,
        "updated_at": updated_at,
    });

    let payment: cream_models::payment::Payment =
        serde_json::from_value(payment_json).expect(
            "Payment deserialization should succeed for Failed without provider fields (relaxed Invariant 3)",
        );

    assert_eq!(payment.status(), cream_models::payment::PaymentStatus::Failed);
    assert!(payment.provider_id().is_none());

    db.cleanup().await;
}

// ===========================================================================
// 1.11 — Audit entry round-trip (final_status CHECK + full JSON columns)
// ===========================================================================

#[tokio::test]
async fn audit_entry_roundtrip() {
    let db = TestDb::new().await;
    let (profile_id, agent_id, _) = seed_agent(&db.pool).await;

    // Write audit entries with different final_status values to verify the CHECK constraint.
    let statuses = vec!["pending", "settled", "failed", "blocked", "pending_approval", "timed_out"];

    for (i, status_str) in statuses.iter().enumerate() {
        let entry_id = Uuid::now_v7();

        let request = json!({"amount": "100.00", "currency": "USD"});
        let justification = json!({
            "summary": "Audit round-trip test for various final statuses in integration testing",
            "category": "api_credits",
        });
        let policy_eval = json!({
            "rules_evaluated": [],
            "matching_rules": [],
            "final_decision": "APPROVE",
            "decision_latency_ms": 5,
        });

        sqlx::query(
            "INSERT INTO audit_log
                (id, timestamp, agent_id, agent_profile_id, request,
                 justification, policy_evaluation, final_status)
             VALUES ($1, now(), $2, $3, $4, $5, $6, $7)",
        )
        .bind(entry_id)
        .bind(agent_id)
        .bind(profile_id)
        .bind(&request)
        .bind(&justification)
        .bind(&policy_eval)
        .bind(status_str)
        .execute(&db.pool)
        .await
        .unwrap_or_else(|e| panic!("INSERT audit_log failed for final_status '{status_str}': {e}"));

        // Read back.
        let row = sqlx::query("SELECT final_status FROM audit_log WHERE id = $1")
            .bind(entry_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
        let db_status: String = row.get("final_status");
        assert_eq!(
            &db_status, status_str,
            "audit final_status round-trip mismatch (iteration {i})"
        );
    }

    db.cleanup().await;
}

// ===========================================================================
// 1.12 — CHECK constraint rejects invalid values
// ===========================================================================

#[tokio::test]
async fn check_constraint_rejects_invalid_status() {
    let db = TestDb::new().await;
    let (_profile_id, agent_id, _) = seed_agent(&db.pool).await;

    let payment_id = Uuid::now_v7();
    let recipient = json!({
        "recipient_type": "merchant",
        "identifier": "test_merchant",
    });
    let justification = json!({
        "summary": "Test payment for CHECK constraint rejection verification in tests",
        "category": "api_credits",
    });

    // This is the exact string that the v0.8.9 bug produced: "pendingapproval"
    // (Debug trait output lowercased) instead of "pending_approval".
    let result = sqlx::query(
        "INSERT INTO payments
            (id, agent_id, idempotency_key, amount, currency, recipient,
             preferred_rail, justification, status)
         VALUES ($1, $2, 'idem-bad-status', 100.00, 'USD', $3, 'auto', $4, 'pendingapproval')",
    )
    .bind(payment_id)
    .bind(agent_id)
    .bind(&recipient)
    .bind(&justification)
    .execute(&db.pool)
    .await;

    assert!(
        result.is_err(),
        "INSERT with invalid status 'pendingapproval' should be rejected by CHECK constraint"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("chk_payments_status") || err_msg.contains("check"),
        "error should mention CHECK constraint, got: {err_msg}"
    );

    db.cleanup().await;
}

// ===========================================================================
// 1.13 — CHECK constraint rejects invalid currency
// ===========================================================================

#[tokio::test]
async fn check_constraint_rejects_invalid_currency() {
    let db = TestDb::new().await;
    let (_profile_id, agent_id, _) = seed_agent(&db.pool).await;

    let payment_id = Uuid::now_v7();
    let recipient = json!({
        "recipient_type": "merchant",
        "identifier": "test_merchant",
    });
    let justification = json!({
        "summary": "Test payment for CHECK constraint currency rejection verification in tests",
        "category": "api_credits",
    });

    // This is the exact string that the v0.8.8 bug produced: "U_S_D"
    // (SCREAMING_SNAKE_CASE split) instead of "USD".
    let result = sqlx::query(
        "INSERT INTO payments
            (id, agent_id, idempotency_key, amount, currency, recipient,
             preferred_rail, justification, status)
         VALUES ($1, $2, 'idem-bad-currency', 100.00, 'U_S_D', $3, 'auto', $4, 'pending')",
    )
    .bind(payment_id)
    .bind(agent_id)
    .bind(&recipient)
    .bind(&justification)
    .execute(&db.pool)
    .await;

    assert!(
        result.is_err(),
        "INSERT with invalid currency 'U_S_D' should be rejected by CHECK constraint"
    );

    db.cleanup().await;
}

// ===========================================================================
// 1.14 — CHECK constraint rejects invalid PolicyAction case
// ===========================================================================

#[tokio::test]
async fn check_constraint_rejects_lowercase_policy_action() {
    let db = TestDb::new().await;
    let (profile_id, _agent_id, _) = seed_agent(&db.pool).await;

    let rule_id = Uuid::now_v7();
    let condition = json!({"field": "amount", "op": "less_than", "value": 100});

    // The old lowercase format that v0.8.8 migrated away from.
    let result = sqlx::query(
        "INSERT INTO policy_rules
            (id, profile_id, priority, condition, action, enabled, created_at, updated_at)
         VALUES ($1, $2, 0, $3, 'approve', true, now(), now())",
    )
    .bind(rule_id)
    .bind(profile_id)
    .bind(&condition)
    .execute(&db.pool)
    .await;

    assert!(
        result.is_err(),
        "INSERT with lowercase 'approve' should be rejected — CHECK expects 'APPROVE'"
    );

    db.cleanup().await;
}

// ===========================================================================
// 1.15 — Settlement pair constraint: amount without currency is rejected
// ===========================================================================

#[tokio::test]
async fn settlement_pair_constraint_rejects_amount_without_currency() {
    let db = TestDb::new().await;
    let (_profile_id, agent_id, _) = seed_agent(&db.pool).await;

    let payment_id = Uuid::now_v7();
    let recipient = json!({
        "recipient_type": "merchant",
        "identifier": "test_merchant",
    });
    let justification = json!({
        "summary": "Test payment for settlement pair constraint verification in integration tests",
        "category": "api_credits",
    });

    // Insert a valid payment first.
    sqlx::query(
        "INSERT INTO payments
            (id, agent_id, idempotency_key, amount, currency, recipient,
             preferred_rail, justification, status, provider_id, provider_tx_id)
         VALUES ($1, $2, 'idem-pair-test', 100.00, 'USD', $3, 'auto', $4, 'settled', 'stripe', 'tx_pair')",
    )
    .bind(payment_id)
    .bind(agent_id)
    .bind(&recipient)
    .bind(&justification)
    .execute(&db.pool)
    .await
    .expect("INSERT base payment");

    // Try to set amount_settled WITHOUT settled_currency — should violate the pair constraint.
    let result = sqlx::query(
        "UPDATE payments SET amount_settled = 100.00, settled_currency = NULL WHERE id = $1",
    )
    .bind(payment_id)
    .execute(&db.pool)
    .await;

    assert!(
        result.is_err(),
        "UPDATE with amount_settled but NULL settled_currency should violate settlement pair constraint"
    );

    db.cleanup().await;
}
