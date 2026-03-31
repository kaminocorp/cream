use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() {
    // Initialise tracing: structured JSON in production, pretty-printed for local dev.
    // Controlled via RUST_LOG env var (default: info for cream crates, warn for everything else).
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("cream_api=debug,cream_models=debug,cream_policy=debug,cream_providers=debug,cream_router=debug,cream_audit=debug,info")
    });

    fmt().with_env_filter(filter).with_target(true).init();

    tracing::info!("cream-api starting");

    // Placeholder: the Axum server will be wired here in Phase 8.
    tracing::info!("cream-api ready (no routes configured yet — scaffold only)");
}
