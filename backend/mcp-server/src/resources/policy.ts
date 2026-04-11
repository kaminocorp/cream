// agent://policy/{agent_id} resource.
//
// Declarative read of an agent's current policy profile and attached rules.
// Resources differ from tools in that they are read-only, addressed by URI,
// and intended for MCP clients to surface as context or browseable data.
// Errors are returned as JSON content blocks rather than thrown — clients
// can inspect the error without the resource read failing entirely.

import { McpServer, ResourceTemplate } from "@modelcontextprotocol/sdk/server/mcp.js";
import { ApiClient } from "../api-client";

export function registerPolicyResource(
  server: McpServer,
  api: ApiClient,
): void {
  server.registerResource(
    "agent-policy",
    new ResourceTemplate("agent://policy/{agent_id}", { list: undefined }),
    {
      description:
        "Current policy profile and rules for a given agent. " +
        "Shows spending limits, allowed categories, geographic restrictions, and escalation thresholds.",
      mimeType: "application/json",
    },
    async (uri, variables) => {
      const agentId = variables.agent_id as string;
      try {
        const policy = await api.getAgentPolicy(agentId);
        return {
          contents: [
            {
              uri: uri.toString(),
              text: JSON.stringify(policy, null, 2),
              mimeType: "application/json",
            },
          ],
        };
      } catch (error) {
        return {
          contents: [
            {
              uri: uri.toString(),
              text: JSON.stringify({
                error: error instanceof Error ? error.message : String(error),
                agent_id: agentId,
              }),
              mimeType: "application/json",
            },
          ],
        };
      }
    },
  );
}
