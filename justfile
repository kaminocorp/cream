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

# ── MCP Server ──────────────────────────────────

mcp-install:
    cd backend/mcp-server && npm install

mcp-dev:
    cd backend/mcp-server && npm run dev

mcp-build:
    cd backend/mcp-server && npm run build

mcp-test:
    cd backend/mcp-server && npm test

mcp-lint:
    cd backend/mcp-server && npm run lint

mcp-start:
    cd backend/mcp-server && npm run start

# ── Frontend ────────────────────────────────────

fe-install:
    cd frontend && npm install

fe-dev:
    cd frontend && npm run dev -- --port 3000

fe-build:
    cd frontend && npm run build

fe-lint:
    cd frontend && npm run lint

fe-type-check:
    cd frontend && npx tsc --noEmit
