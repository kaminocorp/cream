// get_my_policy tool — GET /v1/agents/{id}/policy
//
// Returns the agent's profile (spending limits, allowed categories, etc.)
// along with the rule set currently attached to that profile. Agents should
// call this before initiating payments to understand what will be allowed,
// blocked, or escalated.

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { ApiClient } from "../api-client";
import { ApiError } from "../types";

export function registerGetMyPolicyTool(
  server: McpServer,
  api: ApiClient,
): void {
  server.registerTool(
    "get_my_policy",
    {
      title: "Get My Policy",
      description:
        "Retrieve the current policy profile and rules governing this agent's payment capabilities. " +
        "Use this before initiating payments to understand spending limits, allowed categories, " +
        "and which payments will require human approval. " +
        "The policy is determined by the agent's profile and attached rule set.",
      inputSchema: {
        agent_id: z
          .string()
          .describe(
            "Agent ID (e.g. 'agt_01j...'). Must match the agent whose API key is in use.",
          ),
      },
    },
    async (args) => {
      try {
        const policy = await api.getAgentPolicy(args.agent_id);
        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify(policy, null, 2),
            },
          ],
        };
      } catch (error) {
        return {
          isError: true,
          content: [
            {
              type: "text" as const,
              text:
                error instanceof ApiError
                  ? `[${error.errorCode}] (HTTP ${error.status}): ${error.message}`
                  : error instanceof Error
                    ? error.message
                    : String(error),
            },
          ],
        };
      }
    },
  );
}
