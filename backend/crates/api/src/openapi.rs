//! OpenAPI 3.1 specification generation (Phase 17-F).
//!
//! Builds the spec using utoipa's builder API. All endpoints are registered
//! with request/response schemas, grouped by tag. The spec is served at
//! `/v1/openapi.json` and Swagger UI at `/docs`.

use utoipa::openapi::path::{HttpMethod, OperationBuilder, ParameterBuilder, ParameterIn};
use utoipa::openapi::request_body::RequestBodyBuilder;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityRequirement, SecurityScheme};
use utoipa::openapi::{
    ArrayBuilder, ComponentsBuilder, ContentBuilder, Info, InfoBuilder, ObjectBuilder, PathItem,
    PathsBuilder, ResponseBuilder, Type,
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

fn number_schema() -> utoipa::openapi::RefOr<utoipa::openapi::Schema> {
    ObjectBuilder::new()
        .schema_type(Type::Number)
        .build()
        .into()
}

fn integer_schema() -> utoipa::openapi::RefOr<utoipa::openapi::Schema> {
    ObjectBuilder::new()
        .schema_type(Type::Integer)
        .build()
        .into()
}

fn boolean_schema() -> utoipa::openapi::RefOr<utoipa::openapi::Schema> {
    ObjectBuilder::new()
        .schema_type(Type::Boolean)
        .build()
        .into()
}

fn string_array_schema() -> utoipa::openapi::RefOr<utoipa::openapi::Schema> {
    ArrayBuilder::new()
        .items(ObjectBuilder::new().schema_type(Type::String).build())
        .build()
        .into()
}

fn schema_ref(name: &str) -> utoipa::openapi::RefOr<utoipa::openapi::Schema> {
    utoipa::openapi::Ref::new(format!("#/components/schemas/{name}")).into()
}

// ---------------------------------------------------------------------------
// Typed content helpers
// ---------------------------------------------------------------------------

fn json_content() -> utoipa::openapi::content::Content {
    ContentBuilder::new().schema(Some(object_schema())).build()
}

fn typed_json_content(schema: utoipa::openapi::RefOr<utoipa::openapi::Schema>) -> utoipa::openapi::content::Content {
    ContentBuilder::new().schema(Some(schema)).build()
}

fn json_body(desc: &str) -> utoipa::openapi::request_body::RequestBody {
    RequestBodyBuilder::new()
        .description(Some(desc))
        .content("application/json", json_content())
        .required(Some(utoipa::openapi::Required::True))
        .build()
}

fn typed_json_body(
    desc: &str,
    schema: utoipa::openapi::RefOr<utoipa::openapi::Schema>,
) -> utoipa::openapi::request_body::RequestBody {
    RequestBodyBuilder::new()
        .description(Some(desc))
        .content("application/json", typed_json_content(schema))
        .required(Some(utoipa::openapi::Required::True))
        .build()
}

fn ok_resp(desc: &str) -> utoipa::openapi::Response {
    ResponseBuilder::new()
        .description(desc)
        .content("application/json", json_content())
        .build()
}

fn typed_ok_resp(
    desc: &str,
    schema: utoipa::openapi::RefOr<utoipa::openapi::Schema>,
) -> utoipa::openapi::Response {
    ResponseBuilder::new()
        .description(desc)
        .content("application/json", typed_json_content(schema))
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

fn agent_secured_op(summary: &str, tag: &str) -> OperationBuilder {
    OperationBuilder::new()
        .summary(Some(summary))
        .tag(tag)
        .security(SecurityRequirement::new("agent_api_key", Vec::<String>::new()))
        .response("200", ok_resp("Success"))
        .response("401", ResponseBuilder::new().description("Unauthorized").build())
}

fn operator_secured_op(summary: &str, tag: &str) -> OperationBuilder {
    OperationBuilder::new()
        .summary(Some(summary))
        .tag(tag)
        .security(SecurityRequirement::new("operator_jwt", Vec::<String>::new()))
        .response("200", ok_resp("Success"))
        .response("401", ResponseBuilder::new().description("Unauthorized").build())
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

// ---------------------------------------------------------------------------
// Component schemas — reusable type definitions
// ---------------------------------------------------------------------------

fn build_component_schemas(components: ComponentsBuilder) -> ComponentsBuilder {
    components
        // -- Recipient --
        .schema(
            "Recipient",
            ObjectBuilder::new()
                .schema_type(Type::Object)
                .property("type", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .enum_values(Some(["merchant", "individual", "wallet", "bank_account"])))
                .required("type")
                .property("identifier", string_schema())
                .required("identifier")
                .property("name", string_schema())
                .property("country", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .description(Some("ISO 3166-1 alpha-2 country code"))
                    .min_length(Some(2))
                    .max_length(Some(2)))
                .build(),
        )
        // -- Justification --
        .schema(
            "Justification",
            ObjectBuilder::new()
                .schema_type(Type::Object)
                .property("summary", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .description(Some("Human-readable reason for the payment (min 10 chars)")))
                .required("summary")
                .property("task_id", string_schema())
                .property("category", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .enum_values(Some([
                        "software_subscription", "cloud_infrastructure", "api_credits",
                        "travel", "office_supplies", "professional_services",
                        "marketing", "other",
                    ])))
                .required("category")
                .property("expected_value", string_schema())
                .build(),
        )
        // -- PaymentMetadata --
        .schema(
            "PaymentMetadata",
            ObjectBuilder::new()
                .schema_type(Type::Object)
                .property("agent_session_id", string_schema())
                .property("workflow_id", string_schema())
                .property("operator_ref", string_schema())
                .build(),
        )
        // -- PaymentRequest --
        .schema(
            "PaymentRequest",
            ObjectBuilder::new()
                .schema_type(Type::Object)
                .property("agent_id", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .description(Some("Agent ID (agt_...)")))
                .required("agent_id")
                .property("amount", ObjectBuilder::new()
                    .schema_type(Type::Number)
                    .description(Some("Payment amount (decimal, e.g. 149.99)")))
                .required("amount")
                .property("currency", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .description(Some("ISO 4217 currency code")))
                .required("currency")
                .property("recipient", schema_ref("Recipient"))
                .required("recipient")
                .property("preferred_rail", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .enum_values(Some(["auto", "card", "ach", "swift", "local", "stablecoin"]))
                    .default(Some(serde_json::json!("auto"))))
                .property("justification", schema_ref("Justification"))
                .required("justification")
                .property("metadata", schema_ref("PaymentMetadata"))
                .property("idempotency_key", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .description(Some("Client-provided idempotency key (idem_...)")))
                .required("idempotency_key")
                .build(),
        )
        // -- PaymentResponse --
        .schema(
            "PaymentResponse",
            ObjectBuilder::new()
                .schema_type(Type::Object)
                .property("id", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .description(Some("Payment ID (pay_...)")))
                .property("status", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .enum_values(Some([
                        "pending", "validating", "pending_approval", "approved",
                        "submitted", "settled", "failed", "blocked", "rejected", "timed_out",
                    ])))
                .required("status")
                .property("amount", number_schema())
                .property("currency", string_schema())
                .property("provider_id", string_schema())
                .property("created_at", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .format(Some(utoipa::openapi::SchemaFormat::KnownFormat(utoipa::openapi::KnownFormat::DateTime))))
                .build(),
        )
        // -- Agent --
        .schema(
            "Agent",
            ObjectBuilder::new()
                .schema_type(Type::Object)
                .property("id", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .description(Some("Agent ID (agt_...)")))
                .property("profile_id", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .description(Some("Profile ID (prof_...)")))
                .property("name", string_schema())
                .property("status", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .enum_values(Some(["active", "suspended", "revoked"])))
                .required("status")
                .property("created_at", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .format(Some(utoipa::openapi::SchemaFormat::KnownFormat(utoipa::openapi::KnownFormat::DateTime))))
                .property("updated_at", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .format(Some(utoipa::openapi::SchemaFormat::KnownFormat(utoipa::openapi::KnownFormat::DateTime))))
                .build(),
        )
        // -- AgentProfile --
        .schema(
            "AgentProfile",
            ObjectBuilder::new()
                .schema_type(Type::Object)
                .property("id", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .description(Some("Profile ID (prof_...)")))
                .property("name", string_schema())
                .property("version", integer_schema())
                .property("max_per_transaction", number_schema())
                .property("max_daily_spend", number_schema())
                .property("max_weekly_spend", number_schema())
                .property("max_monthly_spend", number_schema())
                .property("allowed_categories", string_array_schema())
                .property("allowed_rails", string_array_schema())
                .property("geographic_restrictions", string_array_schema())
                .property("escalation_threshold", number_schema())
                .property("timezone", string_schema())
                .build(),
        )
        // -- CardControls --
        .schema(
            "CardControls",
            ObjectBuilder::new()
                .schema_type(Type::Object)
                .property("max_per_transaction", number_schema())
                .property("max_per_cycle", number_schema())
                .property("allowed_mcc_codes", string_array_schema())
                .property("currency", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .description(Some("ISO 4217 currency code")))
                .build(),
        )
        // -- VirtualCard --
        .schema(
            "VirtualCard",
            ObjectBuilder::new()
                .schema_type(Type::Object)
                .property("id", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .description(Some("Card ID (card_...)")))
                .property("agent_id", string_schema())
                .property("provider", string_schema())
                .property("provider_card_id", string_schema())
                .property("card_type", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .enum_values(Some(["single_use", "multi_use"])))
                .property("controls", schema_ref("CardControls"))
                .property("status", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .enum_values(Some(["active", "frozen", "cancelled", "expired"])))
                .property("created_at", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .format(Some(utoipa::openapi::SchemaFormat::KnownFormat(utoipa::openapi::KnownFormat::DateTime))))
                .property("expires_at", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .format(Some(utoipa::openapi::SchemaFormat::KnownFormat(utoipa::openapi::KnownFormat::DateTime))))
                .build(),
        )
        // -- AlertRule --
        .schema(
            "AlertRule",
            ObjectBuilder::new()
                .schema_type(Type::Object)
                .property("id", string_schema())
                .property("name", string_schema())
                .property("description", string_schema())
                .property("metric", string_schema())
                .property("condition", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .enum_values(Some(["gt", "lt", "gte", "lte", "eq"])))
                .property("threshold", number_schema())
                .property("window_seconds", integer_schema())
                .property("cooldown_seconds", integer_schema())
                .property("channels", object_schema())
                .property("enabled", boolean_schema())
                .property("last_fired_at", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .format(Some(utoipa::openapi::SchemaFormat::KnownFormat(utoipa::openapi::KnownFormat::DateTime)))
                    .description(Some("When this rule last fired (nullable)")))
                .property("created_at", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .format(Some(utoipa::openapi::SchemaFormat::KnownFormat(utoipa::openapi::KnownFormat::DateTime))))
                .property("updated_at", ObjectBuilder::new()
                    .schema_type(Type::String)
                    .format(Some(utoipa::openapi::SchemaFormat::KnownFormat(utoipa::openapi::KnownFormat::DateTime))))
                .build(),
        )
        // -- Error response --
        .schema(
            "ErrorResponse",
            ObjectBuilder::new()
                .schema_type(Type::Object)
                .property("error_code", string_schema())
                .property("message", string_schema())
                .required("error_code")
                .required("message")
                .build(),
        )
}

/// Build the full OpenAPI 3.1 specification.
pub fn build_openapi_spec() -> utoipa::openapi::OpenApi {
    let info: Info = InfoBuilder::new()
        .title("Cream Payment Control Plane API")
        .version(env!("CARGO_PKG_VERSION"))
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
        );

    let components = build_component_schemas(components).build();

    let paths = PathsBuilder::new()
        // -- Payments --
        .path(
            "/v1/payments",
            PathItem::new(
                HttpMethod::Post,
                agent_secured_op("Initiate a payment with structured justification", "Payments")
                    .request_body(Some(typed_json_body(
                        "Payment request with justification",
                        schema_ref("PaymentRequest"),
                    )))
                    .response("200", typed_ok_resp("Payment created", schema_ref("PaymentResponse")))
                    .response("403", ResponseBuilder::new().description("Policy blocked").build()),
            ),
        )
        .path(
            "/v1/payments/{id}",
            PathItem::new(
                HttpMethod::Get,
                agent_secured_op("Get payment status and audit record", "Payments")
                    .parameter(path_param("id", "Payment ID (pay_...)"))
                    .response("200", typed_ok_resp("Payment details", schema_ref("PaymentResponse"))),
            ),
        )
        .path(
            "/v1/payments/{id}/approve",
            PathItem::new(
                HttpMethod::Post,
                operator_secured_op("Approve an escalated payment (operator-only)", "Payments")
                    .parameter(path_param("id", "Payment ID")),
            ),
        )
        .path(
            "/v1/payments/{id}/reject",
            PathItem::new(
                HttpMethod::Post,
                operator_secured_op("Reject an escalated payment (operator-only)", "Payments")
                    .parameter(path_param("id", "Payment ID")),
            ),
        )
        // -- Cards --
        .path(
            "/v1/cards",
            PathItem::new(
                HttpMethod::Post,
                agent_secured_op("Issue a scoped virtual card to an agent", "Cards")
                    .request_body(Some(typed_json_body("Card configuration", schema_ref("CardControls"))))
                    .response("200", typed_ok_resp("Card issued", schema_ref("VirtualCard"))),
            ),
        )
        .path(
            "/v1/cards/{id}",
            dual_path(
                HttpMethod::Patch,
                operator_secured_op("Update card controls", "Cards")
                    .parameter(path_param("id", "Card ID (card_...)"))
                    .request_body(Some(typed_json_body("Updated card controls", schema_ref("CardControls")))),
                HttpMethod::Delete,
                operator_secured_op("Cancel/revoke a card immediately", "Cards")
                    .parameter(path_param("id", "Card ID")),
            ),
        )
        // -- Agents --
        .path(
            "/v1/agents",
            dual_path(
                HttpMethod::Get,
                operator_secured_op("List all agents (operator-only)", "Agents")
                    .response("200", typed_ok_resp(
                        "List of agents",
                        ArrayBuilder::new().items(
                            utoipa::openapi::Ref::new("#/components/schemas/Agent"),
                        ).build().into(),
                    )),
                HttpMethod::Post,
                operator_secured_op("Create a new agent (operator-only)", "Agents")
                    .request_body(Some(json_body("Agent name and profile ID")))
                    .response("200", typed_ok_resp("Agent created", schema_ref("Agent"))),
            ),
        )
        .path(
            "/v1/agents/{id}",
            PathItem::new(
                HttpMethod::Patch,
                operator_secured_op("Update agent name, status, or profile (operator-only)", "Agents")
                    .parameter(path_param("id", "Agent ID (agt_...)"))
                    .request_body(Some(json_body("Fields to update")))
                    .response("200", typed_ok_resp("Agent updated", schema_ref("Agent"))),
            ),
        )
        .path(
            "/v1/agents/{id}/rotate-key",
            PathItem::new(
                HttpMethod::Post,
                operator_secured_op("Rotate agent API key (operator-only)", "Agents")
                    .parameter(path_param("id", "Agent ID")),
            ),
        )
        // -- Policies --
        .path(
            "/v1/agents/{id}/policy",
            dual_path(
                HttpMethod::Get,
                agent_secured_op("Get agent's policy profile and rules", "Policies")
                    .parameter(path_param("id", "Agent ID"))
                    .response("200", typed_ok_resp("Policy profile", schema_ref("AgentProfile"))),
                HttpMethod::Put,
                operator_secured_op("Update agent's policy profile (operator-only)", "Policies")
                    .parameter(path_param("id", "Agent ID"))
                    .request_body(Some(json_body("Policy profile fields to update"))),
            ),
        )
        // -- Audit --
        .path(
            "/v1/audit",
            PathItem::new(
                HttpMethod::Get,
                agent_secured_op("Query audit log with filters", "Audit").description(Some(
                    "Supports content negotiation: Accept: application/json (default), \
                     text/csv, application/x-ndjson. CSV/NDJSON capped at 10K rows.",
                )),
            ),
        )
        .path(
            "/v1/audit/export",
            PathItem::new(
                HttpMethod::Post,
                operator_secured_op("Create async audit export job (operator-only)", "Audit")
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
                operator_secured_op("Poll export job status (operator-only)", "Audit")
                    .parameter(path_param("id", "Export job ID (UUID)")),
            ),
        )
        // -- Webhooks --
        .path(
            "/v1/webhooks",
            dual_path(
                HttpMethod::Get,
                operator_secured_op("List webhook endpoints (operator-only)", "Webhooks"),
                HttpMethod::Post,
                operator_secured_op("Register a webhook endpoint (operator-only)", "Webhooks")
                    .request_body(Some(json_body("Webhook URL and event filters"))),
            ),
        )
        .path(
            "/v1/webhooks/{id}",
            PathItem::new(
                HttpMethod::Delete,
                operator_secured_op("Delete a webhook endpoint (operator-only)", "Webhooks")
                    .parameter(path_param("id", "Webhook endpoint ID (whk_...)")),
            ),
        )
        .path(
            "/v1/webhooks/{id}/deliveries",
            PathItem::new(
                HttpMethod::Get,
                operator_secured_op("List delivery attempts for a webhook (operator-only)", "Webhooks")
                    .parameter(path_param("id", "Webhook endpoint ID")),
            ),
        )
        .path(
            "/v1/webhooks/{id}/test",
            PathItem::new(
                HttpMethod::Post,
                operator_secured_op("Send a test event to a webhook endpoint (operator-only)", "Webhooks")
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
                operator_secured_op("List stored provider API keys (masked) (operator-only)", "Settings"),
                HttpMethod::Put,
                operator_secured_op("Store or update a provider API key (operator-only)", "Settings")
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
                operator_secured_op("Apply a template to an agent's policy profile (operator-only)", "Templates")
                    .parameter(path_param("template_id", "Template ID"))
                    .parameter(path_param("agent_id", "Agent ID (agt_...)")),
            ),
        )
        // -- Alerts --
        .path(
            "/v1/alerts",
            dual_path(
                HttpMethod::Get,
                operator_secured_op("List all alert rules (operator-only)", "Alerts"),
                HttpMethod::Post,
                operator_secured_op("Create a new alert rule (operator-only)", "Alerts")
                    .request_body(Some(typed_json_body("Alert rule configuration", schema_ref("AlertRule")))),
            ),
        )
        .path(
            "/v1/alerts/{id}",
            dual_path(
                HttpMethod::Patch,
                operator_secured_op("Update an alert rule (operator-only)", "Alerts")
                    .parameter(path_param("id", "Alert rule ID (UUID)"))
                    .request_body(Some(typed_json_body("Updated alert rule fields", schema_ref("AlertRule")))),
                HttpMethod::Delete,
                operator_secured_op("Disable an alert rule (operator-only)", "Alerts")
                    .parameter(path_param("id", "Alert rule ID")),
            ),
        )
        .path(
            "/v1/alerts/history",
            PathItem::new(
                HttpMethod::Get,
                operator_secured_op("View recently fired alerts (operator-only)", "Alerts"),
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
        assert_eq!(spec.info.version, env!("CARGO_PKG_VERSION"));
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
        // 22 unique path patterns covering 34+ operations.
        assert!(
            path_count >= 20,
            "expected at least 20 paths, got {path_count}"
        );
    }

    #[test]
    fn spec_has_component_schemas() {
        let spec = build_openapi_spec();
        let components = spec.components.as_ref().expect("components must exist");
        let schemas = &components.schemas;
        assert!(schemas.contains_key("PaymentRequest"), "missing PaymentRequest schema");
        assert!(schemas.contains_key("PaymentResponse"), "missing PaymentResponse schema");
        assert!(schemas.contains_key("Justification"), "missing Justification schema");
        assert!(schemas.contains_key("Recipient"), "missing Recipient schema");
        assert!(schemas.contains_key("Agent"), "missing Agent schema");
        assert!(schemas.contains_key("AgentProfile"), "missing AgentProfile schema");
        assert!(schemas.contains_key("VirtualCard"), "missing VirtualCard schema");
        assert!(schemas.contains_key("CardControls"), "missing CardControls schema");
        assert!(schemas.contains_key("AlertRule"), "missing AlertRule schema");
        assert!(schemas.contains_key("ErrorResponse"), "missing ErrorResponse schema");
    }

    #[test]
    fn payment_endpoint_uses_typed_schema() {
        let spec = build_openapi_spec();
        let json = serde_json::to_value(&spec).expect("spec must serialize");
        let post_payments = &json["paths"]["/v1/payments"]["post"];
        let req_body = &post_payments["requestBody"]["content"]["application/json"]["schema"];
        // Should be a $ref, not a generic object
        assert!(
            req_body.get("$ref").is_some(),
            "POST /v1/payments request body should reference a named schema, got: {req_body}"
        );
    }

    #[test]
    fn endpoints_have_security_requirements() {
        let spec = build_openapi_spec();
        let json = serde_json::to_value(&spec).expect("spec must serialize");
        // POST /v1/payments should require agent_api_key
        let post_payments = &json["paths"]["/v1/payments"]["post"];
        assert!(
            post_payments.get("security").is_some(),
            "POST /v1/payments must declare a security requirement"
        );
    }
}
