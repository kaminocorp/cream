use std::env;

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

        Ok(Self {
            database_url,
            redis_url,
            host,
            port,
            rate_limit_requests,
            rate_limit_window_secs,
            escalation_check_interval_secs,
            cors_allowed_origins,
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

    #[test]
    fn missing_database_url_errors() {
        // Clear relevant vars to ensure the test is isolated.
        env::remove_var("DATABASE_URL");
        let result = AppConfig::from_env();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("DATABASE_URL"));
    }
}
