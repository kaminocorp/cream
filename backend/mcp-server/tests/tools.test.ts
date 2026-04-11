// Tool handler tests.
//
// We test the registered handlers directly by intercepting registerTool on a
// minimal McpServer mock. This lets us exercise the argument construction
// logic (the most bug-prone part of each tool) without booting the full SDK
// transport layer. ApiClient is mocked per-test.

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { ApiClient } from "../src/api-client";
import { ApiError } from "../src/types";
import { registerInitiatePaymentTool } from "../src/tools/initiate-payment";
import { registerGetPaymentStatusTool } from "../src/tools/get-payment-status";
import { registerCheckProviderHealthTool } from "../src/tools/check-provider-health";

// Minimal McpServer mock — captures registerTool invocations so we can
// retrieve the handler function and call it directly.
function makeServer(): {
  server: McpServer;
  getHandler: (name: string) => (args?: unknown) => Promise<unknown>;
} {
  const handlers = new Map<
    string,
    (args?: unknown) => Promise<unknown>
  >();
  const server = {
    registerTool: jest.fn(
      (
        name: string,
        _config: unknown,
        handler: (args?: unknown) => Promise<unknown>,
      ) => {
        handlers.set(name, handler);
      },
    ),
  } as unknown as McpServer;
  return {
    server,
    getHandler: (name: string) => {
      const h = handlers.get(name);
      if (!h) throw new Error(`Tool '${name}' not registered`);
      return h;
    },
  };
}

// Minimal ApiClient mock.
function makeApi(overrides: Partial<ApiClient> = {}): ApiClient {
  return {
    initiatePayment: jest.fn(),
    getPayment: jest.fn(),
    getAgentPolicy: jest.fn(),
    queryAudit: jest.fn(),
    createCard: jest.fn(),
    getProviderHealth: jest.fn(),
    ...overrides,
  } as unknown as ApiClient;
}

// Helper to extract text from the first content block.
function firstText(result: unknown): string {
  const r = result as {
    content: Array<{ type: string; text: string }>;
    isError?: boolean;
  };
  return r.content[0].text;
}
function isError(result: unknown): boolean | undefined {
  return (result as { isError?: boolean }).isError;
}

describe("initiate_payment tool", () => {
  it("returns payment JSON on success", async () => {
    const { server, getHandler } = makeServer();
    const payment = { id: "pay_123", status: "submitted" };
    const api = makeApi({
      initiatePayment: jest.fn().mockResolvedValue(payment),
    });

    registerInitiatePaymentTool(server, api);
    const handler = getHandler("initiate_payment");

    const result = await handler({
      amount: "100.00",
      currency: "USD",
      recipient_type: "merchant",
      recipient_identifier: "aws.amazon.com",
      justification_summary: "Paying for cloud compute for ML pipeline",
      justification_category: "cloud_infrastructure",
      preferred_rail: "auto",
    });

    expect(isError(result)).toBeUndefined();
    expect(firstText(result)).toContain("pay_123");
    expect(api.initiatePayment).toHaveBeenCalledWith(
      expect.objectContaining({
        amount: "100.00",
        currency: "USD",
      }),
    );
  });

  it("auto-generates idempotency_key when not provided", async () => {
    const { server, getHandler } = makeServer();
    const api = makeApi({
      initiatePayment: jest
        .fn()
        .mockResolvedValue({ id: "pay_456", status: "submitted" }),
    });

    registerInitiatePaymentTool(server, api);
    const handler = getHandler("initiate_payment");

    await handler({
      amount: "50.00",
      currency: "SGD",
      recipient_type: "merchant",
      recipient_identifier: "stripe.com",
      justification_summary: "Monthly SaaS subscription renewal payment",
      justification_category: "saas_subscription",
      preferred_rail: "card",
    });

    const body = (api.initiatePayment as jest.Mock).mock.calls[0][0];
    expect(typeof body.idempotency_key).toBe("string");
    expect(body.idempotency_key.length).toBeGreaterThan(0);
  });

  it("preserves user-supplied idempotency_key when provided", async () => {
    const { server, getHandler } = makeServer();
    const api = makeApi({
      initiatePayment: jest
        .fn()
        .mockResolvedValue({ id: "pay_789", status: "submitted" }),
    });

    registerInitiatePaymentTool(server, api);
    const handler = getHandler("initiate_payment");

    await handler({
      amount: "10.00",
      currency: "USD",
      recipient_type: "merchant",
      recipient_identifier: "example.com",
      justification_summary: "Testing explicit idempotency key behaviour",
      justification_category: "other",
      preferred_rail: "auto",
      idempotency_key: "idem_explicit_123",
    });

    const body = (api.initiatePayment as jest.Mock).mock.calls[0][0];
    expect(body.idempotency_key).toBe("idem_explicit_123");
  });

  it("returns isError:true when API throws ApiError", async () => {
    const { server, getHandler } = makeServer();
    const api = makeApi({
      initiatePayment: jest
        .fn()
        .mockRejectedValue(
          new ApiError(
            403,
            "POLICY_BLOCKED",
            "Payment blocked by policy rule: spend limit exceeded",
          ),
        ),
    });

    registerInitiatePaymentTool(server, api);
    const handler = getHandler("initiate_payment");

    const result = await handler({
      amount: "99999.00",
      currency: "USD",
      recipient_type: "merchant",
      recipient_identifier: "expensive.com",
      justification_summary: "Buying a very expensive thing for testing",
      justification_category: "procurement",
      preferred_rail: "auto",
    });

    expect(isError(result)).toBe(true);
    expect(firstText(result)).toContain("POLICY_BLOCKED");
    expect(firstText(result)).toContain("spend limit exceeded");
  });
});

describe("get_payment_status tool", () => {
  it("returns payment detail JSON on success", async () => {
    const { server, getHandler } = makeServer();
    const detail = {
      payment: { id: "pay_789", status: "settled" },
      audit_entries: [],
    };
    const api = makeApi({
      getPayment: jest.fn().mockResolvedValue(detail),
    });

    registerGetPaymentStatusTool(server, api);
    const result = await getHandler("get_payment_status")({
      payment_id: "pay_789",
    });

    expect(isError(result)).toBeUndefined();
    expect(firstText(result)).toContain("settled");
  });

  it("returns isError:true on 404", async () => {
    const { server, getHandler } = makeServer();
    const api = makeApi({
      getPayment: jest
        .fn()
        .mockRejectedValue(
          new ApiError(404, "NOT_FOUND", "Payment not found"),
        ),
    });

    registerGetPaymentStatusTool(server, api);
    const result = await getHandler("get_payment_status")({
      payment_id: "pay_missing",
    });

    expect(isError(result)).toBe(true);
    expect(firstText(result)).toContain("NOT_FOUND");
  });
});

describe("check_provider_health tool", () => {
  it("returns health data as JSON", async () => {
    const { server, getHandler } = makeServer();
    const health = [
      { provider_id: "stripe", is_healthy: true, circuit_state: "closed" },
    ];
    const api = makeApi({
      getProviderHealth: jest.fn().mockResolvedValue(health),
    });

    registerCheckProviderHealthTool(server, api);
    const result = await getHandler("check_provider_health")();

    expect(firstText(result)).toContain("stripe");
    expect(firstText(result)).toContain("closed");
  });

  it("returns isError:true when API is unreachable", async () => {
    const { server, getHandler } = makeServer();
    const api = makeApi({
      getProviderHealth: jest
        .fn()
        .mockRejectedValue(new Error("ECONNREFUSED")),
    });

    registerCheckProviderHealthTool(server, api);
    const result = await getHandler("check_provider_health")();

    expect(isError(result)).toBe(true);
    expect(firstText(result)).toContain("ECONNREFUSED");
  });
});
