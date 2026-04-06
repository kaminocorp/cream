// ---------------------------------------------------------------------------
// Typed ID aliases — opaque strings with known prefixes
// ---------------------------------------------------------------------------
export type PaymentId = string;       // "pay_..."
export type AgentId = string;         // "agt_..."
export type AgentProfileId = string;  // "prof_..."
export type PolicyRuleId = string;    // "rule_..."
export type AuditEntryId = string;    // "aud_..."
export type VirtualCardId = string;   // "card_..."
export type WebhookEndpointId = string; // "whk_..."

// ---------------------------------------------------------------------------
// Enums — values match Rust serde serialization exactly (snake_case / SCREAMING_SNAKE_CASE)
// ---------------------------------------------------------------------------

// Matches PaymentStatus DB serialization (snake_case)
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

export const TERMINAL_STATUSES: PaymentStatus[] = [
  "settled", "failed", "blocked", "rejected", "timed_out",
];

export type Currency =
  // Fiat (ISO 4217)
  | "USD" | "EUR" | "GBP" | "SGD" | "JPY" | "CNY" | "HKD" | "AUD" | "CAD"
  | "INR" | "KRW" | "TWD" | "THB" | "MYR" | "IDR" | "PHP" | "VND" | "BRL"
  | "MXN" | "CHF" | "SEK" | "NOK" | "DKK" | "NZD" | "AED"
  // Crypto
  | "BTC" | "ETH" | "USDC" | "USDT" | "SOL" | "MATIC" | "AVAX" | "BASE_ETH";

export type RailPreference = "auto" | "card" | "ach" | "swift" | "local" | "stablecoin";

export type PolicyAction = "APPROVE" | "BLOCK" | "ESCALATE";

export type PaymentCategory =
  | "saas_subscription"
  | "cloud_infrastructure"
  | "api_credits"
  | "travel"
  | "procurement"
  | "marketing"
  | "legal"
  | "other";

export type CardType = "single_use" | "multi_use";
export type CardStatus = "active" | "frozen" | "cancelled" | "expired";
export type AgentStatus = "active" | "suspended" | "revoked";
export type CircuitState = "closed" | "open" | "half_open";

// ---------------------------------------------------------------------------
// Core domain types
// ---------------------------------------------------------------------------

export interface Recipient {
  type: "merchant" | "individual" | "wallet" | "bank_account";
  identifier: string;
  name?: string;
  country?: string;
}

export interface Justification {
  summary: string;
  task_id?: string;
  category: PaymentCategory | { other: string };
  expected_value?: string;
}

export interface PaymentMetadata {
  agent_session_id?: string;
  workflow_id?: string;
  operator_ref?: string;
}

export interface PaymentRequest {
  agent_id: AgentId;
  amount: string;               // Decimal serialized as string — never parse to float
  currency: Currency;
  recipient: Recipient;
  preferred_rail: RailPreference;
  justification: Justification;
  metadata?: PaymentMetadata;
  idempotency_key: string;
}

export interface PaymentResponse {
  id: PaymentId;
  request: PaymentRequest;
  status: PaymentStatus;
  provider_id?: string;
  provider_transaction_id?: string;
  created_at: string;           // ISO 8601
  updated_at: string;           // ISO 8601
}

export interface PaymentDetail {
  payment: PaymentResponse;
  audit_entries: AuditEntry[];
}

// ---------------------------------------------------------------------------
// Policy types
// ---------------------------------------------------------------------------

export interface PolicyEvaluationRecord {
  rules_evaluated: PolicyRuleId[];
  matching_rules: PolicyRuleId[];
  final_decision: PolicyAction;
  decision_latency_ms: number;
}

export interface AgentProfile {
  id: AgentProfileId;
  name: string;
  version: number;
  max_per_transaction?: string;   // Decimal as string
  max_daily_spend?: string;
  max_weekly_spend?: string;
  max_monthly_spend?: string;
  allowed_categories: PaymentCategory[];
  allowed_rails: RailPreference[];
  geographic_restrictions: string[];  // CountryCode[]
  escalation_threshold?: string;
  timezone?: string;
  created_at: string;
  updated_at: string;
}

export interface Agent {
  id: AgentId;
  profile_id: AgentProfileId;
  name: string;
  status: AgentStatus;
  created_at: string;
  updated_at: string;
}

export interface AgentPolicyResponse {
  agent: Agent;
  profile: AgentProfile;
  rules: PolicyRule[];
}

export interface PolicyRule {
  id: PolicyRuleId;
  profile_id: AgentProfileId;
  rule_type?: string;
  priority: number;
  condition: PolicyCondition;
  action: PolicyAction;
  escalation?: EscalationConfig;
  enabled: boolean;
}

export type PolicyCondition =
  | { All: PolicyCondition[] }
  | { Any: PolicyCondition[] }
  | { Not: PolicyCondition }
  | { FieldCheck: { field: string; op: string; value: unknown } };

export interface EscalationConfig {
  channel: "slack" | "email" | "webhook" | "dashboard";
  timeout_minutes: number;
  on_timeout: PolicyAction;
}

// ---------------------------------------------------------------------------
// Routing types
// ---------------------------------------------------------------------------

export interface RoutingCandidate {
  provider_id: string;
  rail: RailPreference;
  estimated_fee: string;
  estimated_latency_ms: number;
  score: number;
}

export interface RoutingDecision {
  candidates: RoutingCandidate[];
  selected?: RoutingCandidate;
  selected_rail?: RailPreference;
  reason?: string;
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
  circuit_state: CircuitState;
}

// ---------------------------------------------------------------------------
// Audit types
// ---------------------------------------------------------------------------

export interface ProviderResponseRecord {
  provider: string;
  transaction_id: string;
  status: string;
  amount_settled: string;
  currency: Currency;
  latency_ms: number;
}

export interface HumanReviewRecord {
  reviewer_id: string;
  decision: PolicyAction;
  reason?: string;
  decided_at: string;
}

export interface AuditEntry {
  id: AuditEntryId;
  timestamp: string;
  agent_id: AgentId;
  agent_profile_id: AgentProfileId;
  payment_id?: PaymentId;
  request: unknown;             // Full PaymentRequest JSON blob
  justification: unknown;       // Full Justification JSON blob
  policy_evaluation: PolicyEvaluationRecord;
  routing_decision?: RoutingDecision;
  provider_response?: ProviderResponseRecord;
  final_status: PaymentStatus;
  human_review?: HumanReviewRecord;
  on_chain_tx_hash?: string;
}

// ---------------------------------------------------------------------------
// Virtual card types
// ---------------------------------------------------------------------------

export interface CardControls {
  max_per_transaction?: string;
  max_per_cycle?: string;
  allowed_mcc_codes: string[];
  currency: Currency;
}

export interface VirtualCard {
  id: VirtualCardId;
  agent_id: AgentId;
  provider_id: string;
  provider_card_id: string;
  card_type: CardType;
  controls: CardControls;
  status: CardStatus;
  created_at: string;
  expires_at?: string;
}

// ---------------------------------------------------------------------------
// Webhook types
// ---------------------------------------------------------------------------

export interface WebhookResponse {
  id: WebhookEndpointId;
  url: string;
  events: string[];
  status: string;
}

// ---------------------------------------------------------------------------
// API error shape (matches ApiError JSON response from Rust)
// ---------------------------------------------------------------------------

export interface ApiErrorResponse {
  error_code: string;
  message: string;
  details?: Record<string, unknown>;
}

export class ApiError extends Error {
  constructor(
    public status: number,
    public error_code: string,
    message: string,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

// ---------------------------------------------------------------------------
// Query filter types (mirrors AuditQuery in audit crate)
// ---------------------------------------------------------------------------

export interface AuditQueryFilters {
  from?: string;
  to?: string;
  status?: PaymentStatus;
  category?: string;
  min_amount?: string;
  max_amount?: string;
  limit?: number;
  offset?: number;
}
