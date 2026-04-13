# Cream API — multi-stage Rust build.
#
# Stage 1 (builder): compiles the cream-api binary in release mode using a
# full Rust toolchain image. Uses cargo-chef for Docker layer caching of
# dependencies (skipped here for simplicity — add cargo-chef if build times
# become a bottleneck).
#
# Stage 2 (runtime): copies only the compiled binary into a minimal Debian
# slim image. The final image contains no source code, no Rust toolchain,
# and no dev dependencies.
#
# Requires: Docker BuildKit (enabled by default in modern Docker).
#
# Build from the backend/ directory:
#   docker build -t cream-api .
#
# Or from the repo root:
#   docker build -t cream-api -f backend/Dockerfile backend/

# ---------------------------------------------------------------------------
# Stage 1 — Builder
# ---------------------------------------------------------------------------
FROM rust:1.85-slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests first for dependency caching.
COPY Cargo.toml Cargo.lock ./
COPY crates/models/Cargo.toml crates/models/Cargo.toml
COPY crates/audit/Cargo.toml crates/audit/Cargo.toml
COPY crates/policy/Cargo.toml crates/policy/Cargo.toml
COPY crates/providers/Cargo.toml crates/providers/Cargo.toml
COPY crates/router/Cargo.toml crates/router/Cargo.toml
COPY crates/api/Cargo.toml crates/api/Cargo.toml

# Create stub lib.rs files so cargo can resolve the workspace and cache deps.
RUN mkdir -p crates/models/src && echo "" > crates/models/src/lib.rs && \
    mkdir -p crates/audit/src && echo "" > crates/audit/src/lib.rs && \
    mkdir -p crates/policy/src && echo "" > crates/policy/src/lib.rs && \
    mkdir -p crates/providers/src && echo "" > crates/providers/src/lib.rs && \
    mkdir -p crates/router/src && echo "" > crates/router/src/lib.rs && \
    mkdir -p crates/api/src && echo "fn main() {}" > crates/api/src/main.rs

# Build dependencies only (cached unless Cargo.toml/Cargo.lock change).
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin cream-api 2>/dev/null || true

# Copy actual source code.
COPY crates/ crates/
COPY migrations/ migrations/

# Touch source files to invalidate the stub build cache.
RUN find crates -name "*.rs" -exec touch {} +

# Build the real binary.
RUN cargo build --release --bin cream-api

# ---------------------------------------------------------------------------
# Stage 2 — Runtime
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Non-root user for security.
RUN groupadd -r cream && useradd -r -g cream cream

WORKDIR /app

# Copy the compiled binary.
COPY --from=builder /app/target/release/cream-api ./cream-api

# Copy migrations so sqlx migrate can run at startup if needed.
COPY --from=builder /app/migrations ./migrations

# Switch to non-root user.
USER cream

ENV RUST_LOG=cream_api=info,info
EXPOSE 8080

CMD ["./cream-api"]
