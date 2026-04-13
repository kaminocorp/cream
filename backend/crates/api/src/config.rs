use std::env;

/// Minimum allowed length for the operator API key. Chosen so that a plausibly
/// generated key (32 hex chars = 128 bits of entropy) is the floor; shorter
/// values are refused at config load. Callers who ignore this and set something
/// trivial like `OPERATOR_API_KEY=test` will fail fast rather than discover it
/// by watching unauthenticated callers get admin access.
pub const MIN_OPERATOR_KEY_LEN: usize = 32;

/// Application configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub redis_url: String,
    pub host: String,
    pub port: u16,
    /// Maximum requests per agent per rate-limit window.
    pub rate_limit_requests: u64,
    /// Rate-limit window duration in seconds.
    pub rate_limit_window_secs: u64,
    /// How often (seconds) the escalation timeout monitor checks for expired approvals.
    pub escalation_check_interval_secs: u64,
    /// Comma-separated list of allowed CORS origins (e.g. "https://dashboard.example.com").
    /// If empty or unset, the server will refuse to start unless
    /// `ALLOW_PERMISSIVE_CORS=true` is also set (development only).
    pub cors_allowed_origins: Vec<String>,
    /// Shared secret that, when presented as `Authorization: Bearer <key>`,
    /// authenticates the caller as the operator (dashboard / admin).
    ///
    /// Phase 15.1 interim design: a single shared key loaded from
    /// `OPERATOR_API_KEY`. Phase 16-B replaces this with real per-user auth
    /// tied to an `operators` table; at that point this field becomes a
    /// legacy fallback that 16-B can delete.
    ///
    /// Left unset = no operator access. The dashboard will be unable to
    /// reach operator-only endpoints (403), which is the correct behavior
    /// for deployments where operator auth has not been configured.
    pub operator_api_key: Option<String>,
    /// Timeout (seconds) for outbound webhook HTTP requests.
    pub webhook_delivery_timeout_secs: u64,
    /// Maximum delivery attempts per webhook event (including initial attempt).
    pub webhook_max_retries: u16,
    /// HMAC secret for signing/verifying JWT access tokens. Must be at least
    /// 32 characters. If unset, JWT auth is disabled and only the legacy
    /// `OPERATOR_API_KEY` works.
    pub jwt_secret: Option<String>,
    /// Access token lifetime in seconds (default: 900 = 15 minutes).
    pub jwt_access_ttl_secs: i64,
    /// Refresh token lifetime in seconds (default: 604800 = 7 days).
    pub jwt_refresh_ttl_secs: i64,
    /// Slack bot OAuth token (xoxb-...). If unset, Slack notifications are disabled.
    pub slack_bot_token: Option<String>,
    /// Slack channel ID to post escalation messages to.
    pub slack_channel_id: Option<String>,
    /// Slack app signing secret for verifying inbound callbacks.
    pub slack_signing_secret: Option<String>,
    /// SMTP host for email notifications (e.g. "smtp.gmail.com").
    pub smtp_host: Option<String>,
    /// SMTP port (default 587 for STARTTLS).
    pub smtp_port: u16,
    /// SMTP username.
    pub smtp_username: Option<String>,
    /// SMTP password.
    pub smtp_password: Option<String>,
    /// Sender address for notification emails.
    pub email_from: Option<String>,
    /// Recipient address for escalation notification emails.
    pub escalation_email_to: Option<String>,
    /// Resend API key (alternative to SMTP).
    pub resend_api_key: Option<String>,
    /// Dashboard base URL for deep links in emails (e.g. "https://dashboard.cream.io").
    pub dashboard_base_url: Option<String>,
    /// AES-256 key (hex-encoded, 64 chars = 32 bytes) for encrypting provider API keys at rest.
    /// If unset, provider key storage endpoints return 503.
    pub provider_key_encryption_secret: Option<Vec<u8>>,
}

impl AppConfig {
    /// Load configuration from environment variables with sensible defaults.
    ///
    /// Required: `DATABASE_URL`, `REDIS_URL`.
    /// Optional (with defaults): `HOST`, `PORT`, `RATE_LIMIT_REQUESTS`,
    /// `RATE_LIMIT_WINDOW_SECS`, `ESCALATION_CHECK_INTERVAL_SECS`.
    pub fn from_env() -> Result<Self, ConfigError> {
        let database_url =
            env::var("DATABASE_URL").map_err(|_| ConfigError::Missing("DATABASE_URL"))?;
        let redis_url = env::var("REDIS_URL").map_err(|_| ConfigError::Missing("REDIS_URL"))?;

        let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = parse_env("PORT", 8080)?;
        let rate_limit_requests = parse_env("RATE_LIMIT_REQUESTS", 100)?;
        let rate_limit_window_secs = parse_env("RATE_LIMIT_WINDOW_SECS", 60)?;
        let escalation_check_interval_secs = parse_env("ESCALATION_CHECK_INTERVAL_SECS", 30)?;
        let webhook_delivery_timeout_secs = parse_env("WEBHOOK_DELIVERY_TIMEOUT_SECS", 10)?;
        let webhook_max_retries: u16 = parse_env("WEBHOOK_MAX_RETRIES", 5)?;
        let jwt_access_ttl_secs: i64 = parse_env("JWT_ACCESS_TTL_SECS", 900)?;
        let jwt_refresh_ttl_secs: i64 = parse_env("JWT_REFRESH_TTL_SECS", 604800)?;

        let jwt_secret = match env::var("JWT_SECRET") {
            Ok(val) => {
                let trimmed = val.trim().to_string();
                if trimmed.is_empty() {
                    None
                } else if trimmed.len() < MIN_OPERATOR_KEY_LEN {
                    return Err(ConfigError::Invalid(
                        "JWT_SECRET",
                        format!(
                            "must be at least {MIN_OPERATOR_KEY_LEN} characters (got {})",
                            trimmed.len()
                        ),
                    ));
                } else {
                    Some(trimmed)
                }
            }
            Err(_) => None,
        };

        let cors_allowed_origins: Vec<String> = env::var("CORS_ALLOWED_ORIGINS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        // Operator API key. Unset → no operator access (safe default). Set →
        // must be at least MIN_OPERATOR_KEY_LEN characters to avoid trivially
        // guessable deployments. Empty-after-trim is treated as unset.
        let operator_api_key = match env::var("OPERATOR_API_KEY") {
            Ok(val) => {
                let trimmed = val.trim().to_string();
                if trimmed.is_empty() {
                    None
                } else if trimmed.len() < MIN_OPERATOR_KEY_LEN {
                    return Err(ConfigError::Invalid(
                        "OPERATOR_API_KEY",
                        format!(
                            "must be at least {MIN_OPERATOR_KEY_LEN} characters (got {})",
                            trimmed.len()
                        ),
                    ));
                } else {
                    Some(trimmed)
                }
            }
            Err(_) => None,
        };

        let slack_bot_token = env::var("SLACK_BOT_TOKEN").ok().filter(|s| !s.trim().is_empty());
        let slack_channel_id = env::var("SLACK_CHANNEL_ID").ok().filter(|s| !s.trim().is_empty());
        let slack_signing_secret =
            env::var("SLACK_SIGNING_SECRET").ok().filter(|s| !s.trim().is_empty());

        if slack_bot_token.is_some() && slack_channel_id.is_some() && slack_signing_secret.is_some()
        {
            tracing::info!("Slack integration enabled");
        }

        let smtp_host = env::var("SMTP_HOST").ok().filter(|s| !s.trim().is_empty());
        let smtp_port: u16 = parse_env("SMTP_PORT", 587)?;
        let smtp_username = env::var("SMTP_USERNAME").ok().filter(|s| !s.trim().is_empty());
        let smtp_password = env::var("SMTP_PASSWORD").ok().filter(|s| !s.trim().is_empty());
        let email_from = env::var("EMAIL_FROM").ok().filter(|s| !s.trim().is_empty());
        let escalation_email_to =
            env::var("ESCALATION_EMAIL_TO").ok().filter(|s| !s.trim().is_empty());
        let resend_api_key = env::var("RESEND_API_KEY").ok().filter(|s| !s.trim().is_empty());
        let dashboard_base_url =
            env::var("DASHBOARD_BASE_URL").ok().filter(|s| !s.trim().is_empty());

        // Provider key encryption secret: if set, MUST be valid. Silently
        // ignoring a malformed secret could lead operators to believe their
        // keys are encrypted when they're not.
        let provider_key_encryption_secret = match env::var("PROVIDER_KEY_ENCRYPTION_SECRET") {
            Ok(val) if !val.trim().is_empty() => {
                let trimmed = val.trim();
                let bytes = hex::decode(trimmed).map_err(|e| {
                    ConfigError::Invalid(
                        "PROVIDER_KEY_ENCRYPTION_SECRET",
                        format!("must be valid hex: {e}"),
                    )
                })?;
                if bytes.len() != 32 {
                    return Err(ConfigError::Invalid(
                        "PROVIDER_KEY_ENCRYPTION_SECRET",
                        format!("must be 64 hex chars (32 bytes), got {} bytes", bytes.len()),
                    ));
                }
                Some(bytes)
            }
            _ => None,
        };

        if operator_api_key.is_none() && jwt_secret.is_none() {
            tracing::warn!(
                "neither JWT_SECRET nor OPERATOR_API_KEY is set — operator-only endpoints \
                 (agent list/create/update, approve/reject, audit cross-agent) will return 401"
            );
        } else if operator_api_key.is_none() && jwt_secret.is_some() {
            tracing::info!("JWT auth enabled; OPERATOR_API_KEY not set (legacy auth disabled)");
        } else if operator_api_key.is_some() && jwt_secret.is_none() {
            tracing::info!(
                "OPERATOR_API_KEY set (legacy auth); JWT_SECRET not set — consider migrating \
                 to JWT auth (Phase 16-B)"
            );
        }

        Ok(Self {
            database_url,
            redis_url,
            host,
            port,
            rate_limit_requests,
            rate_limit_window_secs,
            escalation_check_interval_secs,
            cors_allowed_origins,
            operator_api_key,
            webhook_delivery_timeout_secs,
            webhook_max_retries,
            jwt_secret,
            jwt_access_ttl_secs,
            jwt_refresh_ttl_secs,
            slack_bot_token,
            slack_channel_id,
            slack_signing_secret,
            smtp_host,
            smtp_port,
            smtp_username,
            smtp_password,
            email_from,
            escalation_email_to,
            resend_api_key,
            dashboard_base_url,
            provider_key_encryption_secret,
        })
    }
}

fn parse_env<T: std::str::FromStr>(key: &'static str, default: T) -> Result<T, ConfigError>
where
    T::Err: std::fmt::Display,
{
    match env::var(key) {
        Ok(val) => val
            .parse::<T>()
            .map_err(|e| ConfigError::Invalid(key, e.to_string())),
        Err(_) => Ok(default),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("missing required environment variable: {0}")]
    Missing(&'static str),
    #[error("invalid value for {0}: {1}")]
    Invalid(&'static str, String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Env vars are process-global and Cargo runs tests in parallel within a
    /// binary — so any test that mutates env must hold this mutex for its
    /// entire duration. Poisoned-lock recovery is fine here: we don't care
    /// if a prior test panicked mid-mutation, since every test below
    /// unconditionally sets or clears the vars it cares about.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env<F: FnOnce()>(f: F) {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        // Clean slate — remove anything a prior test might have set.
        env::remove_var("DATABASE_URL");
        env::remove_var("REDIS_URL");
        env::remove_var("OPERATOR_API_KEY");
        f();
        // Leave no residue for the next test in the sequence.
        env::remove_var("DATABASE_URL");
        env::remove_var("REDIS_URL");
        env::remove_var("OPERATOR_API_KEY");
    }

    #[test]
    fn missing_database_url_errors() {
        with_env(|| {
            let result = AppConfig::from_env();
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.to_string().contains("DATABASE_URL"));
        });
    }

    #[test]
    fn short_operator_key_rejected() {
        with_env(|| {
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("REDIS_URL", "redis://localhost");
            env::set_var("OPERATOR_API_KEY", "short");

            let result = AppConfig::from_env();
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("OPERATOR_API_KEY") && err.contains("32"),
                "expected error to mention OPERATOR_API_KEY and 32, got: {err}"
            );
        });
    }

    #[test]
    fn operator_key_empty_treated_as_unset() {
        with_env(|| {
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("REDIS_URL", "redis://localhost");
            env::set_var("OPERATOR_API_KEY", "   ");

            let config = AppConfig::from_env().expect("whitespace key should load as unset");
            assert!(config.operator_api_key.is_none());
        });
    }

    #[test]
    fn operator_key_valid_length_accepted() {
        with_env(|| {
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("REDIS_URL", "redis://localhost");
            env::set_var("OPERATOR_API_KEY", "a".repeat(32));

            let config = AppConfig::from_env().expect("32-char key should load");
            assert_eq!(config.operator_api_key.as_deref().map(str::len), Some(32));
        });
    }
}
