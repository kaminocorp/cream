// Cream MCP Server — typed HTTP client for the Rust REST API.
//
// Thin fetch wrapper: Bearer auth header, JSON request/response, ApiError
// on non-2xx. Used exclusively by tool and resource handlers. This layer has
// zero business logic — all payment processing, policy evaluation, and
// routing happens in the Rust API. The client's only job is to turn method
// calls into HTTP requests and responses into typed objects.

import { Config } from "./config";
import {
  AgentPolicyResponse,
  ApiError,
  ApiErrorBody,
  AuditEntry,
  PaymentDetail,
  PaymentResponse,
  ProviderHealth,
  VirtualCard,
} from "./types";

export class ApiClient {
  constructor(private readonly config: Config) {}

  // ---------------------------------------------------------------------------
  // Core request helper
  // ---------------------------------------------------------------------------

  private async request<T>(
    method: string,
    path: string,
    body?: unknown,
  ): Promise<T> {
    const url = `${this.config.apiBaseUrl}${path}`;

    const res = await fetch(url, {
      method,
      headers: {
        Authorization: `Bearer ${this.config.apiKey}`,
        "Content-Type": "application/json",
        Accept: "application/json",
      },
      body: body !== undefined ? JSON.stringify(body) : undefined,
    });

    if (!res.ok) {
      let errorBody: ApiErrorBody;
      try {
        errorBody = (await res.json()) as ApiErrorBody;
      } catch {
        throw new ApiError(
          res.status,
          "UNKNOWN",
          `HTTP ${res.status} ${res.statusText}`,
        );
      }
      throw new ApiError(res.status, errorBody.error_code, errorBody.message);
    }

    if (res.status === 204) return undefined as T;
    return res.json() as Promise<T>;
  }

  // ---------------------------------------------------------------------------
  // Payments
  // ---------------------------------------------------------------------------

  initiatePayment(req: unknown): Promise<PaymentResponse> {
    return this.request("POST", "/v1/payments", req);
  }

  getPayment(id: string): Promise<PaymentDetail> {
    return this.request("GET", `/v1/payments/${id}`);
  }

  // ---------------------------------------------------------------------------
  // Agents
  // ---------------------------------------------------------------------------

  getAgentPolicy(agentId: string): Promise<AgentPolicyResponse> {
    return this.request("GET", `/v1/agents/${agentId}/policy`);
  }

  // ---------------------------------------------------------------------------
  // Audit
  // ---------------------------------------------------------------------------

  queryAudit(
    filters: Record<string, string | number>,
  ): Promise<AuditEntry[]> {
    const params = new URLSearchParams();
    for (const [k, v] of Object.entries(filters)) {
      if (v !== undefined && v !== null) params.set(k, String(v));
    }
    const qs = params.toString();
    return this.request("GET", `/v1/audit${qs ? `?${qs}` : ""}`);
  }

  // ---------------------------------------------------------------------------
  // Virtual cards
  // ---------------------------------------------------------------------------

  createCard(req: unknown): Promise<VirtualCard> {
    return this.request("POST", "/v1/cards", req);
  }

  // ---------------------------------------------------------------------------
  // Providers
  // ---------------------------------------------------------------------------

  getProviderHealth(): Promise<ProviderHealth[]> {
    return this.request("GET", "/v1/providers/health");
  }
}

export function createApiClient(config: Config): ApiClient {
  return new ApiClient(config);
}
