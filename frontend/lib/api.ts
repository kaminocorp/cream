import {
  AgentPolicyResponse,
  AgentSummary,
  AuditEntry,
  AuditQueryFilters,
  ApiError,
  ApiErrorResponse,
  CreateAgentRequest,
  CreateAgentResponse,
  PaymentDetail,
  PaymentResponse,
  ProviderHealth,
  RotateKeyResponse,
  UpdateAgentRequest,
  VirtualCard,
  CardControls,
  CardType,
  WebhookResponse,
} from "./types";

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

async function request<T>(
  baseUrl: string,
  apiKey: string,
  method: string,
  path: string,
  body?: unknown,
): Promise<T> {
  const url = `${baseUrl}${path}`;

  const res = await fetch(url, {
    method,
    headers: {
      "Authorization": `Bearer ${apiKey}`,
      "Content-Type": "application/json",
      "Accept": "application/json",
    },
    body: body !== undefined ? JSON.stringify(body) : undefined,
    // Opt out of caching for real-time dashboard data.
    cache: "no-store",
  });

  if (!res.ok) {
    let errorBody: ApiErrorResponse;
    try {
      errorBody = await res.json();
    } catch {
      throw new ApiError(res.status, "UNKNOWN", `HTTP ${res.status}`);
    }
    throw new ApiError(res.status, errorBody.error_code, errorBody.message);
  }

  // 204 No Content — return undefined cast to T.
  if (res.status === 204) return undefined as T;

  return res.json() as Promise<T>;
}

// ---------------------------------------------------------------------------
// CreamApiClient
// ---------------------------------------------------------------------------

export class CreamApiClient {
  private baseUrl: string;
  private apiKey: string;

  constructor(baseUrl: string, apiKey: string) {
    if (!baseUrl) throw new Error("NEXT_PUBLIC_API_URL is required");
    if (!apiKey) throw new Error("CREAM_API_KEY is required");
    this.baseUrl = baseUrl.replace(/\/$/, "");
    this.apiKey = apiKey;
  }

  // --- Payments ---

  async initiatePayment(req: unknown): Promise<PaymentResponse> {
    return request(this.baseUrl, this.apiKey, "POST", "/v1/payments", req);
  }

  async getPayment(id: string): Promise<PaymentDetail> {
    return request(this.baseUrl, this.apiKey, "GET", `/v1/payments/${id}`);
  }

  async approvePayment(id: string, reviewerId: string, reason?: string): Promise<PaymentResponse> {
    return request(this.baseUrl, this.apiKey, "POST", `/v1/payments/${id}/approve`, {
      reviewer_id: reviewerId,
      reason,
    });
  }

  async rejectPayment(id: string, reviewerId: string, reason?: string): Promise<PaymentResponse> {
    return request(this.baseUrl, this.apiKey, "POST", `/v1/payments/${id}/reject`, {
      reviewer_id: reviewerId,
      reason,
    });
  }

  // --- Agents ---

  /**
   * List all agents with their profile summary. Operator-only
   * (`OPERATOR_API_KEY` required). Returns up to 500 agents ordered
   * `created_at DESC`.
   */
  async listAgents(): Promise<AgentSummary[]> {
    return request(this.baseUrl, this.apiKey, "GET", "/v1/agents");
  }

  /**
   * Create a new agent with a freshly minted API key. Operator-only.
   *
   * The response body contains the plaintext `api_key` EXACTLY ONCE —
   * the backend persists only its SHA-256 hash, so there is no code path
   * that can retrieve it again. Callers MUST surface the key to the
   * operator via a one-shot copy-to-clipboard UX.
   */
  async createAgent(req: CreateAgentRequest): Promise<CreateAgentResponse> {
    return request(this.baseUrl, this.apiKey, "POST", "/v1/agents", req);
  }

  /**
   * Update an agent's name, status, or profile assignment. Operator-only.
   * All fields optional; only those present are changed (COALESCE semantics
   * on the server).
   */
  async updateAgent(
    agentId: string,
    update: UpdateAgentRequest,
  ): Promise<AgentSummary> {
    return request(this.baseUrl, this.apiKey, "PATCH", `/v1/agents/${agentId}`, update);
  }

  /**
   * Rotate an agent's API key. Operator-only. Generates a new key,
   * persists its hash, invalidates the old key, and returns the new
   * plaintext EXACTLY ONCE. Same one-shot UX as `createAgent`.
   */
  async rotateAgentKey(agentId: string): Promise<RotateKeyResponse> {
    return request(
      this.baseUrl,
      this.apiKey,
      "POST",
      `/v1/agents/${agentId}/rotate-key`,
    );
  }

  async getAgentPolicy(agentId: string): Promise<AgentPolicyResponse> {
    return request(this.baseUrl, this.apiKey, "GET", `/v1/agents/${agentId}/policy`);
  }

  async updateAgentPolicy(agentId: string, update: unknown): Promise<AgentPolicyResponse> {
    return request(this.baseUrl, this.apiKey, "PUT", `/v1/agents/${agentId}/policy`, update);
  }

  // --- Audit ---

  async queryAudit(filters: AuditQueryFilters = {}): Promise<AuditEntry[]> {
    const params = new URLSearchParams();
    if (filters.from)        params.set("from", filters.from);
    if (filters.to)          params.set("to", filters.to);
    if (filters.status)      params.set("status", filters.status);
    if (filters.category)    params.set("category", filters.category);
    if (filters.min_amount)  params.set("min_amount", filters.min_amount);
    if (filters.max_amount)  params.set("max_amount", filters.max_amount);
    if (filters.q)           params.set("q", filters.q);
    if (filters.agent_id)    params.set("agent_id", filters.agent_id);
    if (filters.limit)       params.set("limit", filters.limit.toString());
    if (filters.offset)      params.set("offset", filters.offset.toString());
    const qs = params.toString();
    return request(this.baseUrl, this.apiKey, "GET", `/v1/audit${qs ? `?${qs}` : ""}`);
  }

  // --- Virtual Cards ---

  async createCard(config: {
    agent_id: string;
    card_type: CardType;
    controls: CardControls;
    provider_id: string;
  }): Promise<VirtualCard> {
    return request(this.baseUrl, this.apiKey, "POST", "/v1/cards", config);
  }

  async updateCard(cardId: string, controls: Partial<CardControls>): Promise<VirtualCard> {
    return request(this.baseUrl, this.apiKey, "PATCH", `/v1/cards/${cardId}`, { controls });
  }

  async cancelCard(cardId: string): Promise<void> {
    return request(this.baseUrl, this.apiKey, "DELETE", `/v1/cards/${cardId}`);
  }

  // --- Providers ---

  async getProviderHealth(): Promise<ProviderHealth[]> {
    return request(this.baseUrl, this.apiKey, "GET", "/v1/providers/health");
  }

  // --- Webhooks ---

  async registerWebhook(config: {
    url: string;
    events?: string[];
    secret: string;
  }): Promise<WebhookResponse> {
    return request(this.baseUrl, this.apiKey, "POST", "/v1/webhooks", config);
  }
}

// ---------------------------------------------------------------------------
// Singleton factory — import this in server components
// ---------------------------------------------------------------------------

let _client: CreamApiClient | null = null;

export function getApiClient(): CreamApiClient {
  if (!_client) {
    _client = new CreamApiClient(
      process.env.NEXT_PUBLIC_API_URL ?? "",
      process.env.CREAM_API_KEY ?? "",
    );
  }
  return _client;
}
