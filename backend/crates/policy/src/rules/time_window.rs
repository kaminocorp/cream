use chrono::{FixedOffset, Timelike};
use cream_models::prelude::{PolicyCondition, PolicyRule};

use crate::context::EvaluationContext;
use crate::evaluator::{RuleEvaluator, RuleResult};

/// Blocks payments outside allowed time windows.
///
/// The rule's condition tree should contain a FieldCheck with field "time_window"
/// and value `{"allowed_hours_start": H, "allowed_hours_end": H}` in 24h format.
/// Hours are evaluated in the agent profile's timezone (or UTC if no timezone set).
/// An optional `"utc_offset_hours"` field in the condition value can override
/// the profile timezone for per-rule flexibility.
///
/// If current time is outside [start, end), the rule triggers.
pub struct TimeWindowEvaluator;

impl RuleEvaluator for TimeWindowEvaluator {
    fn evaluate(&self, rule: &PolicyRule, ctx: &EvaluationContext) -> RuleResult {
        let (start_hour, end_hour, offset_override) = match extract_hours(&rule.condition) {
            Some(h) => h,
            None => {
                tracing::error!(
                    rule_id = %rule.id,
                    "time_window rule has missing or invalid config (expected allowed_hours_start + allowed_hours_end), \
                     failing safe — treating as triggered to prevent policy bypass"
                );
                return RuleResult::Triggered(rule.action);
            }
        };

        // Determine UTC offset: condition override > profile timezone > UTC
        // offset_override is already in seconds from extract_hours
        let utc_offset_secs = offset_override
            .or_else(|| parse_timezone_offset(ctx.profile.timezone.as_deref()))
            .unwrap_or(0);

        let offset = match FixedOffset::east_opt(utc_offset_secs) {
            Some(o) => o,
            None => {
                tracing::warn!(utc_offset_secs, "invalid UTC offset, falling back to UTC");
                FixedOffset::east_opt(0).expect("UTC offset 0 is always valid")
            }
        };

        let local_time = ctx.current_time.with_timezone(&offset);
        let current_hour = local_time.hour();

        let in_window = if start_hour <= end_hour {
            // Normal range: e.g., 9..17
            current_hour >= start_hour && current_hour < end_hour
        } else {
            // Overnight range: e.g., 22..6 means 22-23 and 0-5
            current_hour >= start_hour || current_hour < end_hour
        };

        if in_window {
            RuleResult::Pass
        } else {
            RuleResult::Triggered(rule.action)
        }
    }
}

/// Parse a timezone string into a UTC offset in seconds.
/// Supports common IANA-style timezone abbreviations mapped to fixed offsets.
/// Full IANA database support (e.g., DST transitions) would require the `chrono-tz` crate.
fn parse_timezone_offset(tz: Option<&str>) -> Option<i32> {
    match tz? {
        "UTC" | "GMT" => Some(0),
        "Asia/Singapore" | "Asia/Kuala_Lumpur" | "SGT" => Some(8 * 3600),
        "Asia/Tokyo" | "JST" => Some(9 * 3600),
        "Asia/Shanghai" | "Asia/Hong_Kong" | "CST" | "HKT" => Some(8 * 3600),
        "Asia/Kolkata" | "IST" => Some(5 * 3600 + 1800),
        "America/New_York" | "EST" => Some(-5 * 3600),
        "America/Chicago" | "CST6CDT" => Some(-6 * 3600),
        "America/Denver" | "MST" => Some(-7 * 3600),
        "America/Los_Angeles" | "PST" => Some(-8 * 3600),
        "Europe/London" | "WET" => Some(0),
        "Europe/Berlin" | "Europe/Paris" | "CET" => Some(3600),
        "Europe/Moscow" | "MSK" => Some(3 * 3600),
        "Australia/Sydney" | "AEST" => Some(10 * 3600),
        other => {
            tracing::warn!(
                timezone = other,
                "unrecognized timezone, falling back to UTC. Add chrono-tz for full IANA support"
            );
            None
        }
    }
}

/// Returns (start_hour, end_hour, optional_utc_offset_override_in_seconds).
fn extract_hours(condition: &PolicyCondition) -> Option<(u32, u32, Option<i32>)> {
    match condition {
        PolicyCondition::FieldCheck(check) if check.field == "time_window" => {
            let start = check.value.get("allowed_hours_start")?.as_u64()? as u32;
            let end = check.value.get("allowed_hours_end")?.as_u64()? as u32;
            if start > 23 || end > 23 {
                tracing::error!(
                    start,
                    end,
                    "time_window hours out of 0-23 range — failing safe (rule will trigger)"
                );
                return None;
            }
            if start == end {
                tracing::error!(
                    start,
                    end,
                    "time_window start == end defines a zero-width window — failing safe (rule will trigger)"
                );
                return None;
            }
            let offset = check
                .value
                .get("utc_offset_hours")
                .and_then(|v| v.as_i64())
                .and_then(|h| {
                    // Valid UTC offsets range from -26 to +26 (covers all
                    // real-world timezones with generous headroom). Reject
                    // extreme values to prevent i32 overflow on the * 3600
                    // multiplication (which panics in debug mode).
                    if !(-26..=26).contains(&h) {
                        tracing::error!(
                            utc_offset_hours = h,
                            "utc_offset_hours out of valid range (-26..=26), ignoring override"
                        );
                        return None;
                    }
                    Some(h as i32 * 3600)
                });
            Some((start, end, offset))
        }
        PolicyCondition::All(children) | PolicyCondition::Any(children) => {
            children.iter().find_map(extract_hours)
        }
        PolicyCondition::Not(inner) => extract_hours(inner),
        _ => None,
    }
}
