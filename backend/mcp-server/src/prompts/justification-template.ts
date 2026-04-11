// payment_justification_template prompt.
//
// Guided template for producing a well-structured payment justification.
// The agent provides task context, and the prompt returns a user/assistant
// message pair that the agent can use as a scaffold for the justification
// fields passed to initiate_payment.

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";

export function registerJustificationTemplatePrompt(server: McpServer): void {
  server.registerPrompt(
    "payment_justification_template",
    {
      title: "Payment Justification Template",
      description:
        "A guided template for writing a well-structured payment justification. " +
        "Provide the task context and the prompt returns a justification ready to pass to initiate_payment.",
      argsSchema: {
        task_description: z
          .string()
          .describe("What task or workflow requires this payment?"),
        amount: z
          .string()
          .describe("Amount and currency (e.g. '$49.00 USD')."),
        vendor: z.string().describe("Vendor or recipient name."),
        expected_outcome: z
          .string()
          .describe("What does this payment enable or deliver?"),
      },
    },
    (args) => ({
      messages: [
        {
          role: "user" as const,
          content: {
            type: "text" as const,
            text:
              `I need to make a payment and want to write a strong justification.\n\n` +
              `Task: ${args.task_description}\n` +
              `Amount: ${args.amount}\n` +
              `Vendor: ${args.vendor}\n` +
              `Expected outcome: ${args.expected_outcome}\n\n` +
              `Please generate a concise justification summary (1-2 sentences, specific, business-focused) ` +
              `and select the most appropriate category from: ` +
              `saas_subscription, cloud_infrastructure, api_credits, travel, procurement, marketing, legal, other.`,
          },
        },
        {
          role: "assistant" as const,
          content: {
            type: "text" as const,
            text:
              `Based on the context provided, here is a structured justification:\n\n` +
              `**Summary:** Purchasing ${args.vendor} to support ${args.task_description}. ` +
              `This payment delivers: ${args.expected_outcome}.\n\n` +
              `**Category:** [Select the most appropriate from the list above]\n\n` +
              `**Expected value:** ${args.expected_outcome}\n\n` +
              `You can pass this directly to \`initiate_payment\` as the \`justification_summary\` ` +
              `and \`justification_expected_value\` fields.`,
          },
        },
      ],
    }),
  );
}
