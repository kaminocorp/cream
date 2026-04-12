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
    /// If empty or unset, defaults to permissive (development only).
    pub cors_allowed_origins: Vec<String>,
    /// Shared secret that, when presented as `Authorization: Bearer <key>`,
    /// authenticates the caller as the operator (dashboard / admin).
    ///
    /// Phase 15.1 interim design: a single shared key loaded from
    /// `OPERATOR_API_KEY`. Phase 16-A replaces this with real per-user auth
    /// tied to an `operators` table; at that point this field becomes a
    /// legacy fallback that 16-A can delete.
    ///
    /// Left unset = no operator access. The dashboard will be unable to
    /// reach operator-only endpoints (403), which is the correct behavior
    /// for deployments where operator auth has not been configured.
    pub operator_api_key: Option<String>,
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

        if operator_api_key.is_none() {
            tracing::warn!(
                "OPERATOR_API_KEY not set — operator-only endpoints (agent list/create/update, \
                 approve/reject, audit cross-agent) will return 401. Set it to enable the \
                 dashboard."
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
