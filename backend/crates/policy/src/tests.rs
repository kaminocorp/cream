use std::collections::HashSet;

use chrono::{Duration, Utc};
use rust_decimal::Decimal;

use cream_models::justification::{Justification, PaymentCategory};
use cream_models::prelude::*;
use cream_models::recipient::{Recipient, RecipientType};

use crate::context::{EvaluationContext, PaymentSummary};
use crate::engine::PolicyEngine;
use crate::evaluator::{RuleEvaluator, RuleResult};
use crate::rules::{
    amount_cap::AmountCapEvaluator, category_check::CategoryCheckEvaluator,
    duplicate_detection::DuplicateDetectionEvaluator,
    escalation_threshold::EscalationThresholdEvaluator,
    first_time_merchant::FirstTimeMerchantEvaluator, geographic::GeographicEvaluator,
    justification_quality::JustificationQualityEvaluator,
    rail_restriction::RailRestrictionEvaluator, spend_rate::SpendRateEvaluator,
    time_window::TimeWindowEvaluator, velocity_limit::VelocityLimitEvaluator,
};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn test_request(amount: Decimal) -> PaymentRequest {
    PaymentRequest {
        agent_id: AgentId::new(),
        amount,
        currency: Currency::SGD,
        recipient: Recipient {
            recipient_type: RecipientType::Merchant,
            identifier: "stripe_merch_123".to_string(),
            name: Some("Test Merchant".to_string()),
            country: Some(CountryCode::new("SG")),
        },
        preferred_rail: RailPreference::Auto,
        justification: Justification {
            summary: "Purchasing API credits for batch processing job number 4421 in production environment".to_string(),
            task_id: Some("task_8372".to_string()),
            category: PaymentCategory::ApiCredits,
            expected_value: None,
        },
        metadata: None,
        idempotency_key: IdempotencyKey::new("test-key-001"),
    }
}

fn test_agent() -> Agent {
    Agent {
        id: AgentId::new(),
        profile_id: AgentProfileId::new(),
        name: "test-agent".to_string(),
        status: AgentStatus::Active,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn test_profile() -> AgentProfile {
    AgentProfile {
        id: AgentProfileId::new(),
        name: "test-profile".to_string(),
        version: 1,
        max_per_transaction: Decimal::new(500, 0), // $500
        max_daily_spend: Decimal::new(2000, 0),    // $2,000
        max_weekly_spend: Decimal::new(10000, 0),  // $10,000
        max_monthly_spend: Decimal::new(30000, 0), // $30,000
        allowed_categories: vec![],                // empty = all allowed
        allowed_rails: vec![],                     // empty = all allowed
        geographic_restrictions: vec![],           // empty = all allowed
        escalation_threshold: Some(Decimal::new(1000, 0)),
        timezone: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn test_context(amount: Decimal) -> EvaluationContext {
    EvaluationContext {
        request: test_request(amount),
        agent: test_agent(),
        profile: test_profile(),
        recent_payments: vec![],
        known_merchants: HashSet::from(["stripe_merch_123".to_string()]),
        current_time: Utc::now(),
    }
}

fn make_rule(field: &str, value: serde_json::Value, action: PolicyAction) -> PolicyRule {
    // Infer the rule_type from the field name, matching what the engine expects
    let rule_type = match field {
        "amount" => "amount_cap",
        "velocity" => "velocity_limit",
        "spend_rate" | "daily_spend" | "weekly_spend" | "monthly_spend" => "spend_rate",
        "justification.category" => "category_check",
        "recipient.identifier" => "merchant_check",
        "recipient.country" => "geographic",
        "preferred_rail" | "rail" => "rail_restriction",
        "justification.summary" | "justification_quality" => "justification_quality",
        "time_window" => "time_window",
        "first_time_merchant" => "first_time_merchant",
        "duplicate" => "duplicate_detection",
        "proportionality" | "expected_value" => "proportionality",
        other => other,
    };
    PolicyRule {
        id: PolicyRuleId::new(),
        profile_id: AgentProfileId::new(),
        rule_type: Some(rule_type.to_string()),
        priority: 10,
        condition: PolicyCondition::FieldCheck(FieldCheck {
            field: field.to_string(),
            op: ComparisonOp::GreaterThan,
            value,
        }),
        action,
        escalation: None,
        enabled: true,
    }
}

// ---------------------------------------------------------------------------
// Amount Cap tests
// ---------------------------------------------------------------------------

#[test]
fn amount_cap_passes_below_limit() {
    let ctx = test_context(Decimal::new(100, 0));
    let rule = make_rule("amount", serde_json::json!(500), PolicyAction::Block);
    let result = AmountCapEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn amount_cap_triggers_above_limit() {
    let ctx = test_context(Decimal::new(600, 0)); // $600 > $500 limit
    let rule = make_rule("amount", serde_json::json!(500), PolicyAction::Block);
    let result = AmountCapEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block));
}

// ---------------------------------------------------------------------------
// Category Check tests
// ---------------------------------------------------------------------------

#[test]
fn category_check_passes_when_no_restrictions() {
    let ctx = test_context(Decimal::new(100, 0));
    let rule = make_rule(
        "justification.category",
        serde_json::json!("api_credits"),
        PolicyAction::Block,
    );
    let result = CategoryCheckEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn category_check_triggers_when_not_allowed() {
    let mut ctx = test_context(Decimal::new(100, 0));
    ctx.profile.allowed_categories = vec![PaymentCategory::CloudInfrastructure]; // only cloud
                                                                                 // Request is ApiCredits → not allowed
    let rule = make_rule(
        "justification.category",
        serde_json::json!("api_credits"),
        PolicyAction::Block,
    );
    let result = CategoryCheckEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block));
}

#[test]
fn category_check_passes_when_allowed() {
    let mut ctx = test_context(Decimal::new(100, 0));
    ctx.profile.allowed_categories = vec![
        PaymentCategory::ApiCredits,
        PaymentCategory::CloudInfrastructure,
    ];
    let rule = make_rule(
        "justification.category",
        serde_json::json!("api_credits"),
        PolicyAction::Block,
    );
    let result = CategoryCheckEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

// ---------------------------------------------------------------------------
// Geographic Restriction tests
// ---------------------------------------------------------------------------

#[test]
fn geographic_passes_when_no_restrictions() {
    let ctx = test_context(Decimal::new(100, 0));
    let rule = make_rule(
        "recipient.country",
        serde_json::json!("SG"),
        PolicyAction::Block,
    );
    let result = GeographicEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn geographic_triggers_when_country_not_allowed() {
    let mut ctx = test_context(Decimal::new(100, 0));
    ctx.profile.geographic_restrictions = vec![CountryCode::new("US"), CountryCode::new("GB")];
    // Recipient is SG, not in allowed list
    let rule = make_rule(
        "recipient.country",
        serde_json::json!("SG"),
        PolicyAction::Block,
    );
    let result = GeographicEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block));
}

#[test]
fn geographic_passes_when_country_allowed() {
    let mut ctx = test_context(Decimal::new(100, 0));
    ctx.profile.geographic_restrictions = vec![CountryCode::new("SG"), CountryCode::new("US")];
    let rule = make_rule(
        "recipient.country",
        serde_json::json!("SG"),
        PolicyAction::Block,
    );
    let result = GeographicEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

// ---------------------------------------------------------------------------
// Rail Restriction tests
// ---------------------------------------------------------------------------

#[test]
fn rail_restriction_passes_auto() {
    let mut ctx = test_context(Decimal::new(100, 0));
    ctx.profile.allowed_rails = vec![RailPreference::Card];
    // Request has Auto — always passes
    let rule = make_rule(
        "preferred_rail",
        serde_json::json!("auto"),
        PolicyAction::Block,
    );
    let result = RailRestrictionEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn rail_restriction_triggers_disallowed_rail() {
    let mut ctx = test_context(Decimal::new(100, 0));
    ctx.request.preferred_rail = RailPreference::Stablecoin;
    ctx.profile.allowed_rails = vec![RailPreference::Card, RailPreference::Swift];
    let rule = make_rule(
        "preferred_rail",
        serde_json::json!("stablecoin"),
        PolicyAction::Block,
    );
    let result = RailRestrictionEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block));
}

// ---------------------------------------------------------------------------
// Justification Quality tests
// ---------------------------------------------------------------------------

#[test]
fn justification_quality_passes_good_summary() {
    let ctx = test_context(Decimal::new(100, 0));
    let rule = make_rule(
        "justification_quality",
        serde_json::json!(true),
        PolicyAction::Block,
    );
    let result = JustificationQualityEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn justification_quality_triggers_empty_summary() {
    let mut ctx = test_context(Decimal::new(100, 0));
    ctx.request.justification.summary = "".to_string();
    let rule = make_rule(
        "justification_quality",
        serde_json::json!(true),
        PolicyAction::Block,
    );
    let result = JustificationQualityEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block));
}

#[test]
fn justification_quality_triggers_too_short() {
    let mut ctx = test_context(Decimal::new(100, 0));
    ctx.request.justification.summary = "Buy stuff now".to_string(); // 3 words
    let rule = make_rule(
        "justification_quality",
        serde_json::json!(true),
        PolicyAction::Block,
    );
    let result = JustificationQualityEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block));
}

// ---------------------------------------------------------------------------
// First-time Merchant tests
// ---------------------------------------------------------------------------

#[test]
fn first_time_merchant_passes_known() {
    let ctx = test_context(Decimal::new(100, 0)); // known_merchants has stripe_merch_123
    let rule = make_rule(
        "first_time_merchant",
        serde_json::json!(true),
        PolicyAction::Escalate,
    );
    let result = FirstTimeMerchantEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn first_time_merchant_triggers_unknown() {
    let mut ctx = test_context(Decimal::new(100, 0));
    ctx.request.recipient.identifier = "unknown_merchant_456".to_string();
    let rule = make_rule(
        "first_time_merchant",
        serde_json::json!(true),
        PolicyAction::Escalate,
    );
    let result = FirstTimeMerchantEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Escalate));
}

// ---------------------------------------------------------------------------
// Duplicate Detection tests
// ---------------------------------------------------------------------------

#[test]
fn duplicate_detection_passes_no_recent() {
    let ctx = test_context(Decimal::new(100, 0));
    let rule = make_rule(
        "duplicate",
        serde_json::json!({"window_minutes": 5}),
        PolicyAction::Block,
    );
    let result = DuplicateDetectionEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn duplicate_detection_triggers_same_amount_recipient() {
    let mut ctx = test_context(Decimal::new(100, 0));
    ctx.recent_payments.push(PaymentSummary {
        amount: Decimal::new(100, 0),
        currency: Currency::SGD,
        recipient_identifier: "stripe_merch_123".to_string(),
        category: PaymentCategory::ApiCredits,
        status: PaymentStatus::Submitted,
        rail: RailPreference::Card,
        created_at: Utc::now() - Duration::minutes(2), // 2 minutes ago
    });
    let rule = make_rule(
        "duplicate",
        serde_json::json!({"window_minutes": 5}),
        PolicyAction::Block,
    );
    let result = DuplicateDetectionEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block));
}

#[test]
fn duplicate_detection_passes_outside_window() {
    let mut ctx = test_context(Decimal::new(100, 0));
    ctx.recent_payments.push(PaymentSummary {
        amount: Decimal::new(100, 0),
        currency: Currency::SGD,
        recipient_identifier: "stripe_merch_123".to_string(),
        category: PaymentCategory::ApiCredits,
        status: PaymentStatus::Submitted,
        rail: RailPreference::Card,
        created_at: Utc::now() - Duration::minutes(10), // 10 minutes ago, outside 5-min window
    });
    let rule = make_rule(
        "duplicate",
        serde_json::json!({"window_minutes": 5}),
        PolicyAction::Block,
    );
    let result = DuplicateDetectionEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

// ---------------------------------------------------------------------------
// Spend Rate tests
// ---------------------------------------------------------------------------

#[test]
fn spend_rate_passes_below_daily_limit() {
    let ctx = test_context(Decimal::new(100, 0)); // $100, daily limit $2000
    let rule = make_rule("spend_rate", serde_json::json!(true), PolicyAction::Block);
    let result = SpendRateEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn spend_rate_triggers_above_daily_limit() {
    let mut ctx = test_context(Decimal::new(500, 0)); // requesting $500
                                                      // Add $1600 in recent payments → $1600 + $500 = $2100 > $2000 daily
    ctx.recent_payments.push(PaymentSummary {
        amount: Decimal::new(1600, 0),
        currency: Currency::SGD,
        recipient_identifier: "merchant_a".to_string(),
        category: PaymentCategory::ApiCredits,
        status: PaymentStatus::Submitted,
        rail: RailPreference::Card,
        created_at: Utc::now() - Duration::hours(2),
    });
    let rule = make_rule("spend_rate", serde_json::json!(true), PolicyAction::Block);
    let result = SpendRateEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block));
}

#[test]
fn spend_rate_counts_settled_payments() {
    let mut ctx = test_context(Decimal::new(500, 0)); // requesting $500
                                                      // Add $1600 in SETTLED (terminal) payments — these must still count
    ctx.recent_payments.push(PaymentSummary {
        amount: Decimal::new(1600, 0),
        currency: Currency::SGD,
        recipient_identifier: "merchant_a".to_string(),
        category: PaymentCategory::ApiCredits,
        status: PaymentStatus::Settled,
        rail: RailPreference::Card,
        created_at: Utc::now() - Duration::hours(2),
    });
    let rule = make_rule("spend_rate", serde_json::json!(true), PolicyAction::Block);
    let result = SpendRateEvaluator.evaluate(&rule, &ctx);
    // $1600 settled + $500 current = $2100 > $2000 daily limit
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block));
}

#[test]
fn spend_rate_excludes_failed_payments() {
    let mut ctx = test_context(Decimal::new(500, 0)); // requesting $500
                                                      // Add $1600 in FAILED payments — these should NOT count
    ctx.recent_payments.push(PaymentSummary {
        amount: Decimal::new(1600, 0),
        currency: Currency::SGD,
        recipient_identifier: "merchant_a".to_string(),
        category: PaymentCategory::ApiCredits,
        status: PaymentStatus::Failed,
        rail: RailPreference::Card,
        created_at: Utc::now() - Duration::hours(2),
    });
    let rule = make_rule("spend_rate", serde_json::json!(true), PolicyAction::Block);
    let result = SpendRateEvaluator.evaluate(&rule, &ctx);
    // Only $500 current (failed doesn't count) < $2000 daily limit
    assert_eq!(result, RuleResult::Pass);
}

// ---------------------------------------------------------------------------
// Velocity Limit tests
// ---------------------------------------------------------------------------

#[test]
fn velocity_limit_passes_below_count() {
    let ctx = test_context(Decimal::new(100, 0));
    let rule = PolicyRule {
        id: PolicyRuleId::new(),
        profile_id: AgentProfileId::new(),
        rule_type: Some("velocity_limit".to_string()),
        priority: 10,
        condition: PolicyCondition::FieldCheck(FieldCheck {
            field: "velocity".to_string(),
            op: ComparisonOp::GreaterThan,
            value: serde_json::json!({"max_count": 5, "window_minutes": 60}),
        }),
        action: PolicyAction::Block,
        escalation: None,
        enabled: true,
    };
    let result = VelocityLimitEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass); // 1 (current) <= 5
}

#[test]
fn velocity_limit_triggers_above_count() {
    let mut ctx = test_context(Decimal::new(10, 0));
    // Add 5 recent payments in the last hour → 5 + 1 (current) = 6 > 5
    for i in 0..5 {
        ctx.recent_payments.push(PaymentSummary {
            amount: Decimal::new(10, 0),
            currency: Currency::SGD,
            recipient_identifier: format!("merchant_{i}"),
            category: PaymentCategory::ApiCredits,
            status: PaymentStatus::Submitted,
            rail: RailPreference::Card,
            created_at: Utc::now() - Duration::minutes(i as i64 * 10),
        });
    }
    let rule = PolicyRule {
        id: PolicyRuleId::new(),
        profile_id: AgentProfileId::new(),
        rule_type: Some("velocity_limit".to_string()),
        priority: 10,
        condition: PolicyCondition::FieldCheck(FieldCheck {
            field: "velocity".to_string(),
            op: ComparisonOp::GreaterThan,
            value: serde_json::json!({"max_count": 5, "window_minutes": 60}),
        }),
        action: PolicyAction::Block,
        escalation: None,
        enabled: true,
    };
    let result = VelocityLimitEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block));
}

#[test]
fn velocity_limit_counts_settled_payments() {
    let mut ctx = test_context(Decimal::new(10, 0));
    // Add 5 SETTLED payments — these must still count toward velocity
    for i in 0..5 {
        ctx.recent_payments.push(PaymentSummary {
            amount: Decimal::new(10, 0),
            currency: Currency::SGD,
            recipient_identifier: format!("merchant_{i}"),
            category: PaymentCategory::ApiCredits,
            status: PaymentStatus::Settled,
            rail: RailPreference::Card,
            created_at: Utc::now() - Duration::minutes(i as i64 * 10),
        });
    }
    let rule = PolicyRule {
        id: PolicyRuleId::new(),
        profile_id: AgentProfileId::new(),
        rule_type: Some("velocity_limit".to_string()),
        priority: 10,
        condition: PolicyCondition::FieldCheck(FieldCheck {
            field: "velocity".to_string(),
            op: ComparisonOp::GreaterThan,
            value: serde_json::json!({"max_count": 5, "window_minutes": 60}),
        }),
        action: PolicyAction::Block,
        escalation: None,
        enabled: true,
    };
    let result = VelocityLimitEvaluator.evaluate(&rule, &ctx);
    // 5 settled + 1 current = 6 > 5
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block));
}

// ---------------------------------------------------------------------------
// Engine-level tests
// ---------------------------------------------------------------------------

#[test]
fn engine_approves_when_no_rules() {
    let engine = PolicyEngine::new();
    let ctx = test_context(Decimal::new(100, 0));
    let decision = engine.evaluate(&[], &ctx).unwrap();
    assert_eq!(decision.action, PolicyAction::Approve);
    assert!(decision.rules_evaluated.is_empty());
}

#[test]
fn engine_approves_when_rules_pass() {
    let engine = PolicyEngine::new();
    let ctx = test_context(Decimal::new(100, 0)); // under $500 cap
    let rules = vec![make_rule(
        "amount",
        serde_json::json!(500),
        PolicyAction::Block,
    )];
    let decision = engine.evaluate(&rules, &ctx).unwrap();
    assert_eq!(decision.action, PolicyAction::Approve);
    assert_eq!(decision.rules_evaluated.len(), 1);
    assert!(decision.matching_rules.is_empty());
}

#[test]
fn engine_blocks_on_first_block() {
    let engine = PolicyEngine::new();
    let ctx = test_context(Decimal::new(600, 0)); // over $500 cap
    let rules = vec![
        make_rule("amount", serde_json::json!(500), PolicyAction::Block),
        make_rule("amount", serde_json::json!(500), PolicyAction::Escalate), // should not be reached
    ];
    let decision = engine.evaluate(&rules, &ctx).unwrap();
    assert_eq!(decision.action, PolicyAction::Block);
    assert_eq!(decision.matching_rules.len(), 1);
    // Second rule should not have been evaluated (block halts)
    assert_eq!(decision.rules_evaluated.len(), 1);
}

#[test]
fn engine_escalates_when_no_block() {
    let engine = PolicyEngine::new();
    let mut ctx = test_context(Decimal::new(100, 0));
    ctx.request.recipient.identifier = "new_merchant_xyz".to_string(); // not in known set

    let rules = vec![make_rule(
        "first_time_merchant",
        serde_json::json!(true),
        PolicyAction::Escalate,
    )];
    let decision = engine.evaluate(&rules, &ctx).unwrap();
    assert_eq!(decision.action, PolicyAction::Escalate);
    assert_eq!(decision.matching_rules.len(), 1);
}

#[test]
fn engine_block_overrides_escalate() {
    let engine = PolicyEngine::new();
    let mut ctx = test_context(Decimal::new(600, 0)); // over $500 cap
    ctx.request.recipient.identifier = "new_merchant_xyz".to_string(); // unknown merchant

    // Escalate rule runs first (lower priority number), then block rule
    let mut escalate_rule = make_rule(
        "first_time_merchant",
        serde_json::json!(true),
        PolicyAction::Escalate,
    );
    escalate_rule.priority = 1;

    let mut block_rule = make_rule("amount", serde_json::json!(500), PolicyAction::Block);
    block_rule.priority = 2;

    let rules = vec![escalate_rule, block_rule];
    let decision = engine.evaluate(&rules, &ctx).unwrap();
    assert_eq!(decision.action, PolicyAction::Block);
    assert_eq!(decision.matching_rules.len(), 2); // both triggered
}

#[test]
fn engine_respects_priority_order() {
    let engine = PolicyEngine::new();
    let ctx = test_context(Decimal::new(600, 0)); // over cap

    let mut low_priority = make_rule("amount", serde_json::json!(500), PolicyAction::Block);
    low_priority.priority = 100;

    let mut high_priority = make_rule("amount", serde_json::json!(500), PolicyAction::Block);
    high_priority.priority = 1;

    // Even though low_priority is first in the vec, high_priority should evaluate first
    let rules = vec![low_priority, high_priority.clone()];
    let decision = engine.evaluate(&rules, &ctx).unwrap();
    assert_eq!(decision.action, PolicyAction::Block);
    // Should have only evaluated the first (high priority) rule before halting
    assert_eq!(decision.rules_evaluated.len(), 1);
    assert_eq!(decision.matching_rules[0], high_priority.id);
}

#[test]
fn engine_skips_disabled_rules() {
    let engine = PolicyEngine::new();
    let ctx = test_context(Decimal::new(600, 0));

    let mut disabled_rule = make_rule("amount", serde_json::json!(500), PolicyAction::Block);
    disabled_rule.enabled = false;

    let rules = vec![disabled_rule];
    let decision = engine.evaluate(&rules, &ctx).unwrap();
    assert_eq!(decision.action, PolicyAction::Approve);
    assert!(decision.rules_evaluated.is_empty()); // disabled = not evaluated
}

#[test]
fn engine_records_latency() {
    let engine = PolicyEngine::new();
    let ctx = test_context(Decimal::new(100, 0));
    let decision = engine.evaluate(&[], &ctx).unwrap();
    // Latency should be very small (sub-millisecond) but >= 0
    assert!(decision.latency_ms < 100);
}

// ---------------------------------------------------------------------------
// Condition evaluator tests
// ---------------------------------------------------------------------------

#[test]
fn condition_all_requires_all_true() {
    let ctx = test_context(Decimal::new(600, 0));
    // Currency::SGD serializes as "S_G_D" due to SCREAMING_SNAKE_CASE serde rename
    let sgd_value = serde_json::to_value(Currency::SGD).unwrap();
    let condition = PolicyCondition::All(vec![
        PolicyCondition::FieldCheck(FieldCheck {
            field: "amount".to_string(),
            op: ComparisonOp::GreaterThan,
            value: serde_json::json!(500),
        }),
        PolicyCondition::FieldCheck(FieldCheck {
            field: "currency".to_string(),
            op: ComparisonOp::Equals,
            value: sgd_value,
        }),
    ]);
    assert!(crate::evaluator::evaluate_condition(&condition, &ctx));
}

#[test]
fn condition_all_fails_if_one_false() {
    let ctx = test_context(Decimal::new(600, 0));
    let condition = PolicyCondition::All(vec![
        PolicyCondition::FieldCheck(FieldCheck {
            field: "amount".to_string(),
            op: ComparisonOp::GreaterThan,
            value: serde_json::json!(500),
        }),
        PolicyCondition::FieldCheck(FieldCheck {
            field: "currency".to_string(),
            op: ComparisonOp::Equals,
            value: serde_json::to_value(Currency::USD).unwrap(), // Wrong currency
        }),
    ]);
    assert!(!crate::evaluator::evaluate_condition(&condition, &ctx));
}

#[test]
fn condition_any_passes_if_one_true() {
    let ctx = test_context(Decimal::new(100, 0));
    let condition = PolicyCondition::Any(vec![
        PolicyCondition::FieldCheck(FieldCheck {
            field: "amount".to_string(),
            op: ComparisonOp::GreaterThan,
            value: serde_json::json!(500), // false
        }),
        PolicyCondition::FieldCheck(FieldCheck {
            field: "currency".to_string(),
            op: ComparisonOp::Equals,
            value: serde_json::to_value(Currency::SGD).unwrap(), // true
        }),
    ]);
    assert!(crate::evaluator::evaluate_condition(&condition, &ctx));
}

#[test]
fn condition_not_inverts() {
    let ctx = test_context(Decimal::new(100, 0));
    let condition = PolicyCondition::Not(Box::new(PolicyCondition::FieldCheck(FieldCheck {
        field: "amount".to_string(),
        op: ComparisonOp::GreaterThan,
        value: serde_json::json!(500), // false → NOT → true
    })));
    assert!(crate::evaluator::evaluate_condition(&condition, &ctx));
}

#[test]
fn condition_in_operator() {
    let ctx = test_context(Decimal::new(100, 0));
    let condition = PolicyCondition::FieldCheck(FieldCheck {
        field: "justification.category".to_string(),
        op: ComparisonOp::In,
        value: serde_json::json!(["api_credits", "cloud_infrastructure"]),
    });
    assert!(crate::evaluator::evaluate_condition(&condition, &ctx));
}

#[test]
fn condition_not_in_operator() {
    let ctx = test_context(Decimal::new(100, 0));
    let condition = PolicyCondition::FieldCheck(FieldCheck {
        field: "justification.category".to_string(),
        op: ComparisonOp::NotIn,
        value: serde_json::json!(["travel", "marketing"]),
    });
    assert!(crate::evaluator::evaluate_condition(&condition, &ctx));
}

#[test]
fn condition_notin_non_array_fails_safe() {
    let ctx = test_context(Decimal::new(100, 0));
    // NotIn with a non-array value should return false (fail safe), not true
    let condition = PolicyCondition::FieldCheck(FieldCheck {
        field: "justification.category".to_string(),
        op: ComparisonOp::NotIn,
        value: serde_json::json!("not_an_array"),
    });
    assert!(!crate::evaluator::evaluate_condition(&condition, &ctx));
}

// ---------------------------------------------------------------------------
// Time Window tests
// ---------------------------------------------------------------------------

fn make_time_window_rule(start: u32, end: u32) -> PolicyRule {
    PolicyRule {
        id: PolicyRuleId::new(),
        profile_id: AgentProfileId::new(),
        rule_type: Some("time_window".to_string()),
        priority: 10,
        condition: PolicyCondition::FieldCheck(FieldCheck {
            field: "time_window".to_string(),
            op: ComparisonOp::Equals,
            value: serde_json::json!({
                "allowed_hours_start": start,
                "allowed_hours_end": end,
            }),
        }),
        action: PolicyAction::Block,
        escalation: None,
        enabled: true,
    }
}

#[test]
fn time_window_passes_inside_normal_range() {
    let mut ctx = test_context(Decimal::new(100, 0));
    // Set time to 12:00 UTC, window 9-17
    ctx.current_time = chrono::NaiveDate::from_ymd_opt(2026, 4, 1)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
        .and_utc();
    let rule = make_time_window_rule(9, 17);
    let result = TimeWindowEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn time_window_blocks_outside_normal_range() {
    let mut ctx = test_context(Decimal::new(100, 0));
    // Set time to 20:00 UTC, window 9-17
    ctx.current_time = chrono::NaiveDate::from_ymd_opt(2026, 4, 1)
        .unwrap()
        .and_hms_opt(20, 0, 0)
        .unwrap()
        .and_utc();
    let rule = make_time_window_rule(9, 17);
    let result = TimeWindowEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block));
}

#[test]
fn time_window_overnight_range_passes_late() {
    let mut ctx = test_context(Decimal::new(100, 0));
    // Set time to 23:00 UTC, window 22-6 (overnight)
    ctx.current_time = chrono::NaiveDate::from_ymd_opt(2026, 4, 1)
        .unwrap()
        .and_hms_opt(23, 0, 0)
        .unwrap()
        .and_utc();
    let rule = make_time_window_rule(22, 6);
    let result = TimeWindowEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn time_window_overnight_range_passes_early() {
    let mut ctx = test_context(Decimal::new(100, 0));
    // Set time to 03:00 UTC, window 22-6 (overnight)
    ctx.current_time = chrono::NaiveDate::from_ymd_opt(2026, 4, 1)
        .unwrap()
        .and_hms_opt(3, 0, 0)
        .unwrap()
        .and_utc();
    let rule = make_time_window_rule(22, 6);
    let result = TimeWindowEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn time_window_overnight_range_blocks_midday() {
    let mut ctx = test_context(Decimal::new(100, 0));
    // Set time to 12:00 UTC, window 22-6 (overnight) — 12:00 is outside
    ctx.current_time = chrono::NaiveDate::from_ymd_opt(2026, 4, 1)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
        .and_utc();
    let rule = make_time_window_rule(22, 6);
    let result = TimeWindowEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block));
}

#[test]
fn time_window_midnight_boundary() {
    let mut ctx = test_context(Decimal::new(100, 0));
    // Set time to 00:00 UTC, window 22-6 (overnight) — midnight is inside
    ctx.current_time = chrono::NaiveDate::from_ymd_opt(2026, 4, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();
    let rule = make_time_window_rule(22, 6);
    let result = TimeWindowEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn time_window_respects_timezone_from_profile() {
    let mut ctx = test_context(Decimal::new(100, 0));
    // Set time to 15:00 UTC. Window is 9-17.
    // With UTC, 15:00 is inside the window → Pass.
    // But with Asia/Singapore (UTC+8), 15:00 UTC = 23:00 SGT → outside → Block.
    ctx.current_time = chrono::NaiveDate::from_ymd_opt(2026, 4, 1)
        .unwrap()
        .and_hms_opt(15, 0, 0)
        .unwrap()
        .and_utc();
    ctx.profile.timezone = Some("Asia/Singapore".to_string());
    let rule = make_time_window_rule(9, 17);
    let result = TimeWindowEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block));
}

#[test]
fn time_window_utc_offset_override_in_condition() {
    let mut ctx = test_context(Decimal::new(100, 0));
    // Set time to 15:00 UTC. Window is 9-17.
    // Without offset: 15:00 is inside → Pass.
    // With utc_offset_hours=8: 15:00 UTC = 23:00 → outside → Block.
    ctx.current_time = chrono::NaiveDate::from_ymd_opt(2026, 4, 1)
        .unwrap()
        .and_hms_opt(15, 0, 0)
        .unwrap()
        .and_utc();
    let rule = PolicyRule {
        id: PolicyRuleId::new(),
        profile_id: AgentProfileId::new(),
        rule_type: Some("time_window".to_string()),
        priority: 10,
        condition: PolicyCondition::FieldCheck(FieldCheck {
            field: "time_window".to_string(),
            op: ComparisonOp::Equals,
            value: serde_json::json!({
                "allowed_hours_start": 9,
                "allowed_hours_end": 17,
                "utc_offset_hours": 8,
            }),
        }),
        action: PolicyAction::Block,
        escalation: None,
        enabled: true,
    };
    let result = TimeWindowEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block));
}

// ---------------------------------------------------------------------------
// Regex / Matches operator tests
// ---------------------------------------------------------------------------

#[test]
fn condition_matches_with_regex() {
    let ctx = test_context(Decimal::new(100, 0));
    // justification.summary contains "batch processing job number 4421"
    let condition = PolicyCondition::FieldCheck(FieldCheck {
        field: "justification.summary".to_string(),
        op: ComparisonOp::Matches,
        value: serde_json::json!(r"batch processing job number \d+"),
    });
    assert!(crate::evaluator::evaluate_condition(&condition, &ctx));
}

#[test]
fn condition_matches_invalid_regex_returns_false() {
    let ctx = test_context(Decimal::new(100, 0));
    let condition = PolicyCondition::FieldCheck(FieldCheck {
        field: "justification.summary".to_string(),
        op: ComparisonOp::Matches,
        value: serde_json::json!(r"[invalid(regex"),
    });
    // Invalid regex should return false, not panic
    assert!(!crate::evaluator::evaluate_condition(&condition, &ctx));
}

// ---------------------------------------------------------------------------
// Misconfiguration guard tests (Phase 6.3)
// ---------------------------------------------------------------------------

#[test]
fn velocity_limit_skips_on_negative_max_count() {
    let ctx = test_context(Decimal::new(100, 0));
    let rule = PolicyRule {
        id: PolicyRuleId::new(),
        profile_id: AgentProfileId::new(),
        rule_type: Some("velocity_limit".to_string()),
        priority: 10,
        condition: PolicyCondition::FieldCheck(FieldCheck {
            field: "velocity".to_string(),
            op: ComparisonOp::GreaterThan,
            value: serde_json::json!({"max_count": -5, "window_minutes": 60}),
        }),
        action: PolicyAction::Block,
        escalation: None,
        enabled: true,
    };
    // Negative max_count should cause the rule to be skipped (Pass), not bypass
    let result = VelocityLimitEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn velocity_limit_skips_on_zero_window() {
    let ctx = test_context(Decimal::new(100, 0));
    let rule = PolicyRule {
        id: PolicyRuleId::new(),
        profile_id: AgentProfileId::new(),
        rule_type: Some("velocity_limit".to_string()),
        priority: 10,
        condition: PolicyCondition::FieldCheck(FieldCheck {
            field: "velocity".to_string(),
            op: ComparisonOp::GreaterThan,
            value: serde_json::json!({"max_count": 5, "window_minutes": 0}),
        }),
        action: PolicyAction::Block,
        escalation: None,
        enabled: true,
    };
    let result = VelocityLimitEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn time_window_skips_on_out_of_range_hours() {
    let ctx = test_context(Decimal::new(100, 0));
    let rule = PolicyRule {
        id: PolicyRuleId::new(),
        profile_id: AgentProfileId::new(),
        rule_type: Some("time_window".to_string()),
        priority: 10,
        condition: PolicyCondition::FieldCheck(FieldCheck {
            field: "time_window".to_string(),
            op: ComparisonOp::Equals,
            value: serde_json::json!({
                "allowed_hours_start": 25,
                "allowed_hours_end": 50,
            }),
        }),
        action: PolicyAction::Block,
        escalation: None,
        enabled: true,
    };
    // Out-of-range hours should cause the rule to be skipped (Pass)
    let result = TimeWindowEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

// ---------------------------------------------------------------------------
// Merchant Check tests
// ---------------------------------------------------------------------------

#[test]
fn merchant_check_triggers_when_merchant_in_deny_list() {
    let ctx = test_context(Decimal::new(100, 0)); // merchant is "stripe_merch_123"
    let rule = PolicyRule {
        id: PolicyRuleId::new(),
        profile_id: AgentProfileId::new(),
        rule_type: Some("merchant_check".to_string()),
        priority: 10,
        condition: PolicyCondition::FieldCheck(FieldCheck {
            field: "recipient.identifier".to_string(),
            op: ComparisonOp::In,
            value: serde_json::json!(["stripe_merch_123", "banned_merchant"]),
        }),
        action: PolicyAction::Block,
        escalation: None,
        enabled: true,
    };
    let result = crate::rules::merchant_check::MerchantCheckEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block));
}

#[test]
fn merchant_check_passes_when_merchant_not_in_deny_list() {
    let ctx = test_context(Decimal::new(100, 0)); // merchant is "stripe_merch_123"
    let rule = PolicyRule {
        id: PolicyRuleId::new(),
        profile_id: AgentProfileId::new(),
        rule_type: Some("merchant_check".to_string()),
        priority: 10,
        condition: PolicyCondition::FieldCheck(FieldCheck {
            field: "recipient.identifier".to_string(),
            op: ComparisonOp::In,
            value: serde_json::json!(["banned_merchant_a", "banned_merchant_b"]),
        }),
        action: PolicyAction::Block,
        escalation: None,
        enabled: true,
    };
    let result = crate::rules::merchant_check::MerchantCheckEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn merchant_check_triggers_when_merchant_not_in_allow_list() {
    let mut ctx = test_context(Decimal::new(100, 0));
    ctx.request.recipient.identifier = "unknown_merchant".to_string();
    let rule = PolicyRule {
        id: PolicyRuleId::new(),
        profile_id: AgentProfileId::new(),
        rule_type: Some("merchant_check".to_string()),
        priority: 10,
        condition: PolicyCondition::FieldCheck(FieldCheck {
            field: "recipient.identifier".to_string(),
            op: ComparisonOp::NotIn,
            value: serde_json::json!(["stripe_merch_123", "approved_merchant"]),
        }),
        action: PolicyAction::Block,
        escalation: None,
        enabled: true,
    };
    let result = crate::rules::merchant_check::MerchantCheckEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block));
}

#[test]
fn merchant_check_passes_when_merchant_in_allow_list() {
    let ctx = test_context(Decimal::new(100, 0)); // merchant is "stripe_merch_123"
    let rule = PolicyRule {
        id: PolicyRuleId::new(),
        profile_id: AgentProfileId::new(),
        rule_type: Some("merchant_check".to_string()),
        priority: 10,
        condition: PolicyCondition::FieldCheck(FieldCheck {
            field: "recipient.identifier".to_string(),
            op: ComparisonOp::NotIn,
            value: serde_json::json!(["stripe_merch_123", "approved_merchant"]),
        }),
        action: PolicyAction::Block,
        escalation: None,
        enabled: true,
    };
    let result = crate::rules::merchant_check::MerchantCheckEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn merchant_check_non_array_value_fails_safe() {
    let ctx = test_context(Decimal::new(100, 0));
    // Misconfigured rule: value is a string instead of array
    let rule = PolicyRule {
        id: PolicyRuleId::new(),
        profile_id: AgentProfileId::new(),
        rule_type: Some("merchant_check".to_string()),
        priority: 10,
        condition: PolicyCondition::FieldCheck(FieldCheck {
            field: "recipient.identifier".to_string(),
            op: ComparisonOp::In,
            value: serde_json::json!("stripe_merch_123"), // string, not array
        }),
        action: PolicyAction::Block,
        escalation: None,
        enabled: true,
    };
    // Non-array value should fail safe (Pass, not trigger)
    let result = crate::rules::merchant_check::MerchantCheckEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn merchant_check_notin_non_array_value_fails_safe() {
    let ctx = test_context(Decimal::new(100, 0));
    // Misconfigured rule: NotIn with non-array value
    let rule = PolicyRule {
        id: PolicyRuleId::new(),
        profile_id: AgentProfileId::new(),
        rule_type: Some("merchant_check".to_string()),
        priority: 10,
        condition: PolicyCondition::FieldCheck(FieldCheck {
            field: "recipient.identifier".to_string(),
            op: ComparisonOp::NotIn,
            value: serde_json::json!("some_string"), // string, not array
        }),
        action: PolicyAction::Block,
        escalation: None,
        enabled: true,
    };
    // Non-array value should fail safe (Pass, not trigger)
    let result = crate::rules::merchant_check::MerchantCheckEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn merchant_check_equals_operator() {
    let ctx = test_context(Decimal::new(100, 0)); // merchant is "stripe_merch_123"
    let rule = PolicyRule {
        id: PolicyRuleId::new(),
        profile_id: AgentProfileId::new(),
        rule_type: Some("merchant_check".to_string()),
        priority: 10,
        condition: PolicyCondition::FieldCheck(FieldCheck {
            field: "recipient.identifier".to_string(),
            op: ComparisonOp::Equals,
            value: serde_json::json!("stripe_merch_123"),
        }),
        action: PolicyAction::Block,
        escalation: None,
        enabled: true,
    };
    let result = crate::rules::merchant_check::MerchantCheckEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block));
}

// ---------------------------------------------------------------------------
// Phase 6.4: Additional hardening tests
// ---------------------------------------------------------------------------

#[test]
fn duplicate_detection_skips_on_negative_window() {
    let mut ctx = test_context(Decimal::new(100, 0));
    // Add a payment that would match if window were positive
    ctx.recent_payments.push(PaymentSummary {
        amount: Decimal::new(100, 0),
        currency: Currency::SGD,
        recipient_identifier: "stripe_merch_123".to_string(),
        category: PaymentCategory::ApiCredits,
        status: PaymentStatus::Submitted,
        rail: RailPreference::Auto,
        created_at: Utc::now() - Duration::seconds(30),
    });
    let rule = PolicyRule {
        id: PolicyRuleId::new(),
        profile_id: AgentProfileId::new(),
        rule_type: Some("duplicate_detection".to_string()),
        priority: 10,
        condition: PolicyCondition::FieldCheck(FieldCheck {
            field: "duplicate".to_string(),
            op: ComparisonOp::GreaterThan,
            value: serde_json::json!({"window_minutes": -5}),
        }),
        action: PolicyAction::Block,
        escalation: None,
        enabled: true,
    };
    // Negative window should cause the rule to be skipped (Pass), not bypass
    let result = DuplicateDetectionEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn duplicate_detection_skips_on_zero_window() {
    let ctx = test_context(Decimal::new(100, 0));
    let rule = PolicyRule {
        id: PolicyRuleId::new(),
        profile_id: AgentProfileId::new(),
        rule_type: Some("duplicate_detection".to_string()),
        priority: 10,
        condition: PolicyCondition::FieldCheck(FieldCheck {
            field: "duplicate".to_string(),
            op: ComparisonOp::GreaterThan,
            value: serde_json::json!({"window_minutes": 0}),
        }),
        action: PolicyAction::Block,
        escalation: None,
        enabled: true,
    };
    let result = DuplicateDetectionEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn spend_rate_monthly_calendar_boundary() {
    // On April 5 at 12:00 UTC, a payment from March 15 should NOT count
    // toward April's monthly spend (different calendar month).
    let mut ctx = test_context(Decimal::new(100, 0));
    ctx.current_time = chrono::NaiveDate::from_ymd_opt(2026, 4, 5)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
        .and_utc();

    // Payment from March 15 — previous calendar month, outside daily/weekly windows
    ctx.recent_payments.push(PaymentSummary {
        amount: Decimal::new(29950, 0), // $29,950 — just under $30,000 monthly limit
        currency: Currency::SGD,
        recipient_identifier: "stripe_merch_123".to_string(),
        category: PaymentCategory::ApiCredits,
        status: PaymentStatus::Settled,
        rail: RailPreference::Auto,
        created_at: chrono::NaiveDate::from_ymd_opt(2026, 3, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_utc(),
    });

    let rule = make_rule(
        "spend_rate",
        serde_json::json!({"monthly": true}),
        PolicyAction::Block,
    );
    // $29,950 was in March. Current request is $100. Monthly limit is $30,000.
    // Since March payment shouldn't count for April, April total is just $100 → Pass.
    let result = SpendRateEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn geographic_evaluator_case_insensitive() {
    let mut ctx = test_context(Decimal::new(100, 0));
    ctx.profile.geographic_restrictions = vec![CountryCode::new("sg")]; // lowercase
    ctx.request.recipient.country = Some(CountryCode::new("SG")); // uppercase
    let rule = make_rule(
        "recipient.country",
        serde_json::json!(["SG"]),
        PolicyAction::Block,
    );
    // Should match despite case difference
    let result = GeographicEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

// ---------------------------------------------------------------------------
// Fix #1: Currency filtering in spend_rate and duplicate_detection
// ---------------------------------------------------------------------------

#[test]
fn spend_rate_ignores_different_currency_payments() {
    let mut ctx = test_context(Decimal::new(500, 0)); // requesting $500 SGD
                                                      // Add $1600 in USD payments — different currency, should NOT count toward SGD limit
    ctx.recent_payments.push(PaymentSummary {
        amount: Decimal::new(1600, 0),
        currency: Currency::USD, // Different from request currency (SGD)
        recipient_identifier: "merchant_a".to_string(),
        category: PaymentCategory::ApiCredits,
        status: PaymentStatus::Settled,
        rail: RailPreference::Card,
        created_at: Utc::now() - Duration::hours(2),
    });
    let rule = make_rule("spend_rate", serde_json::json!(true), PolicyAction::Block);
    let result = SpendRateEvaluator.evaluate(&rule, &ctx);
    // Only $500 SGD current (USD doesn't count) < $2000 daily limit
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn spend_rate_counts_same_currency_payments() {
    let mut ctx = test_context(Decimal::new(500, 0)); // requesting $500 SGD
                                                      // Add $1600 in SGD payments — same currency, should count
    ctx.recent_payments.push(PaymentSummary {
        amount: Decimal::new(1600, 0),
        currency: Currency::SGD, // Same as request currency
        recipient_identifier: "merchant_a".to_string(),
        category: PaymentCategory::ApiCredits,
        status: PaymentStatus::Settled,
        rail: RailPreference::Card,
        created_at: Utc::now() - Duration::hours(2),
    });
    let rule = make_rule("spend_rate", serde_json::json!(true), PolicyAction::Block);
    let result = SpendRateEvaluator.evaluate(&rule, &ctx);
    // $1600 SGD + $500 SGD = $2100 > $2000 daily limit
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block));
}

#[test]
fn duplicate_detection_ignores_different_currency() {
    let mut ctx = test_context(Decimal::new(100, 0)); // $100 SGD
                                                      // Add a recent $100 USD payment to same merchant — should NOT be flagged as duplicate
    ctx.recent_payments.push(PaymentSummary {
        amount: Decimal::new(100, 0),
        currency: Currency::USD, // Different from request currency (SGD)
        recipient_identifier: "stripe_merch_123".to_string(),
        category: PaymentCategory::ApiCredits,
        status: PaymentStatus::Settled,
        rail: RailPreference::Card,
        created_at: Utc::now() - Duration::minutes(1),
    });
    let rule = make_rule(
        "duplicate",
        serde_json::json!({"window_minutes": 5}),
        PolicyAction::Block,
    );
    let result = DuplicateDetectionEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn duplicate_detection_catches_same_currency() {
    let mut ctx = test_context(Decimal::new(100, 0)); // $100 SGD
                                                      // Add a recent $100 SGD payment to same merchant — IS a duplicate
    ctx.recent_payments.push(PaymentSummary {
        amount: Decimal::new(100, 0),
        currency: Currency::SGD, // Same as request currency
        recipient_identifier: "stripe_merch_123".to_string(),
        category: PaymentCategory::ApiCredits,
        status: PaymentStatus::Settled,
        rail: RailPreference::Card,
        created_at: Utc::now() - Duration::minutes(1),
    });
    let rule = make_rule(
        "duplicate",
        serde_json::json!({"window_minutes": 5}),
        PolicyAction::Block,
    );
    let result = DuplicateDetectionEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block));
}

// ---------------------------------------------------------------------------
// Fix #2: Case-insensitive merchant identifier matching
// ---------------------------------------------------------------------------

#[test]
fn merchant_check_deny_list_case_insensitive() {
    let ctx = test_context(Decimal::new(100, 0)); // merchant = "stripe_merch_123"
    let rule = PolicyRule {
        id: PolicyRuleId::new(),
        profile_id: AgentProfileId::new(),
        rule_type: Some("merchant_check".to_string()),
        priority: 10,
        condition: PolicyCondition::FieldCheck(FieldCheck {
            field: "recipient.identifier".to_string(),
            op: ComparisonOp::In,
            value: serde_json::json!(["STRIPE_MERCH_123"]), // uppercase in deny list
        }),
        action: PolicyAction::Block,
        escalation: None,
        enabled: true,
    };
    // Should trigger despite case difference
    let result = crate::rules::merchant_check::MerchantCheckEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block));
}

#[test]
fn merchant_check_allow_list_case_insensitive() {
    let ctx = test_context(Decimal::new(100, 0)); // merchant = "stripe_merch_123"
    let rule = PolicyRule {
        id: PolicyRuleId::new(),
        profile_id: AgentProfileId::new(),
        rule_type: Some("merchant_check".to_string()),
        priority: 10,
        condition: PolicyCondition::FieldCheck(FieldCheck {
            field: "recipient.identifier".to_string(),
            op: ComparisonOp::NotIn,
            value: serde_json::json!(["STRIPE_MERCH_123"]), // uppercase in allow list
        }),
        action: PolicyAction::Block,
        escalation: None,
        enabled: true,
    };
    // Merchant IS in the allow list (case-insensitive) → should NOT trigger
    let result = crate::rules::merchant_check::MerchantCheckEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn merchant_check_equals_case_insensitive() {
    let ctx = test_context(Decimal::new(100, 0)); // merchant = "stripe_merch_123"
    let rule = PolicyRule {
        id: PolicyRuleId::new(),
        profile_id: AgentProfileId::new(),
        rule_type: Some("merchant_check".to_string()),
        priority: 10,
        condition: PolicyCondition::FieldCheck(FieldCheck {
            field: "recipient.identifier".to_string(),
            op: ComparisonOp::Equals,
            value: serde_json::json!("STRIPE_MERCH_123"), // uppercase
        }),
        action: PolicyAction::Block,
        escalation: None,
        enabled: true,
    };
    // Should trigger despite case difference
    let result = crate::rules::merchant_check::MerchantCheckEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block));
}

// ---------------------------------------------------------------------------
// Escalation threshold tests
// ---------------------------------------------------------------------------

#[test]
fn escalation_threshold_passes_below_threshold() {
    // Profile has escalation_threshold = 1000, request amount = 100
    let ctx = test_context(Decimal::new(100, 0));
    let rule = make_rule(
        "escalation_threshold",
        serde_json::json!(null),
        PolicyAction::Escalate,
    );
    let result = EscalationThresholdEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn escalation_threshold_triggers_above_threshold() {
    // Profile has escalation_threshold = 1000, request amount = 1500
    let ctx = test_context(Decimal::new(1500, 0));
    let rule = make_rule(
        "escalation_threshold",
        serde_json::json!(null),
        PolicyAction::Escalate,
    );
    let result = EscalationThresholdEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Escalate));
}

#[test]
fn escalation_threshold_passes_when_no_threshold_set() {
    let mut ctx = test_context(Decimal::new(999999, 0)); // huge amount
    ctx.profile.escalation_threshold = None;
    let rule = make_rule(
        "escalation_threshold",
        serde_json::json!(null),
        PolicyAction::Escalate,
    );
    let result = EscalationThresholdEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn escalation_threshold_triggers_at_exact_threshold() {
    // At exactly $1000, should escalate (>= semantics — operator intent is
    // "anything at or above this amount needs human approval")
    let ctx = test_context(Decimal::new(1000, 0));
    let rule = make_rule(
        "escalation_threshold",
        serde_json::json!(null),
        PolicyAction::Escalate,
    );
    let result = EscalationThresholdEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Escalate));
}

#[test]
fn escalation_threshold_always_escalates_never_blocks() {
    // Even if the rule's action is Block, the evaluator returns Escalate
    let ctx = test_context(Decimal::new(1500, 0));
    let rule = make_rule(
        "escalation_threshold",
        serde_json::json!(null),
        PolicyAction::Block,
    );
    let result = EscalationThresholdEvaluator.evaluate(&rule, &ctx);
    // The evaluator hardcodes Escalate regardless of rule.action
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Escalate));
}

// ---------------------------------------------------------------------------
// Phase 6.9: metadata field resolution, In operator logging, set_provider
// ---------------------------------------------------------------------------

#[test]
fn condition_evaluator_resolves_metadata_workflow_id() {
    let mut ctx = test_context(Decimal::new(100, 0));
    ctx.request.metadata = Some(cream_models::payment::PaymentMetadata {
        agent_session_id: None,
        workflow_id: Some("wf_abc".to_string()),
        operator_ref: None,
    });

    let condition = PolicyCondition::FieldCheck(FieldCheck {
        field: "metadata.workflow_id".to_string(),
        op: ComparisonOp::Equals,
        value: serde_json::json!("wf_abc"),
    });
    assert!(crate::evaluator::evaluate_condition(&condition, &ctx));
}

#[test]
fn condition_evaluator_metadata_null_when_absent() {
    let ctx = test_context(Decimal::new(100, 0)); // metadata is None

    let condition = PolicyCondition::FieldCheck(FieldCheck {
        field: "metadata.workflow_id".to_string(),
        op: ComparisonOp::Equals,
        value: serde_json::Value::Null,
    });
    assert!(crate::evaluator::evaluate_condition(&condition, &ctx));
}

#[test]
fn condition_in_non_array_returns_false() {
    let ctx = test_context(Decimal::new(100, 0));

    let condition = PolicyCondition::FieldCheck(FieldCheck {
        field: "currency".to_string(),
        op: ComparisonOp::In,
        value: serde_json::json!("not_an_array"),
    });
    // Non-array value for In should return false (fail-safe)
    assert!(!crate::evaluator::evaluate_condition(&condition, &ctx));
}

// ---------------------------------------------------------------------------
// Phase 6.10: Boundary tests — exact threshold semantics
// ---------------------------------------------------------------------------

#[test]
fn amount_cap_passes_at_exact_limit() {
    // $500 == max_per_transaction of $500 → should PASS (uses > not >=)
    let ctx = test_context(Decimal::new(500, 0));
    let rule = make_rule("amount", serde_json::json!(500), PolicyAction::Block);
    let result = AmountCapEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn velocity_limit_passes_at_exact_count() {
    // 4 recent + 1 current = 5, max_count = 5 → should PASS (uses > not >=)
    let mut ctx = test_context(Decimal::new(10, 0));
    for i in 0..4 {
        ctx.recent_payments.push(PaymentSummary {
            amount: Decimal::new(10, 0),
            currency: Currency::SGD,
            recipient_identifier: format!("merchant_{i}"),
            category: PaymentCategory::ApiCredits,
            status: PaymentStatus::Submitted,
            rail: RailPreference::Card,
            created_at: Utc::now() - Duration::minutes(i as i64 * 10),
        });
    }
    let rule = PolicyRule {
        id: PolicyRuleId::new(),
        profile_id: AgentProfileId::new(),
        rule_type: Some("velocity_limit".to_string()),
        priority: 10,
        condition: PolicyCondition::FieldCheck(FieldCheck {
            field: "velocity".to_string(),
            op: ComparisonOp::GreaterThan,
            value: serde_json::json!({"max_count": 5, "window_minutes": 60}),
        }),
        action: PolicyAction::Block,
        escalation: None,
        enabled: true,
    };
    let result = VelocityLimitEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass); // 5 == 5 passes
}

#[test]
fn spend_rate_passes_at_exact_daily_limit() {
    // $500 current + $1500 recent = $2000, daily limit = $2000 → should PASS (uses > not >=)
    let mut ctx = test_context(Decimal::new(500, 0));
    ctx.recent_payments.push(PaymentSummary {
        amount: Decimal::new(1500, 0),
        currency: Currency::SGD,
        recipient_identifier: "merchant_a".to_string(),
        category: PaymentCategory::ApiCredits,
        status: PaymentStatus::Submitted,
        rail: RailPreference::Card,
        created_at: Utc::now() - Duration::hours(2),
    });
    let rule = make_rule("spend_rate", serde_json::json!(true), PolicyAction::Block);
    let result = SpendRateEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass); // $2000 == $2000 passes
}

#[test]
fn spend_rate_triggers_one_cent_over_daily_limit() {
    // $500.01 current + $1500 recent = $2000.01 > $2000 → should TRIGGER
    let mut ctx = test_context(Decimal::new(50001, 2));
    ctx.recent_payments.push(PaymentSummary {
        amount: Decimal::new(1500, 0),
        currency: Currency::SGD,
        recipient_identifier: "merchant_a".to_string(),
        category: PaymentCategory::ApiCredits,
        status: PaymentStatus::Submitted,
        rail: RailPreference::Card,
        created_at: Utc::now() - Duration::hours(2),
    });
    let rule = make_rule("spend_rate", serde_json::json!(true), PolicyAction::Block);
    let result = SpendRateEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block));
}

#[test]
fn time_window_passes_at_start_hour() {
    // Exactly at 9:00 (start of window 9-17) → should PASS (>= start_hour)
    let mut ctx = test_context(Decimal::new(100, 0));
    ctx.current_time = chrono::NaiveDate::from_ymd_opt(2026, 4, 1)
        .unwrap()
        .and_hms_opt(9, 0, 0)
        .unwrap()
        .and_utc();
    let rule = PolicyRule {
        id: PolicyRuleId::new(),
        profile_id: AgentProfileId::new(),
        rule_type: Some("time_window".to_string()),
        priority: 10,
        condition: PolicyCondition::FieldCheck(FieldCheck {
            field: "time_window".to_string(),
            op: ComparisonOp::Equals,
            value: serde_json::json!({"allowed_hours_start": 9, "allowed_hours_end": 17}),
        }),
        action: PolicyAction::Block,
        escalation: None,
        enabled: true,
    };
    let result = TimeWindowEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass); // in window
}

#[test]
fn time_window_blocks_at_end_hour() {
    // Exactly at 17:00 (end of window 9-17) → should be OUTSIDE (< end_hour, exclusive)
    let mut ctx = test_context(Decimal::new(100, 0));
    ctx.current_time = chrono::NaiveDate::from_ymd_opt(2026, 4, 1)
        .unwrap()
        .and_hms_opt(17, 0, 0)
        .unwrap()
        .and_utc();
    let rule = PolicyRule {
        id: PolicyRuleId::new(),
        profile_id: AgentProfileId::new(),
        rule_type: Some("time_window".to_string()),
        priority: 10,
        condition: PolicyCondition::FieldCheck(FieldCheck {
            field: "time_window".to_string(),
            op: ComparisonOp::Equals,
            value: serde_json::json!({"allowed_hours_start": 9, "allowed_hours_end": 17}),
        }),
        action: PolicyAction::Block,
        escalation: None,
        enabled: true,
    };
    let result = TimeWindowEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block)); // outside window
}

// ---------------------------------------------------------------------------
// Fix: Velocity limit currency-aware filtering (v0.6.11)
// ---------------------------------------------------------------------------

#[test]
fn velocity_limit_ignores_different_currency_payments() {
    let mut ctx = test_context(Decimal::new(10, 0));
    // Add 5 USD payments — different currency, should NOT count toward SGD velocity
    for i in 0..5 {
        ctx.recent_payments.push(PaymentSummary {
            amount: Decimal::new(10, 0),
            currency: Currency::USD,
            recipient_identifier: format!("merchant_{i}"),
            category: PaymentCategory::ApiCredits,
            status: PaymentStatus::Submitted,
            rail: RailPreference::Card,
            created_at: Utc::now() - Duration::minutes(i as i64 * 10),
        });
    }
    let rule = PolicyRule {
        id: PolicyRuleId::new(),
        profile_id: AgentProfileId::new(),
        rule_type: Some("velocity_limit".to_string()),
        priority: 10,
        condition: PolicyCondition::FieldCheck(FieldCheck {
            field: "velocity".to_string(),
            op: ComparisonOp::GreaterThan,
            value: serde_json::json!({"max_count": 5, "window_minutes": 60}),
        }),
        action: PolicyAction::Block,
        escalation: None,
        enabled: true,
    };
    let result = VelocityLimitEvaluator.evaluate(&rule, &ctx);
    // Only 1 SGD (current request), USD payments ignored → 1 <= 5
    assert_eq!(result, RuleResult::Pass);
}

#[test]
fn velocity_limit_counts_same_currency_payments() {
    let mut ctx = test_context(Decimal::new(10, 0));
    // Add 5 SGD payments — same currency, should count
    for i in 0..5 {
        ctx.recent_payments.push(PaymentSummary {
            amount: Decimal::new(10, 0),
            currency: Currency::SGD,
            recipient_identifier: format!("merchant_{i}"),
            category: PaymentCategory::ApiCredits,
            status: PaymentStatus::Submitted,
            rail: RailPreference::Card,
            created_at: Utc::now() - Duration::minutes(i as i64 * 10),
        });
    }
    let rule = PolicyRule {
        id: PolicyRuleId::new(),
        profile_id: AgentProfileId::new(),
        rule_type: Some("velocity_limit".to_string()),
        priority: 10,
        condition: PolicyCondition::FieldCheck(FieldCheck {
            field: "velocity".to_string(),
            op: ComparisonOp::GreaterThan,
            value: serde_json::json!({"max_count": 5, "window_minutes": 60}),
        }),
        action: PolicyAction::Block,
        escalation: None,
        enabled: true,
    };
    let result = VelocityLimitEvaluator.evaluate(&rule, &ctx);
    // 5 SGD + 1 current = 6 > 5
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block));
}

// ---------------------------------------------------------------------------
// Fix: First-time merchant case-insensitive matching (v0.6.11)
// ---------------------------------------------------------------------------

#[test]
fn first_time_merchant_case_insensitive_match() {
    // known_merchants has "stripe_merch_123" (lowercase)
    let mut ctx = test_context(Decimal::new(100, 0));
    // Request with uppercase variant — should still match as known
    ctx.request.recipient.identifier = "STRIPE_MERCH_123".to_string();
    let rule = make_rule(
        "first_time_merchant",
        serde_json::json!(true),
        PolicyAction::Escalate,
    );
    let result = FirstTimeMerchantEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass); // recognized as known merchant
}

#[test]
fn first_time_merchant_case_insensitive_mixed_case() {
    let mut ctx = test_context(Decimal::new(100, 0));
    ctx.request.recipient.identifier = "Stripe_Merch_123".to_string();
    let rule = make_rule(
        "first_time_merchant",
        serde_json::json!(true),
        PolicyAction::Escalate,
    );
    let result = FirstTimeMerchantEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass); // recognized as known merchant
}

// ---------------------------------------------------------------------------
// Fix: Duplicate detection case-insensitive merchant matching (v0.6.12)
// ---------------------------------------------------------------------------

#[test]
fn duplicate_detection_case_insensitive_match() {
    let now = Utc::now();
    let mut ctx = test_context(Decimal::new(100, 0));
    // Prior payment used lowercase merchant
    ctx.recent_payments.push(PaymentSummary {
        amount: Decimal::new(100, 0),
        currency: Currency::SGD,
        recipient_identifier: "stripe_merch_123".to_string(),
        created_at: now - Duration::minutes(1),
        status: PaymentStatus::Settled,
        category: PaymentCategory::ApiCredits,
        rail: RailPreference::Card,
    });
    // Current request uses UPPERCASE — should still detect as duplicate
    ctx.request.recipient.identifier = "STRIPE_MERCH_123".to_string();
    ctx.current_time = now;
    let rule = make_rule(
        "duplicate",
        serde_json::json!({"window_minutes": 5}),
        PolicyAction::Block,
    );
    let result = DuplicateDetectionEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block));
}

#[test]
fn duplicate_detection_case_insensitive_mixed_case() {
    let now = Utc::now();
    let mut ctx = test_context(Decimal::new(200, 0));
    ctx.recent_payments.push(PaymentSummary {
        amount: Decimal::new(200, 0),
        currency: Currency::SGD,
        recipient_identifier: "Merchant_ABC".to_string(),
        created_at: now - Duration::minutes(2),
        status: PaymentStatus::Settled,
        category: PaymentCategory::ApiCredits,
        rail: RailPreference::Card,
    });
    ctx.request.recipient.identifier = "merchant_abc".to_string();
    ctx.request.amount = Decimal::new(200, 0);
    ctx.current_time = now;
    let rule = make_rule(
        "duplicate",
        serde_json::json!({"window_minutes": 5}),
        PolicyAction::Block,
    );
    let result = DuplicateDetectionEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Triggered(PolicyAction::Block));
}

// ---------------------------------------------------------------------------
// Fix: Time window start==end rejection (v0.6.12)
// ---------------------------------------------------------------------------

#[test]
fn time_window_start_equals_end_skips_rule() {
    let ctx = test_context(Decimal::new(100, 0));
    // start == end should be rejected as misconfiguration, rule skips (Pass)
    let rule = make_rule(
        "time_window",
        serde_json::json!({"allowed_hours_start": 9, "allowed_hours_end": 9}),
        PolicyAction::Block,
    );
    let result = TimeWindowEvaluator.evaluate(&rule, &ctx);
    assert_eq!(result, RuleResult::Pass); // skipped, not blocking all hours
}

// ---------------------------------------------------------------------------
// Condition evaluator In/NotIn case-insensitivity tests (v0.6.13)
// ---------------------------------------------------------------------------

#[test]
fn condition_in_case_insensitive_string_match() {
    let ctx = test_context(Decimal::new(100, 0));
    // The test request's recipient.identifier is "stripe_merch_123".
    // Check that "In" matches even when the array element uses different casing.
    let condition = PolicyCondition::FieldCheck(FieldCheck {
        field: "recipient.identifier".to_string(),
        op: ComparisonOp::In,
        value: serde_json::json!(["STRIPE_MERCH_123", "other_merchant"]),
    });
    assert!(crate::evaluator::evaluate_condition(&condition, &ctx));
}

#[test]
fn condition_in_exact_case_still_matches() {
    let ctx = test_context(Decimal::new(100, 0));
    let condition = PolicyCondition::FieldCheck(FieldCheck {
        field: "recipient.identifier".to_string(),
        op: ComparisonOp::In,
        value: serde_json::json!(["stripe_merch_123"]),
    });
    assert!(crate::evaluator::evaluate_condition(&condition, &ctx));
}

#[test]
fn condition_not_in_case_insensitive_match() {
    let ctx = test_context(Decimal::new(100, 0));
    // The merchant IS in the array (case-insensitive), so NotIn should return false.
    let condition = PolicyCondition::FieldCheck(FieldCheck {
        field: "recipient.identifier".to_string(),
        op: ComparisonOp::NotIn,
        value: serde_json::json!(["Stripe_Merch_123", "other_merchant"]),
    });
    assert!(!crate::evaluator::evaluate_condition(&condition, &ctx));
}

#[test]
fn condition_in_non_string_values_use_exact_match() {
    let ctx = test_context(Decimal::new(100, 0));
    // Numeric In check — case-insensitivity only applies to strings
    let condition = PolicyCondition::FieldCheck(FieldCheck {
        field: "amount".to_string(),
        op: ComparisonOp::In,
        value: serde_json::json!(["100"]),
    });
    // amount resolves to a string "100" (Decimal serialization), should match
    assert!(crate::evaluator::evaluate_condition(&condition, &ctx));
}
