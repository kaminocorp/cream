// create_virtual_card tool — POST /v1/cards
//
// Issues a scoped virtual card for card-rail payments. Single-use cards are
// cancelled after one transaction; multi-use cards remain active until
// explicitly cancelled or expired. MCC restrictions enforce which merchant
// categories the card can be used at, matching the Stripe Issuing pattern.

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { ApiClient } from "../api-client";
import { ApiError } from "../types";

export function registerCreateVirtualCardTool(
  server: McpServer,
  api: ApiClient,
): void {
  server.registerTool(
    "create_virtual_card",
    {
      title: "Create Virtual Card",
      description:
        "Issue a scoped virtual card for making payments via card rails. " +
        "Single-use cards are cancelled after one transaction. " +
        "Multi-use cards remain active until cancelled or expired. " +
        "Card controls define the spending limits and allowed merchant categories.",
      inputSchema: {
        card_type: z
          .enum(["single_use", "multi_use"])
          .describe(
            "'single_use' for one-time payments; 'multi_use' for recurring spend.",
          ),
        currency: z.string().describe("Currency for the card (e.g. 'USD')."),
        provider_id: z
          .string()
          .describe(
            "Payment provider to issue the card through (e.g. 'stripe', 'airwallex').",
          ),
        max_per_transaction: z
          .string()
          .optional()
          .describe("Maximum amount per transaction as decimal string."),
        max_per_cycle: z
          .string()
          .optional()
          .describe(
            "Maximum total spend per billing cycle as decimal string.",
          ),
        allowed_mcc_codes: z
          .array(z.string())
          .default([])
          .describe(
            "Merchant Category Codes to allow. Empty array means all MCCs are permitted.",
          ),
      },
    },
    async (args) => {
      try {
        const card = await api.createCard({
          card_type: args.card_type,
          provider_id: args.provider_id,
          controls: {
            currency: args.currency,
            max_per_transaction: args.max_per_transaction,
            max_per_cycle: args.max_per_cycle,
            allowed_mcc_codes: args.allowed_mcc_codes,
          },
        });
        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify(card, null, 2),
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
