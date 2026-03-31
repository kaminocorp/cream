default:
    @just --list

# ── Infrastructure ──────────────────────────────

up:
    docker compose up -d

down:
    docker compose down

# ── Database ────────────────────────────────────

db-create:
    sqlx database create

migrate:
    sqlx migrate run --source backend/migrations

migrate-add name:
    sqlx migrate add -r {{name}} --source backend/migrations

# ── Build ───────────────────────────────────────

build:
    cargo build --manifest-path backend/Cargo.toml

check:
    cargo check --manifest-path backend/Cargo.toml

clippy:
    cargo clippy --manifest-path backend/Cargo.toml -- -D warnings

fmt:
    cargo fmt --manifest-path backend/Cargo.toml --all

fmt-check:
    cargo fmt --manifest-path backend/Cargo.toml --all -- --check

# ── Test ────────────────────────────────────────

test:
    cargo test --manifest-path backend/Cargo.toml

test-integration:
    cargo test --manifest-path backend/Cargo.toml -- --ignored

# ── Run ─────────────────────────────────────────

run-api:
    cargo run --manifest-path backend/Cargo.toml -p cream-api

run-mcp:
    cd backend/mcp-server && npx ts-node src/index.ts

# ── Frontend ────────────────────────────────────

fe-dev:
    cd frontend && npm run dev

fe-build:
    cd frontend && npm run build

fe-lint:
    cd frontend && npm run lint
