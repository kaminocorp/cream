# @kaminocorp/cream-mcp-server

**MCP server for [Cream](https://github.com/kaminocorp/cream)** — the universal payment control plane for AI agents.

Connect any MCP-compatible agent (Claude Desktop, GPT-4 via LangChain/LangGraph, custom agents speaking the Model Context Protocol) to the Cream payment control plane. Agents gain the ability to initiate payments, inspect their policy, query the audit log, issue virtual cards, and check provider health — all through standard MCP tools, resources, and prompts.

**Zero business logic lives in this package.** This is a thin TypeScript sidecar that translates MCP protocol calls into HTTP requests against the Cream Rust REST API. All payment processing, policy evaluation, routing, and audit writes happen in the Rust backend. This package just handles the MCP protocol and the JSON wire format.

---

## Prerequisites

You must have a running Cream REST API instance that this MCP server can reach over HTTP. See [the main Cream repository](https://github.com/kaminocorp/cream) for how to run the Rust backend, or use a managed deployment if available.

You need:

- **`CREAM_API_URL`** — the base URL of your running Cream API (e.g. `http://localhost:8080` or `https://cream.yourcompany.com`)
- **`CREAM_API_KEY`** — a per-agent Bearer token issued by the Cream backend

Each agent should have its own API key — the backend enforces policies and spending limits per-agent.

---

## Installation

### Claude Desktop

Add the server to your Claude Desktop configuration file:

- **macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows:** `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "cream": {
      "command": "npx",
      "args": ["-y", "@kaminocorp/cream-mcp-server"],
      "env": {
        "CREAM_API_URL": "http://localhost:8080",
        "CREAM_API_KEY": "your-agent-api-key"
      }
    }
  }
}
```

Restart Claude Desktop. The six Cream tools should appear in the tool list, and Claude can now initiate policy-governed payments on your behalf.

### Other MCP clients (LangChain, LangGraph, custom)

```bash
# Run directly with npx (stdio transport)
CREAM_API_URL=http://localhost:8080 \
CREAM_API_KEY=your-agent-key \
  npx -y @kaminocorp/cream-mcp-server

# Or install globally
npm install -g @kaminocorp/cream-mcp-server
CREAM_API_URL=... CREAM_API_KEY=... cream-mcp-server

# Or in HTTP mode for remote agents
MCP_TRANSPORT=http MCP_HTTP_PORT=3002 \
CREAM_API_URL=... CREAM_API_KEY=... \
  npx -y @kaminocorp/cream-mcp-server
```

### Docker

```bash
docker run --rm -i \
  -e CREAM_API_URL=https://cream.example.com \
  -e CREAM_API_KEY=your-agent-key \
  ghcr.io/kaminocorp/cream-mcp-server
```

*(Docker image published separately — coming soon.)*

---

## What the server exposes

### Tools (6)

| Tool | Description |
|------|-------------|
| `initiate_payment` | Submit a payment request with structured justification. Routed through policy engine, failover, and audit layers. Returns `submitted`, `pending_approval` (human review required), or `blocked`/`rejected`. |
| `get_payment_status` | Retrieve current status and full audit trail for a payment by ID. Use for polling settlement or investigating failures. |
| `create_virtual_card` | Issue a scoped virtual card for card-rail payments. Single-use or multi-use, with per-transaction/cycle limits and MCC whitelist. |
| `get_my_policy` | Retrieve this agent's current profile (spending limits, allowed categories, geographic restrictions) and attached rule set. Call before `initiate_payment` to understand what will be allowed vs. escalated. |
| `get_audit_history` | Query the immutable audit log with filters (status, date range, amount, category). Results ordered by timestamp descending. |
| `check_provider_health` | Real-time health status for all registered payment providers — circuit breaker state, 5-minute error rate, p50/p99 latency. |

### Resources (3)

| URI template | Description |
|--------------|-------------|
| `agent://policy/{agent_id}` | Declarative read of an agent's current policy profile and rules |
| `agent://audit/{agent_id}` | 20 most recent audit log entries for an agent |
| `agent://balance/{wallet_id}` | *(Stub — pending API endpoint)* Balance per wallet/rail |

### Prompts (2)

| Prompt | Purpose |
|--------|---------|
| `payment_justification_template` | Guided template for producing a structured payment justification before calling `initiate_payment` |
| `policy_summary` | Takes raw policy JSON and produces a human-readable summary of spending limits, rails, and escalation triggers |

---

## Configuration

All configuration is via environment variables:

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `CREAM_API_URL` | ✓ | — | Base URL of the Cream Rust API, no trailing slash |
| `CREAM_API_KEY` | ✓ | — | Bearer token for authenticating with the API (agent-scoped) |
| `MCP_TRANSPORT` |  | `stdio` | Transport mode — `stdio` (Claude Desktop, local agents) or `http` (remote agents over the network) |
| `MCP_HTTP_PORT` |  | `3002` | Port for HTTP transport (ignored when `MCP_TRANSPORT=stdio`) |

In stdio mode the server uses stdin/stdout for the MCP wire protocol, so all diagnostic logging goes to stderr exclusively. Do not redirect stdout or you will corrupt the protocol stream.

---

## How Cream works (brief)

Cream is a Rust-based payment control plane that sits between AI agents and payment providers (Stripe, Airwallex, Coinbase x402, etc.). Every payment follows a deterministic 8-step pipeline:

1. **Schema validation** — field types, required fields, amount bounds
2. **Agent identity resolution** — load the agent's policy profile
3. **Justification evaluation** — structured parse of the agent's stated reason
4. **Policy engine** — 12 declarative rule types evaluated in priority order (amount caps, velocity limits, category whitelists, duplicate detection, etc.). Returns `APPROVE`, `BLOCK`, or `ESCALATE`
5. **Routing engine** — select optimal provider based on cost, latency, health, and corridor. Circuit breakers demote unhealthy providers automatically
6. **Provider execution** — dispatch via a trait-abstracted `PaymentProvider` interface, with cross-provider failover
7. **Settlement confirmation** — wait for provider confirmation
8. **Audit write** — append to an immutable ledger (DB triggers block UPDATE/DELETE on audit records)

The `initiate_payment` tool in this MCP server invokes exactly this pipeline. The agent's `justification` field is stored verbatim in the audit ledger — every payment has an agent-authored paper trail.

For the full architecture, see the [main Cream repository](https://github.com/kaminocorp/cream).

---

## Reporting issues

Bugs, feature requests, and questions: https://github.com/kaminocorp/cream/issues

---

## License

Apache-2.0 — see [LICENSE](./LICENSE) for the full text.
