# MCP Integration Guide

Connect AI agents to Cream via the Model Context Protocol (MCP).

## Overview

Cream's MCP server is a TypeScript sidecar that translates MCP tool calls into Cream REST API calls. Any MCP-compatible agent (Claude, GPT-4, LangChain, LangGraph) can use Cream's payment capabilities without custom SDK integration.

## Setup

### Stdio Mode (Local Agents)

For agents running as local processes (e.g., Claude Desktop, local LangChain):

```json
{
  "mcpServers": {
    "cream": {
      "command": "npx",
      "args": ["@kaminocorp/cream-mcp-server"],
      "env": {
        "CREAM_API_URL": "http://localhost:8080",
        "CREAM_API_KEY": "cream_<agent_api_key>"
      }
    }
  }
}
```

### HTTP+SSE Mode (Remote Agents)

For remote or multi-agent deployments:

```bash
cd backend/mcp-server
CREAM_API_URL=http://localhost:8080 \
CREAM_API_KEY=cream_<agent_api_key> \
MCP_TRANSPORT=http \
MCP_PORT=3001 \
npx ts-node src/index.ts
```

Connect MCP clients to `http://localhost:3001/sse`.

## Available Tools

| Tool | Description | Parameters |
|------|-------------|------------|
| `initiate_payment` | Create a payment with justification | `amount`, `currency`, `recipient`, `justification`, `preferred_rail?` |
| `get_payment_status` | Check a payment's current status | `payment_id` |
| `create_virtual_card` | Issue a scoped virtual card | `agent_id`, `controls` |
| `get_my_policy` | Read the calling agent's policy profile | (none) |
| `get_audit_history` | Query the agent's audit log | `filters?` |
| `check_provider_health` | Get provider health status | (none) |

### Example: `initiate_payment`

```json
{
  "tool": "initiate_payment",
  "arguments": {
    "amount": "49.99",
    "currency": "USD",
    "recipient": {
      "type": "merchant",
      "identifier": "aws_marketplace"
    },
    "justification": {
      "summary": "Provisioning EC2 instance for load testing the staging environment",
      "category": "cloud_infrastructure"
    }
  }
}
```

## Available Resources

Resources provide read-only context that agents can access:

| Resource URI | Description |
|-------------|-------------|
| `agent://policy/{agent_id}` | Current policy rules for an agent |
| `agent://balance/{wallet_id}` | Current balance per rail |
| `agent://audit/{agent_id}` | Filtered audit history |

## Available Prompts

| Prompt | Description |
|--------|-------------|
| `payment_justification_template` | Guided structure for writing payment justifications |
| `policy_summary` | Human-readable summary of the agent's policy for context |

## Claude Desktop Integration

Add to your Claude Desktop MCP config (`~/.config/claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "cream-payments": {
      "command": "npx",
      "args": ["@kaminocorp/cream-mcp-server"],
      "env": {
        "CREAM_API_URL": "https://api.cream.example.com",
        "CREAM_API_KEY": "cream_<your_agent_key>"
      }
    }
  }
}
```

Claude can then make payments by calling tools like:

> "Pay $25 to OpenAI for API credits to complete the document processing task."

Claude will use the `initiate_payment` tool with a structured justification automatically.

## LangChain / LangGraph Integration

```python
from langchain_mcp import MCPToolkit

toolkit = MCPToolkit(
    server_command="npx @kaminocorp/cream-mcp-server",
    env={
        "CREAM_API_URL": "https://api.cream.example.com",
        "CREAM_API_KEY": "cream_<your_agent_key>",
    },
)

tools = toolkit.get_tools()
# tools includes: initiate_payment, get_payment_status, etc.
```

## Error Handling

MCP tool calls that fail return structured error messages:

- **Policy blocked**: The tool response includes `error_code: POLICY_BLOCKED` and the blocking rule IDs
- **Validation errors**: Missing or malformed fields are described in the error message
- **Rate limiting**: The tool returns a retry-after suggestion

Agents should handle these gracefully — e.g., by requesting human approval when a payment is escalated, or adjusting the amount when blocked by a spending limit.
