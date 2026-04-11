// ApiClient tests.
//
// Mocks global fetch and verifies: Bearer auth header, JSON error parsing,
// unparseable error fallback, query string construction for audit filters.

import { ApiClient } from "../src/api-client";
import { ApiError } from "../src/types";

// Mock global fetch.
const mockFetch = jest.fn();
(global as unknown as { fetch: jest.Mock }).fetch = mockFetch;

const testConfig = {
  apiBaseUrl: "http://localhost:8080",
  apiKey: "test-key",
  transport: "stdio" as const,
  httpPort: 3002,
};

describe("ApiClient", () => {
  beforeEach(() => {
    mockFetch.mockReset();
  });

  it("sends correct Authorization header", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => ({ id: "pay_123" }),
    });

    const client = new ApiClient(testConfig);
    await client.getPayment("pay_123");

    expect(mockFetch).toHaveBeenCalledWith(
      "http://localhost:8080/v1/payments/pay_123",
      expect.objectContaining({
        headers: expect.objectContaining({
          Authorization: "Bearer test-key",
        }),
      }),
    );
  });

  it("throws ApiError on non-2xx response with error body", async () => {
    mockFetch.mockResolvedValue({
      ok: false,
      status: 404,
      json: async () => ({
        error_code: "NOT_FOUND",
        message: "Payment not found",
      }),
    });

    const client = new ApiClient(testConfig);
    await expect(client.getPayment("pay_unknown")).rejects.toThrow(ApiError);
    await expect(client.getPayment("pay_unknown")).rejects.toMatchObject({
      status: 404,
      errorCode: "NOT_FOUND",
    });
  });

  it("throws ApiError with UNKNOWN code when error body is unparseable", async () => {
    mockFetch.mockResolvedValue({
      ok: false,
      status: 502,
      statusText: "Bad Gateway",
      json: async () => {
        throw new Error("not json");
      },
    });

    const client = new ApiClient(testConfig);
    await expect(client.getPayment("pay_123")).rejects.toMatchObject({
      status: 502,
      errorCode: "UNKNOWN",
    });
  });

  it("returns undefined for 204 No Content", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 204,
      json: async () => {
        throw new Error("should not be called");
      },
    });

    const client = new ApiClient(testConfig);
    // getPayment always returns PaymentDetail in practice; this test just
    // exercises the 204 codepath via a raw internal call — we use a method
    // that happens to hit it: any method works here.
    const result = await client.getPayment("pay_204");
    expect(result).toBeUndefined();
  });

  it("builds correct query string for audit filters", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => [],
    });

    const client = new ApiClient(testConfig);
    await client.queryAudit({ status: "settled", limit: 10 });

    const calledUrl = mockFetch.mock.calls[0][0] as string;
    expect(calledUrl).toContain("status=settled");
    expect(calledUrl).toContain("limit=10");
  });

  it("omits query string when no filters", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => [],
    });

    const client = new ApiClient(testConfig);
    await client.queryAudit({});

    const calledUrl = mockFetch.mock.calls[0][0] as string;
    expect(calledUrl).toBe("http://localhost:8080/v1/audit");
  });
});
