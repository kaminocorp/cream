// policy_summary prompt.
//
// Given raw policy JSON (from get_my_policy or the agent-policy resource),
// returns a user message asking the model to produce a human-readable
// summary. The prompt doesn't compute anything — it's a structured
// conversation-starter that MCP clients can surface to users.

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";

export function registerPolicySummaryPrompt(server: McpServer): void {
  server.registerPrompt(
    "policy_summary",
    {
      title: "Policy Summary",
      description:
        "Given raw policy JSON from get_my_policy or the agent-policy resource, " +
        "produce a human-readable summary of what the agent is and is not allowed to pay for.",
      argsSchema: {
        policy_json: z
          .string()
          .describe(
            "The JSON string returned by get_my_policy or the agent://policy/{agent_id} resource.",
          ),
      },
    },
    (args) => ({
      messages: [
        {
          role: "user" as const,
          content: {
            type: "text" as const,
            text:
              `Here is a Cream payment policy in JSON format:\n\n` +
              `\`\`\`json\n${args.policy_json}\n\`\`\`\n\n` +
              `Please summarise in plain English:\n` +
              `1. What payment categories are permitted?\n` +
              `2. What are the spending limits (per-transaction, daily, weekly, monthly)?\n` +
              `3. Which payment rails are available?\n` +
              `4. Are there geographic restrictions?\n` +
              `5. What triggers human approval (escalation threshold)?\n` +
              `6. Are there any blocking rules I should know about?`,
          },
        },
      ],
    }),
  );
}
