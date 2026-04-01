use chrono::{Datelike, Duration};
use cream_models::prelude::PolicyRule;
use rust_decimal::Decimal;

use crate::context::EvaluationContext;
use crate::evaluator::{RuleEvaluator, RuleResult};

/// Blocks or escalates when cumulative spend exceeds a threshold within a window.
///
/// Uses the profile's daily/weekly/monthly limits. Sums all payments that
/// count toward spend (settled + in-flight), excluding failed/blocked/rejected.
///
/// Monthly window uses calendar month start (1st of current month at 00:00 UTC)
/// rather than a rolling 30-day window, aligning with how financial reporting
/// typically works.
pub struct SpendRateEvaluator;

impl RuleEvaluator for SpendRateEvaluator {
    fn evaluate(&self, rule: &PolicyRule, ctx: &EvaluationContext) -> RuleResult {
        let now = ctx.current_time;

        // Check daily spend
        let daily_cutoff = now - Duration::days(1);
        let daily_spend = sum_payments_since(ctx, daily_cutoff) + ctx.request.amount;
        if daily_spend > ctx.profile.max_daily_spend {
            return RuleResult::Triggered(rule.action);
        }

        // Check weekly spend
        let weekly_cutoff = now - Duration::weeks(1);
        let weekly_spend = sum_payments_since(ctx, weekly_cutoff) + ctx.request.amount;
        if weekly_spend > ctx.profile.max_weekly_spend {
            return RuleResult::Triggered(rule.action);
        }

        // Check monthly spend — use calendar month start (1st at 00:00 UTC)
        let monthly_cutoff = now
            .with_day(1)
            .and_then(|d| d.date_naive().and_hms_opt(0, 0, 0))
            .map(|naive| naive.and_utc())
            .unwrap_or_else(|| {
                tracing::warn!(
                    "failed to compute calendar month start, falling back to 30-day window"
                );
                now - Duration::days(30)
            });
        let monthly_spend = sum_payments_since(ctx, monthly_cutoff) + ctx.request.amount;
        if monthly_spend > ctx.profile.max_monthly_spend {
            return RuleResult::Triggered(rule.action);
        }

        RuleResult::Pass
    }
}

fn sum_payments_since(ctx: &EvaluationContext, cutoff: chrono::DateTime<chrono::Utc>) -> Decimal {
    ctx.recent_payments
        .iter()
        .filter(|p| p.created_at >= cutoff && p.status.counts_toward_spend())
        .map(|p| p.amount)
        .sum()
}
