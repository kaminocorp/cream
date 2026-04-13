use std::env;

/// Minimum allowed length for the operator API key. Chosen so that a plausibly
/// generated key (32 hex chars = 128 bits of entropy) is the floor; shorter
/// values are refused at config load. Callers who ignore this and set something
/// trivial like `OPERATOR_API_KEY=test` will fail fast rather than discover it
/// by watching unauthenticated callers get admin access.
pub const MIN_OPERATOR_KEY_LEN: usize = 32;

/// Log output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// Machine-parseable JSON (one object per line). Default for production.
    Json,
    /// Human-readable, optionally coloured. Default for development.
    Pretty,
}

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
    /// Log output format: `json` for machine-parseable structured logs (production),
    /// `pretty` for human-readable coloured output (development).
    /// Default: `json` when `RUST_LOG` is not set to debug, `pretty` otherwise.
    pub log_format: LogFormat,
    /// Global log level override. Defaults to `info`. Overridden by `RUST_LOG`
    /// if that env var is set.
    pub log_level: String,
    /// Enable request/response body logging with PII redaction. Off by default.
    /// When enabled, the `PiiRedactionLayer` strips sensitive fields
    /// (`password`, `api_key`, `secret`, `refresh_token`) from logged bodies.
    pub log_bodies: bool,
    /// Enable OpenTelemetry distributed tracing. When `false`, zero performance
    /// overhead — the OTEL layer is not added to the subscriber stack.
    pub otel_enabled: bool,
    /// OTLP gRPC endpoint (e.g. `http://localhost:4317`). Required when
    /// `otel_enabled` is true.
    pub otel_exporter_endpoint: Option<String>,
    /// Service name reported in traces. Default: `cream-api`.
    pub otel_service_name: String,
    /// Enable Prometheus metrics endpoint. Default: `true`.
    pub metrics_enabled: bool,
    /// Port for the Prometheus `/metrics` HTTP listener. Default: `9090`.
    /// Separate from the main API port to keep metrics internal-only.
    pub metrics_port: u16,
    /// Path to TLS certificate (PEM). When both `tls_cert_path` and
    /// `tls_key_path` are set, the server starts with HTTPS via rustls.
    /// Unset = plain HTTP (for local dev or behind a reverse proxy).
    pub tls_cert_path: Option<String>,
    /// Path to TLS private key (PEM).
    pub tls_key_path: Option<String>,
    /// HSTS `max-age` in seconds. Default: `31536000` (1 year). Only
    /// effective when the security headers middleware is active.
    pub hsts_max_age: u64,
    /// Warn when an agent's API key is older than this many days.
    /// Default: `90`. The credential age monitor checks on the same
    /// interval as the escalation timeout monitor.
    pub credential_rotation_warn_days: u64,
    /// S3 bucket for async audit exports. If unset, the async export
    /// endpoint returns 503.
    pub audit_export_s3_bucket: Option<String>,
    /// AWS region for the S3 bucket.
    pub audit_export_s3_region: Option<String>,
    /// Key prefix for S3 exports (e.g. `audit/`).
    pub audit_export_s3_prefix: Option<String>,
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

        let log_format = match env::var("LOG_FORMAT").unwrap_or_default().to_lowercase().as_str() {
            "json" => LogFormat::Json,
            "pretty" => LogFormat::Pretty,
            "" => LogFormat::Json, // default: JSON for production
            other => {
                return Err(ConfigError::Invalid(
                    "LOG_FORMAT",
                    format!("must be 'json' or 'pretty', got '{other}'"),
                ));
            }
        };
        let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
        match log_level.as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {}
            other => {
                return Err(ConfigError::Invalid(
                    "LOG_LEVEL",
                    format!("must be one of trace/debug/info/warn/error, got '{other}'"),
                ));
            }
        }
        let log_bodies = env::var("LOG_BODIES")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);

        let otel_enabled = env::var("OTEL_ENABLED")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);
        let otel_exporter_endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
            .ok()
            .filter(|s| !s.trim().is_empty());
        let otel_service_name = env::var("OTEL_SERVICE_NAME")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "cream-api".to_string());

        let metrics_enabled = env::var("METRICS_ENABLED")
            .map(|v| !v.eq_ignore_ascii_case("false") && v != "0")
            .unwrap_or(true); // default: enabled
        let metrics_port: u16 = parse_env("METRICS_PORT", 9090)?;

        let tls_cert_path = env::var("TLS_CERT_PATH").ok().filter(|s| !s.trim().is_empty());
        let tls_key_path = env::var("TLS_KEY_PATH").ok().filter(|s| !s.trim().is_empty());
        let hsts_max_age: u64 = parse_env("HSTS_MAX_AGE", 31_536_000)?;
        let credential_rotation_warn_days: u64 = parse_env("CREDENTIAL_ROTATION_WARN_DAYS", 90)?;

        let audit_export_s3_bucket = env::var("AUDIT_EXPORT_S3_BUCKET")
            .ok()
            .filter(|s| !s.trim().is_empty());
        let audit_export_s3_region = env::var("AUDIT_EXPORT_S3_REGION")
            .ok()
            .filter(|s| !s.trim().is_empty());
        let audit_export_s3_prefix = env::var("AUDIT_EXPORT_S3_PREFIX")
            .ok()
            .filter(|s| !s.trim().is_empty());

        // TLS: both cert and key must be provided together. One without the
        // other is always a misconfiguration that should fail fast.
        match (&tls_cert_path, &tls_key_path) {
            (Some(_), None) => {
                return Err(ConfigError::Invalid(
                    "TLS_KEY_PATH",
                    "required when TLS_CERT_PATH is set".to_string(),
                ));
            }
            (None, Some(_)) => {
                return Err(ConfigError::Invalid(
                    "TLS_CERT_PATH",
                    "required when TLS_KEY_PATH is set".to_string(),
                ));
            }
            _ => {}
        }

        if otel_enabled && otel_exporter_endpoint.is_none() {
            return Err(ConfigError::Invalid(
                "OTEL_EXPORTER_OTLP_ENDPOINT",
                "required when OTEL_ENABLED=true".to_string(),
            ));
        }

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
            log_format,
            log_level,
            log_bodies,
            otel_enabled,
            otel_exporter_endpoint,
            otel_service_name,
            metrics_enabled,
            metrics_port,
            tls_cert_path,
            tls_key_path,
            hsts_max_age,
            credential_rotation_warn_days,
            audit_export_s3_bucket,
            audit_export_s3_region,
            audit_export_s3_prefix,
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
        let vars = [
            "DATABASE_URL", "REDIS_URL", "OPERATOR_API_KEY",
            "LOG_FORMAT", "LOG_LEVEL",
            "OTEL_ENABLED", "OTEL_EXPORTER_OTLP_ENDPOINT", "OTEL_SERVICE_NAME",
            "METRICS_ENABLED", "METRICS_PORT",
            "TLS_CERT_PATH", "TLS_KEY_PATH", "HSTS_MAX_AGE", "CREDENTIAL_ROTATION_WARN_DAYS",
            "AUDIT_EXPORT_S3_BUCKET", "AUDIT_EXPORT_S3_REGION", "AUDIT_EXPORT_S3_PREFIX",
        ];
        for var in &vars { env::remove_var(var); }
        f();
        for var in &vars { env::remove_var(var); }
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

    #[test]
    fn log_format_defaults_to_json() {
        with_env(|| {
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("REDIS_URL", "redis://localhost");

            let config = AppConfig::from_env().expect("should load");
            assert_eq!(config.log_format, LogFormat::Json);
        });
    }

    #[test]
    fn log_format_pretty_accepted() {
        with_env(|| {
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("REDIS_URL", "redis://localhost");
            env::set_var("LOG_FORMAT", "pretty");

            let config = AppConfig::from_env().expect("should load");
            assert_eq!(config.log_format, LogFormat::Pretty);
        });
    }

    #[test]
    fn log_format_json_accepted() {
        with_env(|| {
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("REDIS_URL", "redis://localhost");
            env::set_var("LOG_FORMAT", "json");

            let config = AppConfig::from_env().expect("should load");
            assert_eq!(config.log_format, LogFormat::Json);
        });
    }

    #[test]
    fn log_format_invalid_rejected() {
        with_env(|| {
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("REDIS_URL", "redis://localhost");
            env::set_var("LOG_FORMAT", "xml");

            let result = AppConfig::from_env();
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(err.contains("LOG_FORMAT"), "error should mention LOG_FORMAT, got: {err}");
        });
    }

    #[test]
    fn log_level_defaults_to_info() {
        with_env(|| {
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("REDIS_URL", "redis://localhost");

            let config = AppConfig::from_env().expect("should load");
            assert_eq!(config.log_level, "info");
        });
    }

    #[test]
    fn log_level_custom_accepted() {
        with_env(|| {
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("REDIS_URL", "redis://localhost");
            env::set_var("LOG_LEVEL", "debug");

            let config = AppConfig::from_env().expect("should load");
            assert_eq!(config.log_level, "debug");
        });
    }

    #[test]
    fn log_level_invalid_rejected() {
        with_env(|| {
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("REDIS_URL", "redis://localhost");
            env::set_var("LOG_LEVEL", "banana");

            let err = AppConfig::from_env().unwrap_err();
            let msg = format!("{err}");
            assert!(msg.contains("LOG_LEVEL"), "error should mention LOG_LEVEL: {msg}");
        });
    }

    #[test]
    fn otel_disabled_by_default() {
        with_env(|| {
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("REDIS_URL", "redis://localhost");

            let config = AppConfig::from_env().expect("should load");
            assert!(!config.otel_enabled);
            assert!(config.otel_exporter_endpoint.is_none());
            assert_eq!(config.otel_service_name, "cream-api");
        });
    }

    #[test]
    fn otel_enabled_requires_endpoint() {
        with_env(|| {
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("REDIS_URL", "redis://localhost");
            env::set_var("OTEL_ENABLED", "true");
            // No OTEL_EXPORTER_OTLP_ENDPOINT set

            let result = AppConfig::from_env();
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("OTEL_EXPORTER_OTLP_ENDPOINT"),
                "error should mention OTEL_EXPORTER_OTLP_ENDPOINT, got: {err}"
            );
        });
    }

    #[test]
    fn otel_enabled_with_endpoint_accepted() {
        with_env(|| {
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("REDIS_URL", "redis://localhost");
            env::set_var("OTEL_ENABLED", "true");
            env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4317");

            let config = AppConfig::from_env().expect("should load");
            assert!(config.otel_enabled);
            assert_eq!(
                config.otel_exporter_endpoint.as_deref(),
                Some("http://localhost:4317")
            );
        });
    }

    #[test]
    fn otel_service_name_configurable() {
        with_env(|| {
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("REDIS_URL", "redis://localhost");
            env::set_var("OTEL_SERVICE_NAME", "my-custom-service");

            let config = AppConfig::from_env().expect("should load");
            assert_eq!(config.otel_service_name, "my-custom-service");
        });
    }

    #[test]
    fn otel_disabled_explicit_false() {
        with_env(|| {
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("REDIS_URL", "redis://localhost");
            env::set_var("OTEL_ENABLED", "false");
            env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4317");

            let config = AppConfig::from_env().expect("should load");
            assert!(!config.otel_enabled);
            // Endpoint is still parsed even when disabled (allows toggling without removing)
            assert!(config.otel_exporter_endpoint.is_some());
        });
    }

    #[test]
    fn metrics_enabled_by_default() {
        with_env(|| {
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("REDIS_URL", "redis://localhost");

            let config = AppConfig::from_env().expect("should load");
            assert!(config.metrics_enabled);
            assert_eq!(config.metrics_port, 9090);
        });
    }

    #[test]
    fn metrics_disabled_explicit() {
        with_env(|| {
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("REDIS_URL", "redis://localhost");
            env::set_var("METRICS_ENABLED", "false");

            let config = AppConfig::from_env().expect("should load");
            assert!(!config.metrics_enabled);
        });
    }

    #[test]
    fn metrics_port_configurable() {
        with_env(|| {
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("REDIS_URL", "redis://localhost");
            env::set_var("METRICS_PORT", "9191");

            let config = AppConfig::from_env().expect("should load");
            assert_eq!(config.metrics_port, 9191);
        });
    }

    #[test]
    fn tls_defaults_to_none() {
        with_env(|| {
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("REDIS_URL", "redis://localhost");

            let config = AppConfig::from_env().expect("should load");
            assert!(config.tls_cert_path.is_none());
            assert!(config.tls_key_path.is_none());
        });
    }

    #[test]
    fn tls_cert_without_key_rejected() {
        with_env(|| {
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("REDIS_URL", "redis://localhost");
            env::set_var("TLS_CERT_PATH", "/tmp/cert.pem");

            let result = AppConfig::from_env();
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("TLS_KEY_PATH"),
                "error should mention TLS_KEY_PATH, got: {err}"
            );
        });
    }

    #[test]
    fn tls_key_without_cert_rejected() {
        with_env(|| {
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("REDIS_URL", "redis://localhost");
            env::set_var("TLS_KEY_PATH", "/tmp/key.pem");

            let result = AppConfig::from_env();
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("TLS_CERT_PATH"),
                "error should mention TLS_CERT_PATH, got: {err}"
            );
        });
    }

    #[test]
    fn tls_both_paths_accepted() {
        with_env(|| {
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("REDIS_URL", "redis://localhost");
            env::set_var("TLS_CERT_PATH", "/tmp/cert.pem");
            env::set_var("TLS_KEY_PATH", "/tmp/key.pem");

            let config = AppConfig::from_env().expect("should load");
            assert_eq!(config.tls_cert_path.as_deref(), Some("/tmp/cert.pem"));
            assert_eq!(config.tls_key_path.as_deref(), Some("/tmp/key.pem"));
        });
    }

    #[test]
    fn hsts_max_age_defaults_to_one_year() {
        with_env(|| {
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("REDIS_URL", "redis://localhost");

            let config = AppConfig::from_env().expect("should load");
            assert_eq!(config.hsts_max_age, 31_536_000);
        });
    }

    #[test]
    fn hsts_max_age_configurable() {
        with_env(|| {
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("REDIS_URL", "redis://localhost");
            env::set_var("HSTS_MAX_AGE", "86400");

            let config = AppConfig::from_env().expect("should load");
            assert_eq!(config.hsts_max_age, 86400);
        });
    }

    #[test]
    fn credential_rotation_warn_days_defaults_to_90() {
        with_env(|| {
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("REDIS_URL", "redis://localhost");

            let config = AppConfig::from_env().expect("should load");
            assert_eq!(config.credential_rotation_warn_days, 90);
        });
    }

    #[test]
    fn credential_rotation_warn_days_configurable() {
        with_env(|| {
            env::set_var("DATABASE_URL", "postgres://localhost/test");
            env::set_var("REDIS_URL", "redis://localhost");
            env::set_var("CREDENTIAL_ROTATION_WARN_DAYS", "30");

            let config = AppConfig::from_env().expect("should load");
            assert_eq!(config.credential_rotation_warn_days, 30);
        });
    }
}
