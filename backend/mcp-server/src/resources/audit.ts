// agent://audit/{agent_id} resource.
//
// Returns the 20 most recent audit log entries. This is the "recent activity"
// view — for filtered queries by status, date range, or amount, agents
// should use the get_audit_history tool instead.

import { McpServer, ResourceTemplate } from "@modelcontextprotocol/sdk/server/mcp.js";
import { ApiClient } from "../api-client";

export function registerAuditResource(
  server: McpServer,
  api: ApiClient,
): void {
  server.registerResource(
    "agent-audit",
    new ResourceTemplate("agent://audit/{agent_id}", { list: undefined }),
    {
      description:
        "Recent audit log entries for a given agent. " +
        "Returns the 20 most recent payment lifecycle events. " +
        "For filtered queries (by status, date range, amount), use the get_audit_history tool instead.",
      mimeType: "application/json",
    },
    async (uri, variables) => {
      const agentId = variables.agent_id as string;
      try {
        const entries = await api.queryAudit({ limit: 20 });
        return {
          contents: [
            {
              uri: uri.toString(),
              text: JSON.stringify({ agent_id: agentId, entries }, null, 2),
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
