# Changelog

- [0.1.0](#010--2026-03-31) — Monorepo skeleton, tooling & infrastructure

---

## 0.1.0 — 2026-03-31

**Phase 1: Monorepo Skeleton, Tooling & Infrastructure**

Establishes the complete project structure, build tooling, local infrastructure, and CI pipeline so every subsequent phase has a working environment to build against.

### Added

- **Rust workspace** with 6 crates (`models`, `policy`, `providers`, `router`, `audit`, `api`) arranged as a strict compile-time-enforced dependency DAG
- **Workspace dependency centralisation** — all shared crate versions declared once in root `Cargo.toml`, referenced via `{ workspace = true }`
- **`cream-api` binary** with structured tracing (`tracing-subscriber`, `EnvFilter`, `RUST_LOG` support)
- **Docker Compose** — Postgres 16-alpine (port 5432) and Redis 7-alpine (port 6379) with health checks
- **Justfile** — 15 task runner commands across infrastructure, database, build, test, run, and frontend categories
- **GitHub Actions CI** — two-job pipeline (check: fmt + clippy + build; test: workspace tests) with `SQLX_OFFLINE=true` and `rust-cache`
- **MCP server scaffold** — TypeScript sidecar (`backend/mcp-server/`) with `@modelcontextprotocol/sdk` dependency
- **`.env.example`** documenting `DATABASE_URL`, `REDIS_URL`, `API_HOST`, `API_PORT`, `RUST_LOG`
- **Integration test harness stub** (`backend/tests/common/mod.rs`)
- **Migrations directory** (`backend/migrations/.gitkeep`) ready for SQLx migrations
