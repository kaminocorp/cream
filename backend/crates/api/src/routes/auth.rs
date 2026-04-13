//! Operator authentication endpoints (Phase 16-B).
//!
//! - `GET /v1/auth/status` — is any operator registered?
//! - `POST /v1/auth/register` — first operator registration (blocked when operators exist)
//! - `POST /v1/auth/login` — email + password → access + refresh tokens
//! - `POST /v1/auth/refresh` — rotate refresh token, issue new access token
//! - `POST /v1/auth/logout` — revoke refresh token

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::ApiError;
use crate::extractors::auth::OperatorClaims;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MIN_PASSWORD_LEN: usize = 12;
const MAX_NAME_LEN: usize = 200;
const MAX_EMAIL_LEN: usize = 320; // RFC 5321

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub name: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub operator_id: String,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub registered: bool,
}

// ---------------------------------------------------------------------------
// GET /v1/auth/status
// ---------------------------------------------------------------------------

/// Returns whether any operators have been registered. No auth required.
/// Used by the frontend to decide whether to show registration or login.
pub async fn status(
    State(state): State<AppState>,
) -> Result<Json<StatusResponse>, ApiError> {
    let (count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM operators",
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(StatusResponse {
        registered: count > 0,
    }))
}

// ---------------------------------------------------------------------------
// POST /v1/auth/register
// ---------------------------------------------------------------------------

/// Register the first operator. Blocked once any operator exists.
/// Returns access + refresh tokens on success.
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), ApiError> {
    let jwt_secret = state.config.jwt_secret.as_deref().ok_or_else(|| {
        ApiError::Internal(anyhow::anyhow!(
            "JWT_SECRET must be configured to use operator auth"
        ))
    })?;

    // Validate inputs.
    let email = body.email.trim().to_lowercase();
    if email.is_empty() || email.len() > MAX_EMAIL_LEN || !email.contains('@') {
        return Err(ApiError::ValidationError("invalid email address".into()));
    }
    let name = body.name.trim().to_string();
    if name.is_empty() || name.len() > MAX_NAME_LEN {
        return Err(ApiError::ValidationError(format!(
            "name must be between 1 and {MAX_NAME_LEN} characters"
        )));
    }
    if body.password.len() < MIN_PASSWORD_LEN {
        return Err(ApiError::ValidationError(format!(
            "password must be at least {MIN_PASSWORD_LEN} characters"
        )));
    }

    // Check that no operators exist yet.
    let (count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM operators",
    )
    .fetch_one(&state.db)
    .await?;

    if count > 0 {
        return Err(ApiError::ValidationError(
            "operators already registered — use login or request an invite".into(),
        ));
    }

    // Hash password with argon2id.
    let password_hash = hash_password(&body.password)?;

    // Insert operator.
    let operator_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO operators (id, email, name, password_hash, role, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 'admin', 'active', now(), now())",
    )
    .bind(operator_id)
    .bind(&email)
    .bind(&name)
    .bind(&password_hash)
    .execute(&state.db)
    .await?;

    // Issue tokens.
    let operator_id_str = format!("opr_{}", operator_id);
    let access_token = issue_access_token(jwt_secret, &operator_id_str, &email, "admin", &state.config)?;
    let refresh_token = issue_refresh_token(&state.db, operator_id, &state.config).await?;

    // Update last_login_at.
    sqlx::query("UPDATE operators SET last_login_at = now() WHERE id = $1")
        .bind(operator_id)
        .execute(&state.db)
        .await?;

    tracing::info!(email = %email, "first operator registered");

    // Audit trail.
    log_auth_event(&state.db, Some(operator_id), "operator_registered", &email).await;

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            operator_id: operator_id_str,
            access_token,
            refresh_token,
        }),
    ))
}

// ---------------------------------------------------------------------------
// POST /v1/auth/login
// ---------------------------------------------------------------------------

/// Maximum login attempts per email within the rate limit window.
const MAX_LOGIN_ATTEMPTS: i64 = 5;
/// Rate limit window for login attempts in seconds (1 minute).
const LOGIN_RATE_LIMIT_WINDOW_SECS: i64 = 60;
/// Number of consecutive failures before the account is temporarily locked.
const LOCKOUT_THRESHOLD: i64 = 10;
/// Account lockout duration in seconds (15 minutes).
const LOCKOUT_DURATION_SECS: i64 = 900;

/// Authenticate with email + password, receive access + refresh tokens.
///
/// Rate limited to 5 attempts per email per minute via Redis. After 10
/// consecutive failures the account is temporarily locked for 15 minutes.
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let jwt_secret = state.config.jwt_secret.as_deref().ok_or_else(|| {
        ApiError::Internal(anyhow::anyhow!(
            "JWT_SECRET must be configured to use operator auth"
        ))
    })?;

    let email = body.email.trim().to_lowercase();

    // --- Rate limiting (Redis-backed) ---
    let rate_key = format!("cream:login_rate:{}", email);
    let lockout_key = format!("cream:login_lockout:{}", email);
    let fail_count_key = format!("cream:login_failures:{}", email);

    {
        let mut conn = state.redis.clone();

        // Check account lockout first.
        let locked: Option<String> = redis::cmd("GET")
            .arg(&lockout_key)
            .query_async(&mut conn)
            .await
            .unwrap_or(None);
        if locked.is_some() {
            tracing::warn!(email = %email, "login attempt on locked account");
            return Err(ApiError::RateLimited {
                retry_after_secs: LOCKOUT_DURATION_SECS as u64,
            });
        }

        // Check rate limit window.
        let count: i64 = redis::cmd("INCR")
            .arg(&rate_key)
            .query_async(&mut conn)
            .await
            .unwrap_or(1);
        if count == 1 {
            // First request in this window — set expiry.
            let _: () = redis::cmd("EXPIRE")
                .arg(&rate_key)
                .arg(LOGIN_RATE_LIMIT_WINDOW_SECS)
                .query_async(&mut conn)
                .await
                .unwrap_or(());
        }
        if count > MAX_LOGIN_ATTEMPTS {
            tracing::warn!(email = %email, count, "login rate limit exceeded");
            return Err(ApiError::RateLimited {
                retry_after_secs: LOGIN_RATE_LIMIT_WINDOW_SECS as u64,
            });
        }
    }

    // Look up operator by email.
    let row: Option<(Uuid, String, String, String)> = sqlx::query_as(
        "SELECT id, password_hash, role, status FROM operators WHERE email = $1",
    )
    .bind(&email)
    .fetch_optional(&state.db)
    .await?;

    let (operator_id, stored_hash, role, op_status) = row.ok_or(ApiError::Unauthorized)?;

    if op_status != "active" {
        return Err(ApiError::Unauthorized);
    }

    // Verify password.
    if !verify_password(&body.password, &stored_hash)? {
        // Increment consecutive failure counter for lockout.
        let mut conn = state.redis.clone();
        let failures: i64 = redis::cmd("INCR")
            .arg(&fail_count_key)
            .query_async(&mut conn)
            .await
            .unwrap_or(1);
        if failures == 1 {
            let _: () = redis::cmd("EXPIRE")
                .arg(&fail_count_key)
                .arg(LOCKOUT_DURATION_SECS)
                .query_async(&mut conn)
                .await
                .unwrap_or(());
        }
        if failures >= LOCKOUT_THRESHOLD {
            // Lock the account.
            let _: () = redis::cmd("SET")
                .arg(&lockout_key)
                .arg("1")
                .arg("EX")
                .arg(LOCKOUT_DURATION_SECS)
                .query_async(&mut conn)
                .await
                .unwrap_or(());
            tracing::warn!(email = %email, failures, "account locked after consecutive failures");
        }
        return Err(ApiError::Unauthorized);
    }

    // Login succeeded — clear failure counter.
    {
        let mut conn = state.redis.clone();
        let _: () = redis::cmd("DEL")
            .arg(&fail_count_key)
            .query_async(&mut conn)
            .await
            .unwrap_or(());
    }

    // Issue tokens.
    let operator_id_str = format!("opr_{}", operator_id);
    let access_token = issue_access_token(jwt_secret, &operator_id_str, &email, &role, &state.config)?;
    let refresh_token = issue_refresh_token(&state.db, operator_id, &state.config).await?;

    // Update last_login_at.
    sqlx::query("UPDATE operators SET last_login_at = now() WHERE id = $1")
        .bind(operator_id)
        .execute(&state.db)
        .await?;

    // Audit trail.
    log_auth_event(&state.db, Some(operator_id), "operator_login", &email).await;

    Ok(Json(AuthResponse {
        operator_id: operator_id_str,
        access_token,
        refresh_token,
    }))
}

// ---------------------------------------------------------------------------
// POST /v1/auth/refresh
// ---------------------------------------------------------------------------

/// Rotate a refresh token: verify the old one, issue new access + refresh tokens,
/// revoke the old refresh token. If the old token is already revoked (reuse
/// detection), revoke ALL sessions for that operator (stolen token scenario).
pub async fn refresh(
    State(state): State<AppState>,
    Json(body): Json<RefreshRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let jwt_secret = state.config.jwt_secret.as_deref().ok_or_else(|| {
        ApiError::Internal(anyhow::anyhow!(
            "JWT_SECRET must be configured to use operator auth"
        ))
    })?;

    let token_hash = hex::encode(Sha256::digest(body.refresh_token.as_bytes()));

    // Look up the session by refresh token hash.
    let row: Option<SessionRow> = sqlx::query_as(
        "SELECT id, operator_id, expires_at, revoked_at
         FROM operator_sessions
         WHERE refresh_token_hash = $1",
    )
    .bind(&token_hash)
    .fetch_optional(&state.db)
    .await?;

    let session = row.ok_or(ApiError::Unauthorized)?;
    let session_id = session.id;
    let operator_id = session.operator_id;
    let expires_at = session.expires_at;
    let revoked_at = session.revoked_at;

    // Reuse detection: if the token was already revoked, someone may have
    // stolen it. Revoke ALL sessions for this operator as a precaution.
    if revoked_at.is_some() {
        tracing::warn!(
            operator_id = %operator_id,
            session_id = %session_id,
            "refresh token reuse detected — revoking all sessions"
        );
        sqlx::query(
            "UPDATE operator_sessions SET revoked_at = now()
             WHERE operator_id = $1 AND revoked_at IS NULL",
        )
        .bind(operator_id)
        .execute(&state.db)
        .await?;

        // Audit: this is a security-relevant event — potential token theft.
        log_auth_event(&state.db, Some(operator_id), "refresh_token_reuse_detected", "all sessions revoked").await;

        return Err(ApiError::Unauthorized);
    }

    // Check expiry.
    if expires_at < Utc::now() {
        return Err(ApiError::Unauthorized);
    }

    // Revoke the old session.
    sqlx::query(
        "UPDATE operator_sessions SET revoked_at = now() WHERE id = $1",
    )
    .bind(session_id)
    .execute(&state.db)
    .await?;

    // Look up operator details for the new access token.
    let (email, role, op_status): (String, String, String) = sqlx::query_as(
        "SELECT email, role, status FROM operators WHERE id = $1",
    )
    .bind(operator_id)
    .fetch_one(&state.db)
    .await?;

    if op_status != "active" {
        return Err(ApiError::Unauthorized);
    }

    // Issue new tokens.
    let operator_id_str = format!("opr_{}", operator_id);
    let access_token = issue_access_token(jwt_secret, &operator_id_str, &email, &role, &state.config)?;
    let refresh_token = issue_refresh_token(&state.db, operator_id, &state.config).await?;

    Ok(Json(AuthResponse {
        operator_id: operator_id_str,
        access_token,
        refresh_token,
    }))
}

// ---------------------------------------------------------------------------
// POST /v1/auth/logout
// ---------------------------------------------------------------------------

/// Revoke the refresh token, ending the session.
pub async fn logout(
    State(state): State<AppState>,
    Json(body): Json<RefreshRequest>,
) -> Result<StatusCode, ApiError> {
    let token_hash = hex::encode(Sha256::digest(body.refresh_token.as_bytes()));

    // Look up the operator_id before revoking (for audit).
    let session_operator: Option<(Uuid,)> = sqlx::query_as(
        "SELECT operator_id FROM operator_sessions WHERE refresh_token_hash = $1",
    )
    .bind(&token_hash)
    .fetch_optional(&state.db)
    .await?;

    sqlx::query(
        "UPDATE operator_sessions SET revoked_at = now()
         WHERE refresh_token_hash = $1 AND revoked_at IS NULL",
    )
    .bind(&token_hash)
    .execute(&state.db)
    .await?;

    // Audit trail.
    if let Some((op_id,)) = session_operator {
        log_auth_event(&state.db, Some(op_id), "operator_logout", "session revoked").await;
    }

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Auth audit logging
// ---------------------------------------------------------------------------

/// Best-effort write of an authentication event to the `operator_events` table.
/// Never fails the caller — swallows DB errors with a warning log.
async fn log_auth_event(db: &sqlx::PgPool, operator_id: Option<Uuid>, event_type: &str, detail: &str) {
    let details = serde_json::json!({
        "operator_id": operator_id.map(|id| format!("opr_{id}")),
        "detail": detail,
    });
    if let Err(e) = sqlx::query(
        "INSERT INTO operator_events (event_type, details) VALUES ($1, $2)",
    )
    .bind(event_type)
    .bind(&details)
    .execute(db)
    .await
    {
        tracing::warn!(event_type, error = %e, "failed to log auth event (non-blocking)");
    }
}

// ---------------------------------------------------------------------------
// Password hashing
// ---------------------------------------------------------------------------

fn hash_password(password: &str) -> Result<String, ApiError> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("password hash failed: {e}")))?;

    Ok(hash.to_string())
}

fn verify_password(password: &str, stored_hash: &str) -> Result<bool, ApiError> {
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };

    let parsed_hash = PasswordHash::new(stored_hash)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("parse password hash: {e}")))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

// ---------------------------------------------------------------------------
// Token issuance
// ---------------------------------------------------------------------------

fn issue_access_token(
    secret: &str,
    operator_id: &str,
    email: &str,
    role: &str,
    config: &crate::config::AppConfig,
) -> Result<String, ApiError> {
    let now = Utc::now().timestamp();
    let claims = OperatorClaims {
        sub: operator_id.to_string(),
        email: email.to_string(),
        role: role.to_string(),
        iat: now,
        exp: now + config.jwt_access_ttl_secs,
    };

    jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("JWT encode failed: {e}")))
}

async fn issue_refresh_token(
    db: &sqlx::PgPool,
    operator_id: Uuid,
    config: &crate::config::AppConfig,
) -> Result<String, ApiError> {
    // Generate a high-entropy refresh token (UUIDv7 = 122 bits of randomness).
    // SHA-256 is intentionally used instead of Argon2 because:
    // 1. The token has high entropy — brute-forcing 2^122 possibilities is infeasible
    //    even with fast hashing. Argon2 is for low-entropy passwords, not random tokens.
    // 2. We need deterministic lookup: `WHERE refresh_token_hash = $1`. Argon2's per-hash
    //    salt makes DB lookups impossible without loading all rows.
    // This matches the industry standard (GitHub, Stripe, AWS all SHA-256 hash API tokens).
    let raw_token = format!("crrt_{}", Uuid::now_v7());
    let token_hash = hex::encode(Sha256::digest(raw_token.as_bytes()));
    let expires_at = Utc::now() + chrono::Duration::seconds(config.jwt_refresh_ttl_secs);

    sqlx::query(
        "INSERT INTO operator_sessions (id, operator_id, refresh_token_hash, expires_at, created_at)
         VALUES ($1, $2, $3, $4, now())",
    )
    .bind(Uuid::now_v7())
    .bind(operator_id)
    .bind(&token_hash)
    .bind(expires_at)
    .execute(db)
    .await?;

    Ok(raw_token)
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct SessionRow {
    id: Uuid,
    operator_id: Uuid,
    expires_at: chrono::DateTime<Utc>,
    revoked_at: Option<chrono::DateTime<Utc>>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_hash_roundtrip() {
        let password = "secure_password_12345";
        let hash = hash_password(password).unwrap();
        assert!(verify_password(password, &hash).unwrap());
    }

    #[test]
    fn password_hash_wrong_password() {
        let hash = hash_password("correct_password_123").unwrap();
        assert!(!verify_password("wrong_password_1234", &hash).unwrap());
    }

    #[test]
    fn password_hash_uses_argon2id() {
        let hash = hash_password("test_password_1234").unwrap();
        assert!(hash.starts_with("$argon2id$"));
    }

    #[test]
    fn password_hash_unique_salts() {
        let h1 = hash_password("same_password_12345").unwrap();
        let h2 = hash_password("same_password_12345").unwrap();
        assert_ne!(h1, h2, "same password should produce different hashes due to unique salts");
    }

    #[test]
    fn issue_access_token_produces_valid_jwt() {
        let config = crate::config::AppConfig {
            database_url: String::new(),
            redis_url: String::new(),
            host: String::new(),
            port: 0,
            rate_limit_requests: 0,
            rate_limit_window_secs: 0,
            escalation_check_interval_secs: 0,
            cors_allowed_origins: vec![],
            operator_api_key: None,
            webhook_delivery_timeout_secs: 10,
            webhook_max_retries: 5,
            jwt_secret: Some("a".repeat(32)),
            jwt_access_ttl_secs: 900,
            jwt_refresh_ttl_secs: 604800, slack_bot_token: None, slack_channel_id: None, slack_signing_secret: None, smtp_host: None, smtp_port: 587, smtp_username: None, smtp_password: None, email_from: None, escalation_email_to: None, resend_api_key: None, dashboard_base_url: None, provider_key_encryption_secret: None,
        };

        let token = issue_access_token(
            config.jwt_secret.as_deref().unwrap(),
            "opr_01234567-89ab-cdef-0123-456789abcdef",
            "test@example.com",
            "admin",
            &config,
        )
        .unwrap();

        // Decode and verify.
        let decoding_key = jsonwebtoken::DecodingKey::from_secret(config.jwt_secret.as_deref().unwrap().as_bytes());
        let mut validation = jsonwebtoken::Validation::default();
        validation.set_required_spec_claims(&["sub", "exp", "iat"]);

        let decoded = jsonwebtoken::decode::<OperatorClaims>(&token, &decoding_key, &validation).unwrap();
        assert_eq!(decoded.claims.sub, "opr_01234567-89ab-cdef-0123-456789abcdef");
        assert_eq!(decoded.claims.email, "test@example.com");
        assert_eq!(decoded.claims.role, "admin");
    }

    #[test]
    fn issue_access_token_wrong_secret_fails() {
        let config = crate::config::AppConfig {
            database_url: String::new(),
            redis_url: String::new(),
            host: String::new(),
            port: 0,
            rate_limit_requests: 0,
            rate_limit_window_secs: 0,
            escalation_check_interval_secs: 0,
            cors_allowed_origins: vec![],
            operator_api_key: None,
            webhook_delivery_timeout_secs: 10,
            webhook_max_retries: 5,
            jwt_secret: Some("a".repeat(32)),
            jwt_access_ttl_secs: 900,
            jwt_refresh_ttl_secs: 604800, slack_bot_token: None, slack_channel_id: None, slack_signing_secret: None, smtp_host: None, smtp_port: 587, smtp_username: None, smtp_password: None, email_from: None, escalation_email_to: None, resend_api_key: None, dashboard_base_url: None, provider_key_encryption_secret: None,
        };

        let token = issue_access_token(
            config.jwt_secret.as_deref().unwrap(),
            "opr_test",
            "test@example.com",
            "admin",
            &config,
        )
        .unwrap();

        let wrong_key = jsonwebtoken::DecodingKey::from_secret(b"wrong_secret_that_is_long_enough");
        let mut validation = jsonwebtoken::Validation::default();
        validation.set_required_spec_claims(&["sub", "exp", "iat"]);

        let result = jsonwebtoken::decode::<OperatorClaims>(&token, &wrong_key, &validation);
        assert!(result.is_err());
    }

    #[test]
    fn issue_access_token_expired_fails() {
        let config = crate::config::AppConfig {
            database_url: String::new(),
            redis_url: String::new(),
            host: String::new(),
            port: 0,
            rate_limit_requests: 0,
            rate_limit_window_secs: 0,
            escalation_check_interval_secs: 0,
            cors_allowed_origins: vec![],
            operator_api_key: None,
            webhook_delivery_timeout_secs: 10,
            webhook_max_retries: 5,
            jwt_secret: Some("a".repeat(32)),
            jwt_access_ttl_secs: -120, // expired 2 minutes ago (exceeds default 60s leeway)
            jwt_refresh_ttl_secs: 604800, slack_bot_token: None, slack_channel_id: None, slack_signing_secret: None, smtp_host: None, smtp_port: 587, smtp_username: None, smtp_password: None, email_from: None, escalation_email_to: None, resend_api_key: None, dashboard_base_url: None, provider_key_encryption_secret: None,
        };

        let token = issue_access_token(
            config.jwt_secret.as_deref().unwrap(),
            "opr_test",
            "test@example.com",
            "admin",
            &config,
        )
        .unwrap();

        let key = jsonwebtoken::DecodingKey::from_secret(config.jwt_secret.as_deref().unwrap().as_bytes());
        let mut validation = jsonwebtoken::Validation::default();
        validation.set_required_spec_claims(&["sub", "exp", "iat"]);

        let result = jsonwebtoken::decode::<OperatorClaims>(&token, &key, &validation);
        assert!(result.is_err());
    }

    #[test]
    fn min_password_length() {
        assert_eq!(MIN_PASSWORD_LEN, 12);
    }
}
