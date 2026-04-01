use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use tracing::instrument;

use cream_models::justification::PaymentCategory;
use cream_models::prelude::{
    AgentId, AuditEntry, AuditEntryId, HumanReviewRecord, PaymentId, PaymentStatus,
    PolicyEvaluationRecord, ProviderResponseRecord, RoutingDecision,
};

use crate::error::AuditError;

// ---------------------------------------------------------------------------
// Query filter
// ---------------------------------------------------------------------------

/// Filter parameters for querying the audit ledger.
///
/// All fields are optional — an empty `AuditQuery` returns the most recent
/// entries up to `limit`. The reader translates this into a parameterized
/// SQL query internally, preventing SQL injection and keeping query logic
/// centralized in the audit crate.
///
/// Fields are private to enforce clamped pagination bounds. Use the builder
/// methods to construct queries — `limit` is always clamped to 1000 and
/// `offset` to 100,000 regardless of what the caller requests.
#[derive(Debug, Clone, Default)]
pub struct AuditQuery {
    agent_id: Option<AgentId>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    status: Option<PaymentStatus>,
    category: Option<PaymentCategory>,
    min_amount: Option<Decimal>,
    max_amount: Option<Decimal>,
    limit: Option<u32>,
    offset: Option<u32>,
}

impl AuditQuery {
    /// Create a new empty query (returns most recent entries, default limit 50).
    pub fn new() -> Self {
        Self::default()
    }

    pub fn agent_id(mut self, id: AgentId) -> Self {
        self.agent_id = Some(id);
        self
    }

    pub fn from(mut self, from: DateTime<Utc>) -> Self {
        self.from = Some(from);
        self
    }

    pub fn to(mut self, to: DateTime<Utc>) -> Self {
        self.to = Some(to);
        self
    }

    pub fn status(mut self, status: PaymentStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn category(mut self, category: PaymentCategory) -> Self {
        self.category = Some(category);
        self
    }

    pub fn min_amount(mut self, amount: Decimal) -> Self {
        self.min_amount = Some(amount);
        self
    }

    pub fn max_amount(mut self, amount: Decimal) -> Self {
        self.max_amount = Some(amount);
        self
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Effective limit, defaulting to 50 and clamped to 1000.
    fn effective_limit(&self) -> i64 {
        self.limit.unwrap_or(50).min(1000) as i64
    }

    /// Effective offset, defaulting to 0 and clamped to 100_000 to prevent
    /// expensive full-table scans from unbounded pagination.
    fn effective_offset(&self) -> i64 {
        self.offset.unwrap_or(0).min(100_000) as i64
    }
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Read path for the audit ledger.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AuditReader: Send + Sync {
    /// Query audit entries with optional filters, ordered by timestamp descending.
    async fn query(&self, filters: AuditQuery) -> Result<Vec<AuditEntry>, AuditError>;

    /// Retrieve a single audit entry by its ID.
    async fn get_by_id(&self, id: AuditEntryId) -> Result<Option<AuditEntry>, AuditError>;

    /// Retrieve all audit entries associated with a payment.
    async fn get_by_payment(&self, payment_id: PaymentId) -> Result<Vec<AuditEntry>, AuditError>;
}

// ---------------------------------------------------------------------------
// PostgreSQL implementation
// ---------------------------------------------------------------------------

/// Reads audit entries from a PostgreSQL `audit_log` table.
pub struct PgAuditReader {
    pool: PgPool,
}

impl PgAuditReader {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

// ---------------------------------------------------------------------------
// Query builder helper — co-locates clauses with their bind values
// ---------------------------------------------------------------------------

/// A bind value that can be applied to a sqlx query.
/// Keeps clause SQL and bind value together so they can never get out of sync.
enum BindValue {
    Uuid(uuid::Uuid),
    Timestamp(DateTime<Utc>),
    String(String),
    Decimal(Decimal),
    Int64(i64),
}

/// Lightweight query builder that auto-increments bind parameter indices
/// and co-locates each clause with its bind value.
struct QueryBuilder {
    sql: String,
    bind_idx: u32,
    binds: Vec<BindValue>,
}

impl QueryBuilder {
    fn new(base: &str) -> Self {
        Self {
            sql: base.to_string(),
            bind_idx: 0,
            binds: Vec::new(),
        }
    }

    /// Push a WHERE clause with its bind value. Clause SQL and value are
    /// recorded together, preventing ordering mismatches.
    fn push_clause(&mut self, column: &str, value: BindValue) {
        self.bind_idx += 1;
        let trimmed = column.trim();
        if trimmed.ends_with(">=")
            || trimmed.ends_with("<=")
            || trimmed.ends_with('>')
            || trimmed.ends_with('<')
        {
            self.sql
                .push_str(&format!(" AND {} ${}", trimmed, self.bind_idx));
        } else {
            self.sql
                .push_str(&format!(" AND {} = ${}", trimmed, self.bind_idx));
        }
        self.binds.push(value);
    }

    fn push_limit_offset(&mut self, limit: i64, offset: i64) {
        self.bind_idx += 1;
        let limit_idx = self.bind_idx;
        self.bind_idx += 1;
        let offset_idx = self.bind_idx;
        self.sql.push_str(&format!(
            " ORDER BY timestamp DESC LIMIT ${limit_idx} OFFSET ${offset_idx}"
        ));
        self.binds.push(BindValue::Int64(limit));
        self.binds.push(BindValue::Int64(offset));
    }

    fn finish(self) -> (String, Vec<BindValue>) {
        (self.sql, self.binds)
    }
}

/// Serialize a serde-able enum to its string representation.
///
/// Returns an error instead of silently falling back — silent fallbacks would
/// cause audit queries to match wrong records (e.g., querying for "unknown"
/// instead of the intended status).
fn serialize_enum_to_string<T: serde::Serialize + std::fmt::Debug>(
    value: &T,
) -> Result<String, AuditError> {
    let json_val = serde_json::to_value(value)?;
    json_val
        .as_str()
        .map(|s| s.to_owned())
        .ok_or_else(|| {
            <serde_json::Error as serde::ser::Error>::custom(format!(
                "enum {value:?} serialized to non-string JSON value"
            ))
        })
        .map_err(AuditError::Serialization)
}

/// Raw row returned by SQLx before we map it to the domain type.
struct AuditRow {
    id: uuid::Uuid,
    timestamp: DateTime<Utc>,
    agent_id: uuid::Uuid,
    agent_profile_id: uuid::Uuid,
    request: serde_json::Value,
    justification: serde_json::Value,
    policy_evaluation: serde_json::Value,
    routing_decision: Option<serde_json::Value>,
    provider_response: Option<serde_json::Value>,
    final_status: String,
    human_review: Option<serde_json::Value>,
    on_chain_tx_hash: Option<String>,
}

impl AuditRow {
    fn into_entry(self) -> Result<AuditEntry, AuditError> {
        let policy_evaluation: PolicyEvaluationRecord =
            serde_json::from_value(self.policy_evaluation)?;
        let routing_decision: Option<RoutingDecision> = self
            .routing_decision
            .map(serde_json::from_value)
            .transpose()?;
        let provider_response: Option<ProviderResponseRecord> = self
            .provider_response
            .map(serde_json::from_value)
            .transpose()?;
        let human_review: Option<HumanReviewRecord> =
            self.human_review.map(serde_json::from_value).transpose()?;

        // PaymentStatus is serialized as a snake_case string by serde
        let final_status: PaymentStatus =
            serde_json::from_value(serde_json::Value::String(self.final_status))?;

        Ok(AuditEntry {
            id: AuditEntryId::from_uuid(self.id),
            timestamp: self.timestamp,
            agent_id: AgentId::from_uuid(self.agent_id),
            agent_profile_id: cream_models::prelude::AgentProfileId::from_uuid(
                self.agent_profile_id,
            ),
            request: self.request,
            justification: self.justification,
            policy_evaluation,
            routing_decision,
            provider_response,
            final_status,
            human_review,
            on_chain_tx_hash: self.on_chain_tx_hash,
        })
    }
}

#[async_trait]
impl AuditReader for PgAuditReader {
    #[instrument(skip(self, filters))]
    async fn query(&self, filters: AuditQuery) -> Result<Vec<AuditEntry>, AuditError> {
        // Build a dynamic query where each clause is co-located with its bind
        // value, preventing the fragile two-phase pattern where clause order
        // and bind order could silently diverge.
        let mut qb = QueryBuilder::new(
            "SELECT id, timestamp, agent_id, agent_profile_id, \
             request, justification, policy_evaluation, \
             routing_decision, provider_response, final_status, human_review, \
             on_chain_tx_hash \
             FROM audit_log WHERE true",
        );

        if let Some(ref agent_id) = filters.agent_id {
            qb.push_clause("agent_id", BindValue::Uuid(*agent_id.as_uuid()));
        }
        if let Some(ref from) = filters.from {
            qb.push_clause("timestamp >=", BindValue::Timestamp(*from));
        }
        if let Some(ref to) = filters.to {
            qb.push_clause("timestamp <=", BindValue::Timestamp(*to));
        }
        if let Some(ref status) = filters.status {
            let status_str = serialize_enum_to_string(status)?;
            qb.push_clause("final_status", BindValue::String(status_str));
        }
        if let Some(ref category) = filters.category {
            let cat_str = serialize_enum_to_string(category)?;
            qb.push_clause("justification->>'category'", BindValue::String(cat_str));
        }
        if let Some(ref min_amount) = filters.min_amount {
            qb.push_clause(
                "(request->>'amount')::numeric >=",
                BindValue::Decimal(*min_amount),
            );
        }
        if let Some(ref max_amount) = filters.max_amount {
            qb.push_clause(
                "(request->>'amount')::numeric <=",
                BindValue::Decimal(*max_amount),
            );
        }

        qb.push_limit_offset(filters.effective_limit(), filters.effective_offset());

        let (sql, binds) = qb.finish();

        // Apply all bind values in the order they were collected
        let mut query = sqlx::query_as::<
            _,
            (
                uuid::Uuid,
                DateTime<Utc>,
                uuid::Uuid,
                uuid::Uuid,
                serde_json::Value,
                serde_json::Value,
                serde_json::Value,
                Option<serde_json::Value>,
                Option<serde_json::Value>,
                String,
                Option<serde_json::Value>,
                Option<String>,
            ),
        >(&sql);

        for bind in binds {
            query = match bind {
                BindValue::Uuid(v) => query.bind(v),
                BindValue::Timestamp(v) => query.bind(v),
                BindValue::String(v) => query.bind(v),
                BindValue::Decimal(v) => query.bind(v),
                BindValue::Int64(v) => query.bind(v),
            };
        }

        let rows = query.fetch_all(&self.pool).await?;

        rows.into_iter()
            .map(|row| {
                let audit_row = AuditRow {
                    id: row.0,
                    timestamp: row.1,
                    agent_id: row.2,
                    agent_profile_id: row.3,
                    request: row.4,
                    justification: row.5,
                    policy_evaluation: row.6,
                    routing_decision: row.7,
                    provider_response: row.8,
                    final_status: row.9,
                    human_review: row.10,
                    on_chain_tx_hash: row.11,
                };
                audit_row.into_entry()
            })
            .collect()
    }

    #[instrument(skip(self))]
    async fn get_by_id(&self, id: AuditEntryId) -> Result<Option<AuditEntry>, AuditError> {
        let row = sqlx::query_as::<
            _,
            (
                uuid::Uuid,
                DateTime<Utc>,
                uuid::Uuid,
                uuid::Uuid,
                serde_json::Value,
                serde_json::Value,
                serde_json::Value,
                Option<serde_json::Value>,
                Option<serde_json::Value>,
                String,
                Option<serde_json::Value>,
                Option<String>,
            ),
        >(
            "SELECT id, timestamp, agent_id, agent_profile_id, \
             request, justification, policy_evaluation, \
             routing_decision, provider_response, final_status, human_review, \
             on_chain_tx_hash \
             FROM audit_log WHERE id = $1",
        )
        .bind(*id.as_uuid())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let audit_row = AuditRow {
                    id: row.0,
                    timestamp: row.1,
                    agent_id: row.2,
                    agent_profile_id: row.3,
                    request: row.4,
                    justification: row.5,
                    policy_evaluation: row.6,
                    routing_decision: row.7,
                    provider_response: row.8,
                    final_status: row.9,
                    human_review: row.10,
                    on_chain_tx_hash: row.11,
                };
                Ok(Some(audit_row.into_entry()?))
            }
            None => Ok(None),
        }
    }

    #[instrument(skip(self), fields(payment_id = %payment_id))]
    async fn get_by_payment(&self, payment_id: PaymentId) -> Result<Vec<AuditEntry>, AuditError> {
        let rows = sqlx::query_as::<
            _,
            (
                uuid::Uuid,
                DateTime<Utc>,
                uuid::Uuid,
                uuid::Uuid,
                serde_json::Value,
                serde_json::Value,
                serde_json::Value,
                Option<serde_json::Value>,
                Option<serde_json::Value>,
                String,
                Option<serde_json::Value>,
                Option<String>,
            ),
        >(
            "SELECT id, timestamp, agent_id, agent_profile_id, \
             request, justification, policy_evaluation, \
             routing_decision, provider_response, final_status, human_review, \
             on_chain_tx_hash \
             FROM audit_log WHERE payment_id = $1 ORDER BY timestamp DESC",
        )
        .bind(*payment_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let audit_row = AuditRow {
                    id: row.0,
                    timestamp: row.1,
                    agent_id: row.2,
                    agent_profile_id: row.3,
                    request: row.4,
                    justification: row.5,
                    policy_evaluation: row.6,
                    routing_decision: row.7,
                    provider_response: row.8,
                    final_status: row.9,
                    human_review: row.10,
                    on_chain_tx_hash: row.11,
                };
                audit_row.into_entry()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cream_models::prelude::*;

    #[test]
    fn audit_query_default_limit() {
        let q = AuditQuery::default();
        assert_eq!(q.effective_limit(), 50);
        assert_eq!(q.effective_offset(), 0);
    }

    #[test]
    fn audit_query_clamps_limit_to_1000() {
        let q = AuditQuery::new().limit(5000);
        assert_eq!(q.effective_limit(), 1000);
    }

    #[test]
    fn audit_query_clamps_offset_to_100k() {
        let q = AuditQuery::new().offset(500_000);
        assert_eq!(q.effective_offset(), 100_000);
    }

    #[test]
    fn audit_query_respects_custom_limit() {
        let q = AuditQuery::new().limit(200).offset(10);
        assert_eq!(q.effective_limit(), 200);
        assert_eq!(q.effective_offset(), 10);
    }

    #[test]
    fn audit_row_into_entry_roundtrip() {
        let agent_id = uuid::Uuid::now_v7();
        let profile_id = uuid::Uuid::now_v7();
        let entry_id = uuid::Uuid::now_v7();

        let policy_eval = PolicyEvaluationRecord {
            rules_evaluated: vec![],
            matching_rules: vec![],
            final_decision: PolicyAction::Approve,
            decision_latency_ms: 5,
        };

        let row = AuditRow {
            id: entry_id,
            timestamp: Utc::now(),
            agent_id,
            agent_profile_id: profile_id,
            request: serde_json::json!({"amount": "100.00", "currency": "sgd"}),
            justification: serde_json::json!({"summary": "test purchase", "category": "api_credits"}),
            policy_evaluation: serde_json::to_value(&policy_eval).unwrap(),
            routing_decision: None,
            provider_response: None,
            final_status: "settled".to_string(),
            human_review: None,
            on_chain_tx_hash: None,
        };

        let entry = row.into_entry().unwrap();
        assert_eq!(*entry.id.as_uuid(), entry_id);
        assert_eq!(*entry.agent_id.as_uuid(), agent_id);
        assert_eq!(entry.final_status, PaymentStatus::Settled);
        assert!(entry.routing_decision.is_none());
        assert!(entry.human_review.is_none());
    }

    #[test]
    fn audit_row_with_all_optional_fields() {
        use cream_models::prelude::*;
        use std::str::FromStr;

        let policy_eval = PolicyEvaluationRecord {
            rules_evaluated: vec![PolicyRuleId::new()],
            matching_rules: vec![PolicyRuleId::new()],
            final_decision: PolicyAction::Escalate,
            decision_latency_ms: 12,
        };

        let routing = RoutingDecision {
            candidates: vec![],
            selected: ProviderId::new("stripe_issuing"),
            selected_rail: cream_models::payment::RailPreference::Card,
            reason: "lowest_fee".to_string(),
        };

        let provider_resp = ProviderResponseRecord {
            provider: ProviderId::new("stripe_issuing"),
            transaction_id: "ch_test_123".to_string(),
            status: "succeeded".to_string(),
            amount_settled: Decimal::from_str("149.99").unwrap(),
            currency: Currency::SGD,
            latency_ms: 187,
        };

        let human = HumanReviewRecord {
            reviewer_id: "admin@example.com".to_string(),
            decision: PolicyAction::Approve,
            reason: Some("Looks good".to_string()),
            decided_at: Utc::now(),
        };

        let row = AuditRow {
            id: uuid::Uuid::now_v7(),
            timestamp: Utc::now(),
            agent_id: uuid::Uuid::now_v7(),
            agent_profile_id: uuid::Uuid::now_v7(),
            request: serde_json::json!({"amount": "149.99"}),
            justification: serde_json::json!({"summary": "booking flight"}),
            policy_evaluation: serde_json::to_value(&policy_eval).unwrap(),
            routing_decision: Some(serde_json::to_value(&routing).unwrap()),
            provider_response: Some(serde_json::to_value(&provider_resp).unwrap()),
            final_status: "settled".to_string(),
            human_review: Some(serde_json::to_value(&human).unwrap()),
            on_chain_tx_hash: Some("0xabc123".to_string()),
        };

        let entry = row.into_entry().unwrap();
        assert!(entry.routing_decision.is_some());
        assert!(entry.provider_response.is_some());
        assert!(entry.human_review.is_some());
        assert_eq!(entry.on_chain_tx_hash.as_deref(), Some("0xabc123"));
        assert_eq!(entry.final_status, PaymentStatus::Settled);
    }

    #[test]
    fn audit_row_invalid_status_returns_error() {
        let policy_eval = PolicyEvaluationRecord {
            rules_evaluated: vec![],
            matching_rules: vec![],
            final_decision: PolicyAction::Approve,
            decision_latency_ms: 1,
        };

        let row = AuditRow {
            id: uuid::Uuid::now_v7(),
            timestamp: Utc::now(),
            agent_id: uuid::Uuid::now_v7(),
            agent_profile_id: uuid::Uuid::now_v7(),
            request: serde_json::json!({}),
            justification: serde_json::json!({}),
            policy_evaluation: serde_json::to_value(&policy_eval).unwrap(),
            routing_decision: None,
            provider_response: None,
            final_status: "invalid_status_xyz".to_string(),
            human_review: None,
            on_chain_tx_hash: None,
        };

        assert!(row.into_entry().is_err());
    }

    #[tokio::test]
    async fn mock_reader_get_by_id_returns_none() {
        let mut mock = MockAuditReader::new();
        mock.expect_get_by_id().returning(|_| Ok(None));

        let result = mock.get_by_id(AuditEntryId::new()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn mock_reader_query_returns_empty() {
        let mut mock = MockAuditReader::new();
        mock.expect_query().returning(|_| Ok(vec![]));

        let result = mock.query(AuditQuery::default()).await.unwrap();
        assert!(result.is_empty());
    }
}
