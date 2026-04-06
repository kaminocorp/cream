// Shared test utilities for integration tests.
//
// Provides a `TestDb` that creates a uniquely-named Postgres database per test,
// runs all migrations, yields a PgPool, and drops the database on cleanup.

use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor, PgPool};
use uuid::Uuid;

/// A disposable test database.
///
/// Each `TestDb` creates a fresh Postgres database with a unique name,
/// runs all migrations, and provides a `PgPool` connected to it.
/// The database is dropped when `cleanup()` is called.
pub struct TestDb {
    /// Connection pool pointed at the test database.
    pub pool: PgPool,
    /// The unique database name (for cleanup).
    db_name: String,
    /// Admin connection URL (pointed at `postgres` database) used for
    /// CREATE/DROP DATABASE.
    admin_url: String,
}

impl TestDb {
    /// Create a new test database, run all migrations, and return a pool.
    ///
    /// Uses `DATABASE_URL` from the environment if set, otherwise falls back
    /// to a localhost connection using the current OS user (standard Homebrew
    /// Postgres setup).
    pub async fn new() -> Self {
        let base_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://localhost:5432".to_string()
        });

        // Parse the base URL to extract the host/port/user, then connect to
        // the `postgres` admin database for CREATE/DROP.
        let admin_url = if base_url.contains("/cream") || base_url.ends_with('/') {
            let idx = base_url.rfind('/').unwrap();
            format!("{}/postgres", &base_url[..idx])
        } else {
            format!("{}/postgres", base_url.trim_end_matches('/'))
        };

        let db_name = format!("cream_test_{}", Uuid::new_v4().simple());

        let admin_pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(&admin_url)
            .await
            .expect("failed to connect to admin database — is Postgres running?");

        admin_pool
            .execute(format!(r#"CREATE DATABASE "{}""#, db_name).as_str())
            .await
            .unwrap_or_else(|e| panic!("failed to create test database {db_name}: {e}"));

        admin_pool.close().await;

        let test_url = admin_url.replace("/postgres", &format!("/{db_name}"));

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&test_url)
            .await
            .unwrap_or_else(|e| panic!("failed to connect to test database {db_name}: {e}"));

        // Run all migrations — path relative to the crate root (crates/api/).
        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .unwrap_or_else(|e| panic!("migrations failed on {db_name}: {e}"));

        Self {
            pool,
            db_name,
            admin_url,
        }
    }

    /// Explicitly clean up the test database.
    pub async fn cleanup(self) {
        self.pool.close().await;

        let admin_pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(&self.admin_url)
            .await
            .expect("failed to reconnect to admin database for cleanup");

        let _ = admin_pool
            .execute(
                format!(
                    r#"SELECT pg_terminate_backend(pid)
                       FROM pg_stat_activity
                       WHERE datname = '{}' AND pid <> pg_backend_pid()"#,
                    self.db_name
                )
                .as_str(),
            )
            .await;

        let _ = admin_pool
            .execute(format!(r#"DROP DATABASE IF EXISTS "{}""#, self.db_name).as_str())
            .await;

        admin_pool.close().await;
    }
}

/// Seed an agent profile + agent into the test database.
/// Returns (profile_id, agent_id).
pub async fn seed_agent(pool: &PgPool) -> (uuid::Uuid, uuid::Uuid) {
    let profile_id = Uuid::now_v7();
    let agent_id = Uuid::now_v7();
    let api_key_hash = "sha256_test_key_hash_for_integration_tests";

    sqlx::query(
        "INSERT INTO agent_profiles
            (id, name, max_per_transaction, max_daily_spend, max_weekly_spend, max_monthly_spend, created_at, updated_at)
         VALUES ($1, $2, 10000, 50000, 200000, 500000, now(), now())",
    )
    .bind(profile_id)
    .bind("test-profile")
    .execute(pool)
    .await
    .expect("seed agent_profiles");

    sqlx::query(
        "INSERT INTO agents (id, profile_id, name, api_key_hash, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 'active', now(), now())",
    )
    .bind(agent_id)
    .bind(profile_id)
    .bind("test-agent")
    .bind(api_key_hash)
    .execute(pool)
    .await
    .expect("seed agents");

    (profile_id, agent_id)
}
