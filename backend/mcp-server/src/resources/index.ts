// Cream MCP Server — resource registration barrel.

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { ApiClient } from "../api-client";
import { registerPolicyResource } from "./policy";
import { registerBalanceResource } from "./balance";
import { registerAuditResource } from "./audit";

export function registerAllResources(
  server: McpServer,
  api: ApiClient,
): void {
  registerPolicyResource(server, api);
  registerBalanceResource(server);
  registerAuditResource(server, api);
}
