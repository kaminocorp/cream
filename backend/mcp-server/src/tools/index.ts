// Cream MCP Server — tool registration barrel.
//
// registerAllTools wires every tool handler into the McpServer in one call.
// Adding a new tool is a two-step change: implement the handler in a new
// file, then import + register it here.

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { ApiClient } from "../api-client";
import { registerInitiatePaymentTool } from "./initiate-payment";
import { registerGetPaymentStatusTool } from "./get-payment-status";
import { registerCreateVirtualCardTool } from "./create-virtual-card";
import { registerGetMyPolicyTool } from "./get-my-policy";
import { registerGetAuditHistoryTool } from "./get-audit-history";
import { registerCheckProviderHealthTool } from "./check-provider-health";

export function registerAllTools(server: McpServer, api: ApiClient): void {
  registerInitiatePaymentTool(server, api);
  registerGetPaymentStatusTool(server, api);
  registerCreateVirtualCardTool(server, api);
  registerGetMyPolicyTool(server, api);
  registerGetAuditHistoryTool(server, api);
  registerCheckProviderHealthTool(server, api);
}
