// agent://balance/{wallet_id} resource — STUB.
//
// No GET /v1/wallets/{id}/balance endpoint exists in the Phase 8 API yet.
// This resource is registered so MCP clients can discover the planned URI
// scheme, but returns a stub response explaining the gap. When the API
// endpoint is added (Phase 11+), this handler should be wired to call
// api.getWalletBalance(walletId) and return the real data.

import { McpServer, ResourceTemplate } from "@modelcontextprotocol/sdk/server/mcp.js";

export function registerBalanceResource(server: McpServer): void {
  server.registerResource(
    "agent-balance",
    new ResourceTemplate("agent://balance/{wallet_id}", { list: undefined }),
    {
      description:
        "[Stub — pending API endpoint] Balance per wallet/rail for a given wallet ID. " +
        "No balance endpoint exists in the Phase 8 API — this resource is a placeholder " +
        "and will return a stub response until the endpoint is implemented.",
      mimeType: "application/json",
    },
    async (uri, variables) => {
      const walletId = variables.wallet_id as string;
      return {
        contents: [
          {
            uri: uri.toString(),
            text: JSON.stringify({
              wallet_id: walletId,
              status: "stub",
              message:
                "Balance lookup is not yet implemented. " +
                "This resource will return real data once GET /v1/wallets/{id}/balance is added to the Cream API.",
            }),
            mimeType: "application/json",
          },
        ],
      };
    },
  );
}
