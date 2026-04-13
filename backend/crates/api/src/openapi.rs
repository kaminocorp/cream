//! OpenAPI 3.1 specification generation (Phase 17-F).
//!
//! Builds the spec using utoipa's builder API. All endpoints are registered
//! with request/response schemas, grouped by tag. The spec is served at
//! `/v1/openapi.json` and Swagger UI at `/docs`.

use utoipa::openapi::path::{HttpMethod, OperationBuilder, ParameterBuilder, ParameterIn};
use utoipa::openapi::request_body::RequestBodyBuilder;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::openapi::{
    ComponentsBuilder, ContentBuilder, Info, InfoBuilder, ObjectBuilder, PathItem, PathsBuilder,
    ResponseBuilder, Type,
};

fn string_schema() -> utoipa::openapi::RefOr<utoipa::openapi::Schema> {
    ObjectBuilder::new()
        .schema_type(Type::String)
        .build()
        .into()
}

fn object_schema() -> utoipa::openapi::RefOr<utoipa::openapi::Schema> {
    ObjectBuilder::new()
        .schema_type(Type::Object)
        .build()
        .into()
}

fn json_content() -> utoipa::openapi::content::Content {
    ContentBuilder::new().schema(Some(object_schema())).build()
}

fn json_body(desc: &str) -> utoipa::openapi::request_body::RequestBody {
    RequestBodyBuilder::new()
        .description(Some(desc))
        .content("application/json", json_content())
        .required(Some(utoipa::openapi::Required::True))
        .build()
}

fn ok_resp(desc: &str) -> utoipa::openapi::Response {
    ResponseBuilder::new()
        .description(desc)
        .content("application/json", json_content())
        .build()
}

fn path_param(name: &str, desc: &str) -> utoipa::openapi::path::Parameter {
    ParameterBuilder::new()
        .name(name)
        .parameter_in(ParameterIn::Path)
        .required(utoipa::openapi::Required::True)
        .description(Some(desc))
        .schema(Some(string_schema()))
        .build()
}

fn op(summary: &str, tag: &str) -> OperationBuilder {
    OperationBuilder::new()
        .summary(Some(summary))
        .tag(tag)
        .response("200", ok_resp("Success"))
}

/// Create a PathItem with two HTTP methods (e.g., GET + POST on same path).
fn dual_path(
    m1: HttpMethod,
    o1: OperationBuilder,
    m2: HttpMethod,
    o2: OperationBuilder,
) -> PathItem {
    let mut item = PathItem::new(m1, o1);
    item.merge_operations(PathItem::new(m2, o2));
    item
}

/// Build the full OpenAPI 3.1 specification.
pub fn build_openapi_spec() -> utoipa::openapi::OpenApi {
    let info: Info = InfoBuilder::new()
        .title("Cream Payment Control Plane API")
        .version("0.21.5")
        .description(Some(
            "Universal payment control plane for AI agents. \
             Abstracts payment providers, enforces operator-defined policies, \
             requires structured agent justification, and produces immutable audit logs.",
        ))
        .build();

    let components = ComponentsBuilder::new()
        .security_scheme(
            "agent_api_key",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("API Key")
                    .description(Some(
                        "Agent API key (cream_...). Issued via POST /v1/agents.",
                    ))
                    .build(),
            ),
        )
        .security_scheme(
            "operator_jwt",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .description(Some(
                        "Operator JWT access token. Obtained via POST /v1/auth/login.",
                    ))
                    .build(),
            ),
        )
        .build();

    let paths = PathsBuilder::new()
        // -- Payments --
        .path(
            "/v1/payments",
            PathItem::new(
                HttpMethod::Post,
                op("Initiate a payment with structured justification", "Payments")
                    .request_body(Some(json_body("Payment request with justification")))
                    .response("403", ResponseBuilder::new().description("Policy blocked").build()),
            ),
        )
        .path(
            "/v1/payments/{id}",
            PathItem::new(
                HttpMethod::Get,
                op("Get payment status and audit record", "Payments")
                    .parameter(path_param("id", "Payment ID (pay_...)")),
            ),
        )
        .path(
            "/v1/payments/{id}/approve",
            PathItem::new(
                HttpMethod::Post,
                op("Approve an escalated payment (operator-only)", "Payments")
                    .parameter(path_param("id", "Payment ID")),
            ),
        )
        .path(
            "/v1/payments/{id}/reject",
            PathItem::new(
                HttpMethod::Post,
                op("Reject an escalated payment (operator-only)", "Payments")
                    .parameter(path_param("id", "Payment ID")),
            ),
        )
        // -- Cards --
        .path(
            "/v1/cards",
            PathItem::new(
                HttpMethod::Post,
                op("Issue a scoped virtual card to an agent", "Cards")
                    .request_body(Some(json_body("Card configuration"))),
            ),
        )
        .path(
            "/v1/cards/{id}",
            dual_path(
                HttpMethod::Patch,
                op("Update card controls", "Cards")
                    .parameter(path_param("id", "Card ID (card_...)"))
                    .request_body(Some(json_body("Updated card controls"))),
                HttpMethod::Delete,
                op("Cancel/revoke a card immediately", "Cards")
                    .parameter(path_param("id", "Card ID")),
            ),
        )
        // -- Agents --
        .path(
            "/v1/agents",
            dual_path(
                HttpMethod::Get,
                op("List all agents (operator-only)", "Agents"),
                HttpMethod::Post,
                op("Create a new agent (operator-only)", "Agents")
                    .request_body(Some(json_body("Agent name and profile ID"))),
            ),
        )
        .path(
            "/v1/agents/{id}",
            PathItem::new(
                HttpMethod::Patch,
                op("Update agent name, status, or profile (operator-only)", "Agents")
                    .parameter(path_param("id", "Agent ID (agt_...)"))
                    .request_body(Some(json_body("Fields to update"))),
            ),
        )
        .path(
            "/v1/agents/{id}/rotate-key",
            PathItem::new(
                HttpMethod::Post,
                op("Rotate agent API key (operator-only)", "Agents")
                    .parameter(path_param("id", "Agent ID")),
            ),
        )
        // -- Policies --
        .path(
            "/v1/agents/{id}/policy",
            dual_path(
                HttpMethod::Get,
                op("Get agent's policy profile and rules", "Policies")
                    .parameter(path_param("id", "Agent ID")),
                HttpMethod::Put,
                op("Update agent's policy profile (operator-only)", "Policies")
                    .parameter(path_param("id", "Agent ID"))
                    .request_body(Some(json_body("Policy profile fields to update"))),
            ),
        )
        // -- Audit --
        .path(
            "/v1/audit",
            PathItem::new(
                HttpMethod::Get,
                op("Query audit log with filters", "Audit").description(Some(
                    "Supports content negotiation: Accept: application/json (default), \
                     text/csv, application/x-ndjson. CSV/NDJSON capped at 10K rows.",
                )),
            ),
        )
        .path(
            "/v1/audit/export",
            PathItem::new(
                HttpMethod::Post,
                op("Create async audit export job (operator-only)", "Audit")
                    .request_body(Some(json_body("Export filters, format, and S3 destination")))
                    .response(
                        "202",
                        ResponseBuilder::new()
                            .description("Export job created")
                            .build(),
                    ),
            ),
        )
        .path(
            "/v1/audit/exports/{id}",
            PathItem::new(
                HttpMethod::Get,
                op("Poll export job status (operator-only)", "Audit")
                    .parameter(path_param("id", "Export job ID (UUID)")),
            ),
        )
        // -- Webhooks --
        .path(
            "/v1/webhooks",
            dual_path(
                HttpMethod::Get,
                op("List webhook endpoints (operator-only)", "Webhooks"),
                HttpMethod::Post,
                op("Register a webhook endpoint (operator-only)", "Webhooks")
                    .request_body(Some(json_body("Webhook URL and event filters"))),
            ),
        )
        .path(
            "/v1/webhooks/{id}",
            PathItem::new(
                HttpMethod::Delete,
                op("Delete a webhook endpoint (operator-only)", "Webhooks")
                    .parameter(path_param("id", "Webhook endpoint ID (whk_...)")),
            ),
        )
        .path(
            "/v1/webhooks/{id}/deliveries",
            PathItem::new(
                HttpMethod::Get,
                op("List delivery attempts for a webhook (operator-only)", "Webhooks")
                    .parameter(path_param("id", "Webhook endpoint ID")),
            ),
        )
        .path(
            "/v1/webhooks/{id}/test",
            PathItem::new(
                HttpMethod::Post,
                op("Send a test event to a webhook endpoint (operator-only)", "Webhooks")
                    .parameter(path_param("id", "Webhook endpoint ID")),
            ),
        )
        // -- Auth --
        .path(
            "/v1/auth/status",
            PathItem::new(
                HttpMethod::Get,
                op("Check if any operator is registered", "Auth"),
            ),
        )
        .path(
            "/v1/auth/register",
            PathItem::new(
                HttpMethod::Post,
                op("Register first operator (blocked when operators exist)", "Auth")
                    .request_body(Some(json_body("Name, email, password"))),
            ),
        )
        .path(
            "/v1/auth/login",
            PathItem::new(
                HttpMethod::Post,
                op("Login with email + password, receive JWT tokens", "Auth")
                    .request_body(Some(json_body("Email and password"))),
            ),
        )
        .path(
            "/v1/auth/refresh",
            PathItem::new(
                HttpMethod::Post,
                op("Rotate refresh token, issue new access token", "Auth")
                    .request_body(Some(json_body("Refresh token"))),
            ),
        )
        .path(
            "/v1/auth/logout",
            PathItem::new(
                HttpMethod::Post,
                op("Revoke refresh token", "Auth")
                    .request_body(Some(json_body("Refresh token"))),
            ),
        )
        // -- Settings --
        .path(
            "/v1/settings/provider-keys",
            dual_path(
                HttpMethod::Get,
                op("List stored provider API keys (masked) (operator-only)", "Settings"),
                HttpMethod::Put,
                op("Store or update a provider API key (operator-only)", "Settings")
                    .request_body(Some(json_body("Provider name and API key"))),
            ),
        )
        // -- Policy Templates --
        .path(
            "/v1/policy-templates",
            PathItem::new(
                HttpMethod::Get,
                op("List available policy templates", "Templates"),
            ),
        )
        .path(
            "/v1/policy-templates/{id}",
            PathItem::new(
                HttpMethod::Get,
                op("Get a specific policy template", "Templates")
                    .parameter(path_param("id", "Template ID")),
            ),
        )
        .path(
            "/v1/policy-templates/{template_id}/apply/{agent_id}",
            PathItem::new(
                HttpMethod::Post,
                op("Apply a template to an agent's policy profile (operator-only)", "Templates")
                    .parameter(path_param("template_id", "Template ID"))
                    .parameter(path_param("agent_id", "Agent ID (agt_...)")),
            ),
        )
        // -- Alerts --
        .path(
            "/v1/alerts",
            dual_path(
                HttpMethod::Get,
                op("List all alert rules (operator-only)", "Alerts"),
                HttpMethod::Post,
                op("Create a new alert rule (operator-only)", "Alerts")
                    .request_body(Some(json_body("Alert rule configuration"))),
            ),
        )
        .path(
            "/v1/alerts/{id}",
            dual_path(
                HttpMethod::Patch,
                op("Update an alert rule (operator-only)", "Alerts")
                    .parameter(path_param("id", "Alert rule ID (UUID)"))
                    .request_body(Some(json_body("Updated alert rule fields"))),
                HttpMethod::Delete,
                op("Disable an alert rule (operator-only)", "Alerts")
                    .parameter(path_param("id", "Alert rule ID")),
            ),
        )
        .path(
            "/v1/alerts/history",
            PathItem::new(
                HttpMethod::Get,
                op("View recently fired alerts (operator-only)", "Alerts"),
            ),
        )
        // -- Providers --
        .path(
            "/v1/providers/health",
            PathItem::new(
                HttpMethod::Get,
                op("Get real-time health status of all connected providers", "Providers"),
            ),
        )
        // -- Integrations --
        .path(
            "/v1/integrations/slack/callback",
            PathItem::new(
                HttpMethod::Post,
                op("Slack interaction callback (verified by signing secret)", "Integrations"),
            ),
        )
        .build();

    let mut spec = utoipa::openapi::OpenApi::new(info, paths);
    spec.components = Some(components);
    spec
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_is_valid_json() {
        let spec = build_openapi_spec();
        let json = serde_json::to_string_pretty(&spec).expect("spec must serialize to JSON");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("serialized spec must be valid JSON");
        assert!(parsed.is_object());
    }

    #[test]
    fn spec_has_correct_version() {
        let spec = build_openapi_spec();
        assert_eq!(spec.info.version, "0.21.5");
    }

    #[test]
    fn spec_covers_all_route_groups() {
        let spec = build_openapi_spec();
        let paths = &spec.paths;

        assert!(paths.paths.contains_key("/v1/payments"));
        assert!(paths.paths.contains_key("/v1/agents"));
        assert!(paths.paths.contains_key("/v1/audit"));
        assert!(paths.paths.contains_key("/v1/webhooks"));
        assert!(paths.paths.contains_key("/v1/auth/login"));
        assert!(paths.paths.contains_key("/v1/providers/health"));
        assert!(paths.paths.contains_key("/v1/policy-templates"));
        assert!(paths.paths.contains_key("/v1/settings/provider-keys"));
        assert!(paths.paths.contains_key("/v1/audit/export"));
    }

    #[test]
    fn spec_has_security_schemes() {
        let spec = build_openapi_spec();
        let components = spec.components.as_ref().expect("components must exist");
        assert!(components.security_schemes.contains_key("agent_api_key"));
        assert!(components.security_schemes.contains_key("operator_jwt"));
    }

    #[test]
    fn spec_path_count_covers_all_endpoints() {
        let spec = build_openapi_spec();
        let path_count = spec.paths.paths.len();
        // 22 unique path patterns covering 34 operations.
        assert!(
            path_count >= 20,
            "expected at least 20 paths, got {path_count}"
        );
    }
}
