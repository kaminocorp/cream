//! # Phase 15.1 — Integration tests for operator-scoped endpoints
//!
//! These tests exercise the SQL paths added by Phase 15.1 against a real
//! Postgres instance using the shared `TestDb` harness. They cover:
//!
//!   - Agent lifecycle SQL (list/create/update/rotate-key) in the shape the
//!     Axum handlers execute.
//!   - Cross-agent visibility on the `PgAuditReader` when no `agent_id` is
//!     bound (operator mode).
//!   - Free-text `q` search via ILIKE on `justification.summary`, including
//!     metacharacter-escaping.
//!
//! What these tests deliberately do NOT cover: full HTTP-level auth chains
//! (bearer header → extractor → handler → response), because constructing
//! a full `AppState` requires a running Redis + provider registry + policy
//! engine. Those gaps are caught by (a) the focused unit tests in
//! `config::tests` and `routes::agents::tests` for the new code paths and
//! (b) the end-to-end runtime smoke testing that will happen in Phase 15.2
//! when the frontend starts calling these endpoints for real.

mod common;

use common::{seed_agent, TestDb};
use cream_audit::{AuditQuery, AuditReader, PgAuditReader};
use cream_models::prelude::*;
use rust_decimal::Decimal;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::str::FromStr;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Seed a second agent distinct from the one `seed_agent` creates. Returns
/// (profile_id, agent_id). Uses a different api_key_hash so the two agents
/// can coexist if a UNIQUE constraint is ever added later.
async fn seed_second_agent(pool: &sqlx::PgPool) -> (Uuid, Uuid) {
    let profile_id = Uuid::now_v7();
    let agent_id = Uuid::now_v7();
    let api_key_hash = "sha256_second_test_key_hash";

    sqlx::query(
        "INSERT INTO agent_profiles
            (id, name, max_per_transaction, max_daily_spend, max_weekly_spend, max_monthly_spend, created_at, updated_at)
         VALUES ($1, $2, 10000, 50000, 200000, 500000, now(), now())",
    )
    .bind(profile_id)
    .bind("second-test-profile")
    .execute(pool)
    .await
    .expect("seed second agent_profiles");

    sqlx::query(
        "INSERT INTO agents (id, profile_id, name, api_key_hash, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 'active', now(), now())",
    )
    .bind(agent_id)
    .bind(profile_id)
    .bind("second-test-agent")
    .bind(api_key_hash)
    .execute(pool)
    .await
    .expect("seed second agent");

    (profile_id, agent_id)
}

/// Insert a minimal audit entry for a specific agent with a given summary.
/// Mirrors the JSON shape the handler would write.
async fn insert_audit_entry(
    pool: &sqlx::PgPool,
    agent_id: Uuid,
    profile_id: Uuid,
    summary: &str,
) -> Uuid {
    let entry_id = Uuid::now_v7();
    let policy_eval = json!({
        "rules_evaluated": [],
        "matching_rules": [],
        "final_decision": "APPROVE",
        "decision_latency_ms": 5
    });
    let request = json!({
        "amount": "100.00",
        "currency": "sgd",
        "recipient": {"recipient_type": "merchant", "identifier": "test_m"}
    });
    let justification = json!({
        "summary": summary,
        "category": "api_credits"
    });

    sqlx::query(
        "INSERT INTO audit_log
            (id, timestamp, agent_id, agent_profile_id, payment_id,
             request, justification, policy_evaluation, final_status)
         VALUES ($1, now(), $2, $3, NULL, $4, $5, $6, 'settled')",
    )
    .bind(entry_id)
    .bind(agent_id)
    .bind(profile_id)
    .bind(&request)
    .bind(&justification)
    .bind(&policy_eval)
    .execute(pool)
    .await
    .expect("insert audit entry");

    entry_id
}

// ===========================================================================
// Agent lifecycle SQL
// ===========================================================================

#[tokio::test]
async fn list_agents_sql_returns_rows_with_profile_name() {
    let db = TestDb::new().await;
    let (profile_id_1, agent_id_1) = seed_agent(&db.pool).await;
    let (profile_id_2, agent_id_2) = seed_second_agent(&db.pool).await;

    // Query the same SQL the `list_agents` handler runs. `name` is selected
    // but not asserted on in this test — the JOIN shape is what we're
    // verifying.
    #[derive(sqlx::FromRow)]
    #[allow(dead_code)]
    struct Row {
        id: Uuid,
        profile_id: Uuid,
        profile_name: String,
        name: String,
        status: String,
    }

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT a.id, a.profile_id, p.name AS profile_name, a.name, a.status,
                a.created_at, a.updated_at
         FROM agents a
         JOIN agent_profiles p ON p.id = a.profile_id
         ORDER BY a.created_at DESC
         LIMIT 500",
    )
    .fetch_all(&db.pool)
    .await
    .expect("list query");

    assert_eq!(rows.len(), 2);
    let by_id: std::collections::HashMap<Uuid, Row> =
        rows.into_iter().map(|r| (r.id, r)).collect();

    let first = by_id.get(&agent_id_1).expect("first agent present");
    assert_eq!(first.profile_id, profile_id_1);
    assert_eq!(first.profile_name, "test-profile");
    assert_eq!(first.status, "active");

    let second = by_id.get(&agent_id_2).expect("second agent present");
    assert_eq!(second.profile_id, profile_id_2);
    assert_eq!(second.profile_name, "second-test-profile");

    db.cleanup().await;
}

#[tokio::test]
async fn create_agent_sql_inserts_with_hashed_key() {
    let db = TestDb::new().await;
    let (profile_id, _) = seed_agent(&db.pool).await;

    // Simulate what the handler does: generate key, hash it, insert.
    let plaintext = format!("cream_{}", hex::encode([0x42u8; 32]));
    let hash = hex::encode(Sha256::digest(plaintext.as_bytes()));
    let new_agent_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO agents (id, profile_id, name, api_key_hash, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 'active', now(), now())",
    )
    .bind(new_agent_id)
    .bind(profile_id)
    .bind("fresh-agent")
    .bind(&hash)
    .execute(&db.pool)
    .await
    .expect("insert new agent");

    // The plaintext must not be retrievable from the DB — only the hash.
    let row: (String, String, String) = sqlx::query_as(
        "SELECT name, status, api_key_hash FROM agents WHERE id = $1",
    )
    .bind(new_agent_id)
    .fetch_one(&db.pool)
    .await
    .expect("fetch inserted agent");

    assert_eq!(row.0, "fresh-agent");
    assert_eq!(row.1, "active");
    assert_eq!(row.2, hash);
    // The plaintext must never appear anywhere.
    assert_ne!(row.2, plaintext);
    assert_eq!(row.2.len(), 64); // SHA-256 hex = 64 chars

    db.cleanup().await;
}

#[tokio::test]
async fn rotate_key_sql_updates_hash_and_invalidates_old() {
    let db = TestDb::new().await;
    let (_, agent_id) = seed_agent(&db.pool).await;

    // Fetch the initial hash.
    let (old_hash,): (String,) =
        sqlx::query_as("SELECT api_key_hash FROM agents WHERE id = $1")
            .bind(agent_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();

    // Rotate: new plaintext + hash, then UPDATE.
    let new_plaintext = format!("cream_{}", hex::encode([0x99u8; 32]));
    let new_hash = hex::encode(Sha256::digest(new_plaintext.as_bytes()));

    let rows_affected =
        sqlx::query("UPDATE agents SET api_key_hash = $1, updated_at = now() WHERE id = $2")
            .bind(&new_hash)
            .bind(agent_id)
            .execute(&db.pool)
            .await
            .unwrap()
            .rows_affected();
    assert_eq!(rows_affected, 1);

    // Old hash is gone; new hash is in place.
    let (current_hash,): (String,) =
        sqlx::query_as("SELECT api_key_hash FROM agents WHERE id = $1")
            .bind(agent_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(current_hash, new_hash);
    assert_ne!(current_hash, old_hash);

    // A lookup via the old hash returns zero rows (matches the
    // `lookup_agent_by_key_hash` failure case).
    let remaining: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM agents WHERE api_key_hash = $1 AND status = 'active'")
            .bind(&old_hash)
            .fetch_optional(&db.pool)
            .await
            .unwrap();
    assert!(remaining.is_none());

    db.cleanup().await;
}

#[tokio::test]
async fn update_agent_sql_partial_fields() {
    let db = TestDb::new().await;
    let (profile_id_1, agent_id) = seed_agent(&db.pool).await;
    let (profile_id_2, _) = seed_second_agent(&db.pool).await;

    // Same SQL the `update_agent` handler runs: COALESCE each optional field.
    // Case 1: update name only.
    sqlx::query(
        "UPDATE agents SET
            name = COALESCE($1, name),
            status = COALESCE($2, status),
            profile_id = COALESCE($3, profile_id),
            updated_at = now()
         WHERE id = $4",
    )
    .bind(Some("renamed"))
    .bind(None::<&str>)
    .bind(None::<Uuid>)
    .bind(agent_id)
    .execute(&db.pool)
    .await
    .unwrap();

    let (name, status, pid): (String, String, Uuid) = sqlx::query_as(
        "SELECT name, status, profile_id FROM agents WHERE id = $1",
    )
    .bind(agent_id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(name, "renamed");
    assert_eq!(status, "active");
    assert_eq!(pid, profile_id_1);

    // Case 2: update status + profile, leave name.
    sqlx::query(
        "UPDATE agents SET
            name = COALESCE($1, name),
            status = COALESCE($2, status),
            profile_id = COALESCE($3, profile_id),
            updated_at = now()
         WHERE id = $4",
    )
    .bind(None::<&str>)
    .bind(Some("suspended"))
    .bind(Some(profile_id_2))
    .bind(agent_id)
    .execute(&db.pool)
    .await
    .unwrap();

    let (name, status, pid): (String, String, Uuid) = sqlx::query_as(
        "SELECT name, status, profile_id FROM agents WHERE id = $1",
    )
    .bind(agent_id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(name, "renamed"); // unchanged
    assert_eq!(status, "suspended");
    assert_eq!(pid, profile_id_2);

    db.cleanup().await;
}

// ===========================================================================
// Audit reader — cross-agent visibility (operator path)
// ===========================================================================

#[tokio::test]
async fn audit_query_without_agent_id_returns_all_agents() {
    let db = TestDb::new().await;
    let (profile_id_1, agent_id_1) = seed_agent(&db.pool).await;
    let (profile_id_2, agent_id_2) = seed_second_agent(&db.pool).await;

    // Insert one entry for each agent.
    insert_audit_entry(&db.pool, agent_id_1, profile_id_1, "agent one paid API credits").await;
    insert_audit_entry(&db.pool, agent_id_2, profile_id_2, "agent two paid API credits").await;

    let reader = PgAuditReader::new(db.pool.clone());

    // Operator mode: no agent_id filter — should return both.
    let query = AuditQuery::new();
    let entries = reader.query(query).await.expect("query all");
    assert_eq!(entries.len(), 2);

    let seen_agents: std::collections::HashSet<_> =
        entries.iter().map(|e| *e.agent_id.as_uuid()).collect();
    assert!(seen_agents.contains(&agent_id_1));
    assert!(seen_agents.contains(&agent_id_2));

    // Agent mode: with agent_id filter — should return only that agent's entries.
    let query = AuditQuery::new().agent_id(AgentId::from_uuid(agent_id_1));
    let entries = reader.query(query).await.expect("query scoped");
    assert_eq!(entries.len(), 1);
    assert_eq!(*entries[0].agent_id.as_uuid(), agent_id_1);

    db.cleanup().await;
}

// ===========================================================================
// Audit reader — free-text q search
// ===========================================================================

#[tokio::test]
async fn audit_q_search_finds_substring_match_case_insensitive() {
    let db = TestDb::new().await;
    let (profile_id, agent_id) = seed_agent(&db.pool).await;

    insert_audit_entry(&db.pool, agent_id, profile_id, "Purchased OpenAI API credits for task #4421").await;
    insert_audit_entry(&db.pool, agent_id, profile_id, "Anthropic monthly subscription renewal").await;
    insert_audit_entry(&db.pool, agent_id, profile_id, "AWS S3 bucket storage fees").await;

    let reader = PgAuditReader::new(db.pool.clone());

    // Case-insensitive: "openai" should match "OpenAI"
    let entries = reader
        .query(AuditQuery::new().q("openai"))
        .await
        .expect("q query");
    assert_eq!(entries.len(), 1);
    assert!(
        entries[0].justification["summary"]
            .as_str()
            .unwrap()
            .contains("OpenAI")
    );

    // Substring in the middle
    let entries = reader
        .query(AuditQuery::new().q("subscription"))
        .await
        .unwrap();
    assert_eq!(entries.len(), 1);

    // No match
    let entries = reader
        .query(AuditQuery::new().q("grafana"))
        .await
        .unwrap();
    assert!(entries.is_empty());

    db.cleanup().await;
}

#[tokio::test]
async fn audit_q_search_escapes_ilike_metacharacters() {
    let db = TestDb::new().await;
    let (profile_id, agent_id) = seed_agent(&db.pool).await;

    // Insert a literal `%` character in the summary.
    insert_audit_entry(&db.pool, agent_id, profile_id, "discount 50% off for beta tier").await;
    insert_audit_entry(&db.pool, agent_id, profile_id, "plain summary with no percent").await;

    let reader = PgAuditReader::new(db.pool.clone());

    // Searching for `50%` should match only the first entry. If metacharacter
    // escaping were missing, `%` would be interpreted as "match anything" and
    // both rows would come back.
    let entries = reader.query(AuditQuery::new().q("50%")).await.unwrap();
    assert_eq!(
        entries.len(),
        1,
        "metacharacter-escaped search should match literal '%'"
    );
    assert!(entries[0].justification["summary"]
        .as_str()
        .unwrap()
        .contains("50%"));

    db.cleanup().await;
}

#[tokio::test]
async fn audit_q_search_respects_agent_id_scope() {
    let db = TestDb::new().await;
    let (profile_id_1, agent_id_1) = seed_agent(&db.pool).await;
    let (profile_id_2, agent_id_2) = seed_second_agent(&db.pool).await;

    // Both agents have an entry mentioning "stripe".
    insert_audit_entry(&db.pool, agent_id_1, profile_id_1, "stripe subscription for agent 1").await;
    insert_audit_entry(&db.pool, agent_id_2, profile_id_2, "stripe subscription for agent 2").await;

    let reader = PgAuditReader::new(db.pool.clone());

    // Operator view: q only → both match.
    let entries = reader.query(AuditQuery::new().q("stripe")).await.unwrap();
    assert_eq!(entries.len(), 2);

    // Agent view: q + agent_id → only one.
    let entries = reader
        .query(
            AuditQuery::new()
                .q("stripe")
                .agent_id(AgentId::from_uuid(agent_id_1)),
        )
        .await
        .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(*entries[0].agent_id.as_uuid(), agent_id_1);

    db.cleanup().await;
}

// ===========================================================================
// Smoke: AuditQuery q combined with other filters (status, amount)
// ===========================================================================

#[tokio::test]
async fn audit_q_combines_with_other_filters() {
    let db = TestDb::new().await;
    let (profile_id, agent_id) = seed_agent(&db.pool).await;

    // Two entries with "flight", different amounts.
    sqlx::query(
        "INSERT INTO audit_log
            (id, timestamp, agent_id, agent_profile_id, payment_id,
             request, justification, policy_evaluation, final_status)
         VALUES
            ($1, now(), $2, $3, NULL,
             $4::jsonb, $5::jsonb, $6::jsonb, 'settled'),
            ($7, now(), $2, $3, NULL,
             $8::jsonb, $9::jsonb, $6::jsonb, 'settled')",
    )
    .bind(Uuid::now_v7())
    .bind(agent_id)
    .bind(profile_id)
    .bind(json!({"amount": "100.00", "currency": "sgd", "recipient": {"recipient_type": "merchant", "identifier": "m"}}))
    .bind(json!({"summary": "cheap domestic flight", "category": "api_credits"}))
    .bind(json!({"rules_evaluated": [], "matching_rules": [], "final_decision": "APPROVE", "decision_latency_ms": 1}))
    .bind(Uuid::now_v7())
    .bind(json!({"amount": "2000.00", "currency": "sgd", "recipient": {"recipient_type": "merchant", "identifier": "m"}}))
    .bind(json!({"summary": "expensive international flight", "category": "api_credits"}))
    .execute(&db.pool)
    .await
    .unwrap();

    let reader = PgAuditReader::new(db.pool.clone());

    // q="flight" should find both.
    let both = reader.query(AuditQuery::new().q("flight")).await.unwrap();
    assert_eq!(both.len(), 2);

    // q="flight" + min_amount=1000 → only the expensive one.
    let filtered = reader
        .query(
            AuditQuery::new()
                .q("flight")
                .min_amount(Decimal::from_str("1000").unwrap()),
        )
        .await
        .unwrap();
    assert_eq!(filtered.len(), 1);
    assert!(filtered[0].justification["summary"]
        .as_str()
        .unwrap()
        .contains("expensive"));

    db.cleanup().await;
}
