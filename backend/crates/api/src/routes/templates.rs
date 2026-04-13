use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use cream_models::prelude::*;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::extractors::auth::AuthenticatedOperator;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct PolicyTemplateResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub rules: serde_json::Value,
    pub is_builtin: bool,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `GET /v1/policy-templates` — list all policy templates.
pub async fn list_templates(
    State(state): State<AppState>,
    _op: AuthenticatedOperator,
) -> Result<Json<Vec<PolicyTemplateResponse>>, ApiError> {
    let rows = sqlx::query_as::<_, (uuid::Uuid, String, String, String, serde_json::Value, bool, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, name, description, category, rules, is_builtin, created_at FROM policy_templates ORDER BY category, name",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("list templates: {e}")))?;

    let templates: Vec<PolicyTemplateResponse> = rows
        .into_iter()
        .map(|r| PolicyTemplateResponse {
            id: r.0.to_string(),
            name: r.1,
            description: r.2,
            category: r.3,
            rules: r.4,
            is_builtin: r.5,
            created_at: r.6.to_rfc3339(),
        })
        .collect();

    Ok(Json(templates))
}

/// `GET /v1/policy-templates/{id}` — get a single template with its rules.
pub async fn get_template(
    State(state): State<AppState>,
    _op: AuthenticatedOperator,
    Path(id): Path<String>,
) -> Result<Json<PolicyTemplateResponse>, ApiError> {
    let template_id: uuid::Uuid = id
        .parse()
        .map_err(|e| ApiError::ValidationError(format!("invalid template ID: {e}")))?;

    let row = sqlx::query_as::<_, (uuid::Uuid, String, String, String, serde_json::Value, bool, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, name, description, category, rules, is_builtin, created_at FROM policy_templates WHERE id = $1",
    )
    .bind(template_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("get template: {e}")))?
    .ok_or_else(|| ApiError::NotFound(format!("template {id}")))?;

    Ok(Json(PolicyTemplateResponse {
        id: row.0.to_string(),
        name: row.1,
        description: row.2,
        category: row.3,
        rules: row.4,
        is_builtin: row.5,
        created_at: row.6.to_rfc3339(),
    }))
}

/// `POST /v1/policy-templates/{template_id}/apply/{agent_id}` — apply a template's
/// rules to an agent's profile.
///
/// Inserts the template's rules into the agent's `policy_rules` table. Does NOT
/// delete existing custom rules — the template rules are layered on top.
pub async fn apply_template(
    State(state): State<AppState>,
    _op: AuthenticatedOperator,
    Path((template_id_str, agent_id_str)): Path<(String, String)>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let template_id: uuid::Uuid = template_id_str
        .parse()
        .map_err(|e| ApiError::ValidationError(format!("invalid template ID: {e}")))?;

    let agent_id = agent_id_str
        .parse::<AgentId>()
        .map_err(|e| ApiError::ValidationError(format!("invalid agent ID: {e}")))?;

    // Fetch the template.
    let template_row = sqlx::query_as::<_, (serde_json::Value, String)>(
        "SELECT rules, name FROM policy_templates WHERE id = $1",
    )
    .bind(template_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("fetch template: {e}")))?
    .ok_or_else(|| ApiError::NotFound(format!("template {template_id_str}")))?;

    let rules = template_row.0;
    let template_name = template_row.1;

    // Look up the agent's profile_id.
    let profile_id = crate::extractors::auth::lookup_profile_id_for_agent(
        &state.db,
        &agent_id,
    )
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("agent {agent_id_str}")))?;

    // Parse rules array and insert each one.
    let rule_array = rules
        .as_array()
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("template rules is not an array")))?;

    let mut inserted = 0u32;
    for rule_json in rule_array {
        let rule_id = PolicyRuleId::new();
        let priority = rule_json
            .get("priority")
            .and_then(|v| v.as_i64())
            .unwrap_or(100) as i32;
        let rule_type = rule_json
            .get("rule_type")
            .and_then(|v| v.as_str())
            .map(String::from);
        let condition = rule_json.get("condition").cloned().unwrap_or(serde_json::json!({}));
        let action = rule_json
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("BLOCK");
        let escalation = rule_json.get("escalation").cloned();

        sqlx::query(
            r#"
            INSERT INTO policy_rules (id, profile_id, rule_type, priority, condition, action, escalation, enabled)
            VALUES ($1, $2, $3, $4, $5, $6, $7, true)
            "#,
        )
        .bind(rule_id.as_uuid())
        .bind(profile_id.as_uuid())
        .bind(&rule_type)
        .bind(priority)
        .bind(&condition)
        .bind(action)
        .bind(&escalation)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("insert rule: {e}")))?;

        inserted += 1;
    }

    // Log operator event.
    sqlx::query(
        "INSERT INTO operator_events (event_type, details) VALUES ('template_applied', $1)",
    )
    .bind(serde_json::json!({
        "template_id": template_id_str,
        "template_name": template_name,
        "agent_id": agent_id_str,
        "profile_id": profile_id.to_string(),
        "rules_inserted": inserted,
    }))
    .execute(&state.db)
    .await
    .ok(); // Best-effort audit.

    tracing::info!(
        template = %template_name,
        agent = %agent_id_str,
        rules_inserted = inserted,
        "policy template applied"
    );

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "template_id": template_id_str,
            "template_name": template_name,
            "agent_id": agent_id_str,
            "rules_inserted": inserted,
        })),
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_response_serializes() {
        let resp = PolicyTemplateResponse {
            id: "123".to_string(),
            name: "Starter".to_string(),
            description: "Basic limits".to_string(),
            category: "starter".to_string(),
            rules: serde_json::json!([{"rule_type": "amount_cap"}]),
            is_builtin: true,
            created_at: "2026-04-13T00:00:00Z".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["name"], "Starter");
        assert!(json["rules"].is_array());
        assert_eq!(json["is_builtin"], true);
    }

    #[test]
    fn seed_rule_json_is_valid() {
        // Verify the seed rule JSON from the migration is parseable.
        let starter_rules: serde_json::Value = serde_json::from_str(r#"[
            {"rule_type": "amount_cap", "priority": 10, "condition": {"field_check": {"field": "amount", "op": "greater_than", "value": 1000}}, "action": "BLOCK"},
            {"rule_type": "spend_rate", "priority": 20, "condition": {"field_check": {"field": "daily_spend", "op": "greater_than", "value": 5000}}, "action": "BLOCK"},
            {"rule_type": "velocity_limit", "priority": 30, "condition": {"field_check": {"field": "hourly_count", "op": "greater_than", "value": 20}}, "action": "BLOCK"}
        ]"#).unwrap();

        assert!(starter_rules.is_array());
        assert_eq!(starter_rules.as_array().unwrap().len(), 3);

        // Each rule has required fields.
        for rule in starter_rules.as_array().unwrap() {
            assert!(rule.get("rule_type").is_some());
            assert!(rule.get("priority").is_some());
            assert!(rule.get("condition").is_some());
            assert!(rule.get("action").is_some());
        }
    }
}
