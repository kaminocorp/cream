// check_provider_health tool — GET /v1/providers/health
//
// Zero-argument tool. Returns real-time health status for all registered
// payment providers: circuit breaker state, 5-minute error rate, latency
// percentiles. Agents can use this to understand why a specific provider
// was selected (or demoted) by the router, or to decide whether to retry
// after a failed payment.

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { ApiClient } from "../api-client";
import { ApiError } from "../types";

export function registerCheckProviderHealthTool(
  server: McpServer,
  api: ApiClient,
): void {
  server.registerTool(
    "check_provider_health",
    {
      title: "Check Provider Health",
      description:
        "Retrieve real-time health status for all registered payment providers. " +
        "Shows circuit breaker state (closed/open/half_open), 5-minute error rate, " +
        "and p50/p99 latency. " +
        "Use this to understand why a payment might have been routed to a specific provider, " +
        "or to investigate provider-side failures before retrying.",
      // No input schema — zero-argument tool.
    },
    async () => {
      try {
        const health = await api.getProviderHealth();
        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify(health, null, 2),
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
