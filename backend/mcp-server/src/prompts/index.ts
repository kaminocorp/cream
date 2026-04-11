// Cream MCP Server — prompt registration barrel.

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { registerJustificationTemplatePrompt } from "./justification-template";
import { registerPolicySummaryPrompt } from "./policy-summary";

export function registerAllPrompts(server: McpServer): void {
  registerJustificationTemplatePrompt(server);
  registerPolicySummaryPrompt(server);
}
