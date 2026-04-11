// get_payment_status tool — GET /v1/payments/{id}
//
// Returns the payment record and all associated audit log entries. Use for
// polling settlement after initiate_payment, or for investigating a failed
// payment. Errors (including 404) are surfaced as isError content blocks
// rather than thrown exceptions, so the agent gets a structured message.

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { ApiClient } from "../api-client";
import { ApiError } from "../types";

export function registerGetPaymentStatusTool(
  server: McpServer,
  api: ApiClient,
): void {
  server.registerTool(
    "get_payment_status",
    {
      title: "Get Payment Status",
      description:
        "Retrieve the current status and full audit trail for a payment by its ID. " +
        "Use this to poll for settlement after initiating a payment, or to investigate a failed payment. " +
        "Returns the payment record and all associated audit log entries.",
      inputSchema: {
        payment_id: z
          .string()
          .describe(
            "Payment ID (e.g. 'pay_01j...'). Returned by initiate_payment.",
          ),
      },
    },
    async (args) => {
      try {
        const detail = await api.getPayment(args.payment_id);
        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify(detail, null, 2),
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
