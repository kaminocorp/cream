// initiate_payment tool — POST /v1/payments
//
// Maps MCP tool arguments into a Cream payment request body. Auto-generates
// an idempotency key if none is provided, so agents can call the tool safely
// without managing keys manually. All business logic (policy evaluation,
// routing, provider dispatch) lives in the Rust API.

import { randomUUID } from "node:crypto";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { ApiClient } from "../api-client";
import { ApiError } from "../types";

export function registerInitiatePaymentTool(
  server: McpServer,
  api: ApiClient,
): void {
  server.registerTool(
    "initiate_payment",
    {
      title: "Initiate Payment",
      description:
        "Submit a payment request to the Cream payment control plane. " +
        "The payment will be evaluated against your agent's policy rules before execution. " +
        "Returns immediately — if status is 'pending_approval', a human must approve before the payment executes. " +
        "If status is 'settled', the payment is complete.",
      inputSchema: {
        amount: z
          .string()
          .describe(
            "Payment amount as a decimal string (e.g. '42.50'). Never use a float.",
          ),
        currency: z
          .string()
          .describe(
            "ISO 4217 fiat code (e.g. 'USD', 'SGD') or crypto ticker (e.g. 'USDC', 'ETH').",
          ),
        recipient_type: z
          .enum(["merchant", "individual", "wallet", "bank_account"])
          .describe("Type of recipient."),
        recipient_identifier: z
          .string()
          .describe(
            "Recipient identifier: merchant name, wallet address, account number, etc.",
          ),
        recipient_name: z
          .string()
          .optional()
          .describe("Human-readable recipient name."),
        recipient_country: z
          .string()
          .optional()
          .describe("ISO 3166-1 alpha-2 country code."),
        justification_summary: z
          .string()
          .min(10)
          .describe(
            "Why this payment is necessary. Be specific — vague summaries may be blocked by policy. Minimum 10 characters.",
          ),
        justification_category: z
          .enum([
            "saas_subscription",
            "cloud_infrastructure",
            "api_credits",
            "travel",
            "procurement",
            "marketing",
            "legal",
            "other",
          ])
          .describe("Payment category for policy evaluation."),
        justification_task_id: z
          .string()
          .optional()
          .describe("ID of the task or workflow that requires this payment."),
        justification_expected_value: z
          .string()
          .optional()
          .describe(
            "Expected business value delivered by this payment (e.g. '3 months of CI/CD runtime').",
          ),
        preferred_rail: z
          .enum(["auto", "card", "ach", "swift", "local", "stablecoin"])
          .default("auto")
          .describe(
            "Preferred payment rail. 'auto' lets the router select optimally.",
          ),
        idempotency_key: z
          .string()
          .optional()
          .describe(
            "Unique key for idempotency. Auto-generated if omitted. Provide if retrying a failed call.",
          ),
      },
    },
    async (args) => {
      const idempotency_key = args.idempotency_key ?? randomUUID();

      const requestBody = {
        amount: args.amount,
        currency: args.currency,
        recipient: {
          type: args.recipient_type,
          identifier: args.recipient_identifier,
          name: args.recipient_name,
          country: args.recipient_country,
        },
        justification: {
          summary: args.justification_summary,
          category: args.justification_category,
          task_id: args.justification_task_id,
          expected_value: args.justification_expected_value,
        },
        preferred_rail: args.preferred_rail,
        idempotency_key,
      };

      try {
        const payment = await api.initiatePayment(requestBody);
        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify(payment, null, 2),
            },
          ],
        };
      } catch (error) {
        return {
          isError: true,
          content: [
            {
              type: "text" as const,
              text: formatError(error),
            },
          ],
        };
      }
    },
  );
}

function formatError(error: unknown): string {
  if (error instanceof ApiError) {
    return `Payment API error [${error.errorCode}] (HTTP ${error.status}): ${error.message}`;
  }
  return error instanceof Error ? error.message : String(error);
}
