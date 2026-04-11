// Cream MCP Server — configuration loader.
//
// Reads environment variables into a typed Config object. The MCP server is
// agent-scoped: each agent has its own CREAM_API_KEY, and config is loaded
// once at process startup. No hot reloading.

export interface Config {
  /** Base URL of the Cream Rust API, no trailing slash. */
  apiBaseUrl: string;
  /** Bearer token for authenticating with the Rust API. Scoped per agent. */
  apiKey: string;
  /** Transport mode: "stdio" (default, for Claude Desktop / local) or "http" (for remote agents). */
  transport: "stdio" | "http";
  /** Port for HTTP transport mode. Default 3002. */
  httpPort: number;
}

export function loadConfig(): Config {
  const apiBaseUrl = process.env.CREAM_API_URL;
  const apiKey = process.env.CREAM_API_KEY;

  if (!apiBaseUrl) {
    throw new Error(
      "CREAM_API_URL is required. Set it to the base URL of the Cream API (e.g. http://localhost:8080)",
    );
  }
  if (!apiKey) {
    throw new Error(
      "CREAM_API_KEY is required. Set it to the agent's Bearer API key.",
    );
  }

  const transport = process.env.MCP_TRANSPORT === "http" ? "http" : "stdio";
  const httpPort = parseInt(process.env.MCP_HTTP_PORT ?? "3002", 10);

  if (isNaN(httpPort) || httpPort < 1 || httpPort > 65535) {
    throw new Error(
      `MCP_HTTP_PORT must be a valid port number, got: ${process.env.MCP_HTTP_PORT}`,
    );
  }

  return {
    apiBaseUrl: apiBaseUrl.replace(/\/$/, ""),
    apiKey,
    transport,
    httpPort,
  };
}
