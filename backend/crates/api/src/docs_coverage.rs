//! Documentation coverage validation tests (Phase 17-H).
//!
//! These tests verify that the documentation stays in sync with the code:
//! - Every environment variable in config.rs is documented in self-hosting.md
//! - Every route in lib.rs is documented in api-reference.md
//! - Every policy rule type in the engine is documented in policy-authoring.md

#[cfg(test)]
mod tests {
    /// Extract environment variable names from config.rs source.
    fn extract_env_vars_from_config() -> Vec<String> {
        let config_src = include_str!("config.rs");
        let mut vars = Vec::new();

        // Match env::var("NAME") and parse_env("NAME", ...)
        for line in config_src.lines() {
            let line = line.trim();
            for pattern in &["env::var(\"", "parse_env(\""] {
                if let Some(start) = line.find(pattern) {
                    let after = &line[start + pattern.len()..];
                    if let Some(end) = after.find('"') {
                        let var_name = &after[..end];
                        if var_name.chars().all(|c| c.is_ascii_uppercase() || c == '_')
                            && !vars.contains(&var_name.to_string())
                        {
                            vars.push(var_name.to_string());
                        }
                    }
                }
            }
        }

        vars
    }

    /// Extract route paths from lib.rs source.
    fn extract_routes_from_lib() -> Vec<String> {
        let lib_src = include_str!("lib.rs");
        let mut routes = Vec::new();

        for line in lib_src.lines() {
            let line = line.trim();
            // Match .route("/v1/...", ...) and .route("/health", ...)
            if let Some(start) = line.find(".route(\"") {
                let after = &line[start + 8..]; // skip .route("
                if let Some(end) = after.find('"') {
                    let path = &after[..end];
                    if path.starts_with('/') && !routes.contains(&path.to_string()) {
                        routes.push(path.to_string());
                    }
                }
            }
        }

        routes
    }

    #[test]
    fn env_vars_documented_in_self_hosting_guide() {
        let env_vars = extract_env_vars_from_config();
        let self_hosting =
            include_str!("../../../../docs/guides/self-hosting.md");

        // These are internal/meta vars that don't need documentation.
        let skip = [
            "RUST_LOG",           // Standard Rust env var, not Cream-specific
            "ALLOW_PERMISSIVE_CORS", // Documented inline with CORS_ALLOWED_ORIGINS
        ];

        let mut missing = Vec::new();
        for var in &env_vars {
            if skip.contains(&var.as_str()) {
                continue;
            }
            if !self_hosting.contains(var) {
                missing.push(var.clone());
            }
        }

        assert!(
            missing.is_empty(),
            "environment variables in config.rs not documented in self-hosting.md: {:?}",
            missing
        );
    }

    #[test]
    fn routes_documented_in_api_reference() {
        let routes = extract_routes_from_lib();
        let api_ref = include_str!("../../../../docs/api-reference.md");

        let mut missing = Vec::new();
        for route in &routes {
            // Normalize path params: /v1/agents/{id} → /v1/agents/{id}
            // The doc uses the same {param} format so direct match works.
            if !api_ref.contains(route) {
                missing.push(route.clone());
            }
        }

        assert!(
            missing.is_empty(),
            "routes in lib.rs not documented in api-reference.md: {:?}",
            missing
        );
    }

    #[test]
    fn policy_rule_types_documented() {
        // All 12 rule types registered in PolicyEngine::new().
        let rule_types = [
            "amount_cap",
            "velocity_limit",
            "spend_rate",
            "category_check",
            "merchant_check",
            "geographic",
            "rail_restriction",
            "justification_quality",
            "time_window",
            "first_time_merchant",
            "duplicate_detection",
            "escalation_threshold",
        ];

        let policy_doc =
            include_str!("../../../../docs/guides/policy-authoring.md");

        let mut missing = Vec::new();
        for rule_type in &rule_types {
            if !policy_doc.contains(rule_type) {
                missing.push(*rule_type);
            }
        }

        assert!(
            missing.is_empty(),
            "policy rule types not documented in policy-authoring.md: {:?}",
            missing
        );
    }
}
