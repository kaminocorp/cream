//! # cream-policy
//!
//! Declarative rule evaluation engine for the Cream payment control plane.
//!
//! Stateless and purely computational — receives an [`EvaluationContext`] with
//! all pre-loaded data and returns a [`PolicyDecision`] (Approve / Block / Escalate).
//! Zero database dependencies, trivially unit-testable.
//!
//! The engine evaluates rules in priority order with first-block-wins,
//! escalation-accumulates semantics. 12 built-in rule types cover amount caps,
//! velocity limits, category restrictions, geographic restrictions, and more.

pub mod context;
pub mod engine;
pub mod error;
pub mod evaluator;
pub mod rules;

// Convenience re-exports
pub use context::{EvaluationContext, PaymentSummary};
pub use engine::{PolicyDecision, PolicyEngine};
pub use error::PolicyError;
pub use evaluator::{RuleEvaluator, RuleResult};

#[cfg(test)]
mod tests;
