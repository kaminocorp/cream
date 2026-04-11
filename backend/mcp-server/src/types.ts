// Cream MCP Server — TypeScript types mirroring the Rust API's JSON responses.
//
// Intentionally a subset of the full Rust domain model: only the fields that
// tools and resources actually read are included. The authoritative shape
// lives in backend/crates/models/. Fields using monetary amounts are always
// `string` (Rust Decimal → string serialization) — never parse to float.

// ---------------------------------------------------------------------------
// Payment types
// ---------------------------------------------------------------------------

export type PaymentStatus =
  | "pending"
  | "validating"
  | "pending_approval"
  | "approved"
  | "submitted"
  | "settled"
  | "failed"
  | "blocked"
  | "rejected"
  | "timed_out";

export interface PaymentRequest {
  agent_id: string;
  amount: string; // Decimal as string — never parse to float
  currency: string;
  recipient: {
    type: "merchant" | "individual" | "wallet" | "bank_account";
    identifier: string;
    name?: string;
    country?: string;
  };
  preferred_rail: string;
  justification: {
    summary: string;
    category: string | { other: string };
    task_id?: string;
    expected_value?: string;
  };
  idempotency_key: string;
}

export interface PaymentResponse {
  id: string;
  request: PaymentRequest;
  status: PaymentStatus;
  provider_id?: string;
  provider_transaction_id?: string;
  created_at: string;
  updated_at: string;
}

export interface PaymentDetail {
  payment: PaymentResponse;
  audit_entries: AuditEntry[];
}

// ---------------------------------------------------------------------------
// Policy types
// ---------------------------------------------------------------------------

export interface AgentProfile {
  id: string;
  name: string;
  max_per_transaction?: string;
  max_daily_spend?: string;
  max_weekly_spend?: string;
  max_monthly_spend?: string;
  allowed_categories: string[];
  allowed_rails: string[];
  geographic_restrictions: string[];
  escalation_threshold?: string;
  timezone?: string;
}

export interface PolicyRule {
  id: string;
  rule_type?: string;
  priority: number;
  action: "APPROVE" | "BLOCK" | "ESCALATE";
  enabled: boolean;
}

export interface AgentPolicyResponse {
  agent: {
    id: string;
    name: string;
    status: string;
    profile_id: string;
  };
  profile: AgentProfile;
  rules: PolicyRule[];
}

// ---------------------------------------------------------------------------
// Audit types
// ---------------------------------------------------------------------------

export interface AuditEntry {
  id: string;
  timestamp: string;
  agent_id: string;
  payment_id?: string;
  final_status: PaymentStatus;
  policy_evaluation: {
    final_decision: "APPROVE" | "BLOCK" | "ESCALATE";
    decision_latency_ms: number;
  };
}

// ---------------------------------------------------------------------------
// Provider types
// ---------------------------------------------------------------------------

export interface ProviderHealth {
  provider_id: string;
  is_healthy: boolean;
  error_rate_5m: number;
  p50_latency_ms: number;
  p99_latency_ms: number;
  circuit_state: "closed" | "open" | "half_open";
}

// ---------------------------------------------------------------------------
// Virtual card types
// ---------------------------------------------------------------------------

export interface VirtualCard {
  id: string;
  agent_id: string;
  card_type: "single_use" | "multi_use";
  status: "active" | "frozen" | "cancelled" | "expired";
  controls: {
    currency: string;
    max_per_transaction?: string;
    max_per_cycle?: string;
    allowed_mcc_codes: string[];
  };
  created_at: string;
  expires_at?: string;
}

// ---------------------------------------------------------------------------
// Error shape (matches Rust ApiError JSON response)
// ---------------------------------------------------------------------------

export interface ApiErrorBody {
  error_code: string;
  message: string;
  details?: Record<string, unknown>;
}

export class ApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly errorCode: string,
    message: string,
  ) {
    super(message);
    this.name = "ApiError";
  }
}
