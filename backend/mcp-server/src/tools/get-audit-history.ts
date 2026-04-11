// get_audit_history tool — GET /v1/audit
//
// Query the immutable audit log with filters. Used by agents to review their
// own payment history (policy decisions, settlement outcomes, errors). For
// unfiltered recent activity, prefer the agent://audit/{agent_id} resource
// which returns a fixed small window.

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { ApiClient } from "../api-client";
import { ApiError } from "../types";

export function registerGetAuditHistoryTool(
  server: McpServer,
  api: ApiClient,
): void {
  server.registerTool(
    "get_audit_history",
    {
      title: "Get Audit History",
      description:
        "Query the immutable audit log for payment lifecycle events. " +
        "Every payment request, policy decision, routing selection, and settlement outcome is recorded here. " +
        "Filter by status, date range, amount, or category. " +
        "Results are ordered by timestamp descending.",
      inputSchema: {
        status: z
          .enum([
            "pending",
            "validating",
            "pending_approval",
            "approved",
            "submitted",
            "settled",
            "failed",
            "blocked",
            "rejected",
            "timed_out",
          ])
          .optional()
          .describe("Filter by payment status."),
        from: z
          .string()
          .optional()
          .describe(
            "Start of date range (ISO 8601, e.g. '2026-04-01T00:00:00Z').",
          ),
        to: z
          .string()
          .optional()
          .describe("End of date range (ISO 8601)."),
        min_amount: z
          .string()
          .optional()
          .describe("Minimum payment amount as decimal string."),
        max_amount: z
          .string()
          .optional()
          .describe("Maximum payment amount as decimal string."),
        category: z
          .string()
          .optional()
          .describe(
            "Payment category filter (e.g. 'cloud_infrastructure').",
          ),
        limit: z
          .number()
          .int()
          .min(1)
          .max(100)
          .default(20)
          .describe("Number of results to return (1–100)."),
        offset: z
          .number()
          .int()
          .min(0)
          .default(0)
          .describe("Pagination offset."),
      },
    },
    async (args) => {
      const filters: Record<string, string | number> = {};
      if (args.status) filters.status = args.status;
      if (args.from) filters.from = args.from;
      if (args.to) filters.to = args.to;
      if (args.min_amount) filters.min_amount = args.min_amount;
      if (args.max_amount) filters.max_amount = args.max_amount;
      if (args.category) filters.category = args.category;
      if (args.limit) filters.limit = args.limit;
      if (args.offset) filters.offset = args.offset;

      try {
        const entries = await api.queryAudit(filters);
        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify(entries, null, 2),
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
