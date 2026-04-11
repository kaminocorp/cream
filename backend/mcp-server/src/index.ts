// Cream MCP Server — entry point.
//
// Thin TypeScript sidecar that translates MCP tool calls into Rust REST API
// calls. Zero business logic lives here; all payment processing happens in
// the Rust API. Supports two transport modes:
//
//   - stdio (default) — for Claude Desktop and locally-spawned agent
//     processes. Communicates via stdin/stdout. The MCP wire protocol owns
//     stdout exclusively — any diagnostic logging MUST go to stderr, or it
//     will corrupt the protocol stream.
//
//   - http (opt-in via MCP_TRANSPORT=http) — Streamable HTTP transport for
//     remote agents connecting over the network. Uses a stateless session
//     model: each request creates a fresh session.

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { StreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/streamableHttp.js";
import { createServer } from "node:http";
import { loadConfig } from "./config";
import { createApiClient } from "./api-client";
import { registerAllTools } from "./tools";
import { registerAllResources } from "./resources";
import { registerAllPrompts } from "./prompts";

async function main(): Promise<void> {
  const config = loadConfig();
  const api = createApiClient(config);

  const server = new McpServer({
    name: "cream-mcp-server",
    version: "0.9.0",
  });

  // Register all MCP capabilities.
  registerAllTools(server, api);
  registerAllResources(server, api);
  registerAllPrompts(server);

  if (config.transport === "stdio") {
    // stdio transport — process communicates via stdin/stdout.
    // Used by Claude Desktop and locally-spawned agent processes.
    const transport = new StdioServerTransport();
    await server.connect(transport);
    // No console.log here — stdout is the MCP wire protocol in stdio mode.
    process.stderr.write(`cream-mcp-server: running on stdio\n`);
  } else {
    // Streamable HTTP transport — remote agents connect over HTTP.
    // Each request creates a new stateless session.
    const transport = new StreamableHTTPServerTransport({
      sessionIdGenerator: undefined, // stateless mode
    });

    const httpServer = createServer(async (req, res) => {
      await transport.handleRequest(req, res);
    });

    await server.connect(transport);

    httpServer.listen(config.httpPort, () => {
      process.stderr.write(
        `cream-mcp-server: Streamable HTTP on port ${config.httpPort}\n`,
      );
    });

    // Graceful shutdown.
    const shutdown = (): void => {
      httpServer.close();
      void server.close();
    };
    process.on("SIGTERM", shutdown);
    process.on("SIGINT", shutdown);
  }
}

main().catch((error) => {
  process.stderr.write(`cream-mcp-server: fatal error: ${error}\n`);
  process.exit(1);
});
