use std::collections::HashMap;

use cream_models::prelude::{
    AgentProfile, PaymentRequest, ProviderHealth, ProviderId, RoutingCandidate, RoutingDecision,
};

use crate::error::RoutingError;
use crate::scorer::{ProviderCapabilities, ProviderScorer, ScoredProviderInput};

// ---------------------------------------------------------------------------
// Health source trait
// ---------------------------------------------------------------------------

/// Provides health snapshots for scoring.
///
/// Trait-based so the selector can be tested without Redis.
#[async_trait::async_trait]
pub trait HealthSource: Send + Sync {
    /// Get the health snapshot for a specific provider.
    async fn get_health(&self, provider_id: &ProviderId) -> Result<ProviderHealth, RoutingError>;
}

// ---------------------------------------------------------------------------
// Route selector
// ---------------------------------------------------------------------------

/// Orchestrates provider scoring and selection.
///
/// Loads health data, builds scored inputs, delegates to `ProviderScorer`,
/// and returns a `RoutingDecision`. Does NOT execute payments — the caller
/// (API crate's orchestrator) handles execution and failover.
pub struct RouteSelector {
    scorer: ProviderScorer,
    capabilities: HashMap<ProviderId, ProviderCapabilities>,
    health_source: Box<dyn HealthSource>,
}

impl RouteSelector {
    pub fn new(
        scorer: ProviderScorer,
        capabilities: HashMap<ProviderId, ProviderCapabilities>,
        health_source: Box<dyn HealthSource>,
    ) -> Self {
        Self {
            scorer,
            capabilities,
            health_source,
        }
    }

    /// Select the optimal provider for a payment.
    ///
    /// Returns a `RoutingDecision` with ranked candidates. The top candidate
    /// is the selected provider. If no viable provider exists, returns
    /// `RoutingError::NoViableProvider`.
    pub async fn select(
        &self,
        request: &PaymentRequest,
        profile: &AgentProfile,
    ) -> Result<RoutingDecision, RoutingError> {
        // Build scored inputs: capabilities + health for each known provider
        let mut inputs = Vec::new();
        for (provider_id, caps) in &self.capabilities {
            let health = match self.health_source.get_health(provider_id).await {
                Ok(h) => h,
                Err(e) => {
                    tracing::warn!(
                        provider = %provider_id,
                        error = %e,
                        "failed to get health for provider, excluding from candidates"
                    );
                    continue;
                }
            };
            inputs.push(ScoredProviderInput {
                capabilities: caps.clone(),
                health,
            });
        }

        // Score and rank
        let candidates = self.scorer.score_candidates(&inputs, request, profile);

        if candidates.is_empty() {
            tracing::warn!(
                currency = ?request.currency,
                preferred_rail = ?request.preferred_rail,
                "no viable provider found"
            );
            return Err(RoutingError::NoViableProvider);
        }

        let reason = build_reason(&candidates);
        let selected_provider = candidates[0].provider_id.clone();
        let selected_rail = candidates[0].rail;
        let score = candidates[0].score;

        tracing::info!(
            selected_provider = %selected_provider,
            selected_rail = ?selected_rail,
            score,
            candidate_count = candidates.len(),
            reason = %reason,
            "provider selected"
        );

        Ok(RoutingDecision {
            candidates,
            selected: selected_provider,
            selected_rail,
            reason,
        })
    }
}

/// Build a human-readable reason string for the routing decision.
fn build_reason(candidates: &[RoutingCandidate]) -> String {
    if candidates.len() == 1 {
        return "only_viable_provider".to_string();
    }

    let top = &candidates[0];
    let runner_up = &candidates[1];
    let margin = top.score - runner_up.score;

    if margin.abs() < 0.01 {
        format!(
            "tied_score_with_{}_selected_{}",
            runner_up.provider_id, top.provider_id
        )
    } else {
        format!(
            "highest_score_{:.3}_over_{}_at_{:.3}",
            top.score, runner_up.provider_id, runner_up.score
        )
    }
}

// ---------------------------------------------------------------------------
// In-memory health source (for tests)
// ---------------------------------------------------------------------------

/// Static health source that returns pre-configured health snapshots.
pub struct StaticHealthSource {
    health: HashMap<ProviderId, ProviderHealth>,
}

impl StaticHealthSource {
    pub fn new(health: HashMap<ProviderId, ProviderHealth>) -> Self {
        Self { health }
    }
}

#[async_trait::async_trait]
impl HealthSource for StaticHealthSource {
    async fn get_health(&self, provider_id: &ProviderId) -> Result<ProviderHealth, RoutingError> {
        self.health
            .get(provider_id)
            .cloned()
            .ok_or_else(|| RoutingError::Config(format!("no health data for {provider_id}")))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use cream_models::prelude::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn make_caps(
        id: &str,
        rails: Vec<RailPreference>,
        currencies: Vec<Currency>,
        fee_pct: &str,
    ) -> ProviderCapabilities {
        ProviderCapabilities {
            provider_id: ProviderId::new(id),
            supported_rails: rails,
            supported_currencies: currencies,
            fee_percentage: Decimal::from_str(fee_pct).unwrap(),
            flat_fee_usd: Decimal::from_str("0.30").unwrap(),
        }
    }

    fn make_health(id: &str, error_rate: f64, p50: u64) -> ProviderHealth {
        ProviderHealth {
            provider_id: ProviderId::new(id),
            is_healthy: true,
            error_rate_5m: error_rate,
            p50_latency_ms: p50,
            p99_latency_ms: p50 * 3,
            last_checked_at: Utc::now(),
            circuit_state: CircuitState::Closed,
        }
    }

    fn sample_request() -> PaymentRequest {
        PaymentRequest {
            agent_id: AgentId::new(),
            amount: Decimal::new(10000, 2),
            currency: Currency::SGD,
            recipient: Recipient {
                recipient_type: RecipientType::Merchant,
                identifier: "merch_123".to_string(),
                name: None,
                country: Some(CountryCode::new("SG")),
            },
            preferred_rail: RailPreference::Auto,
            justification: Justification {
                summary: "Test payment for API credits via automated processing workflow"
                    .to_string(),
                task_id: None,
                category: PaymentCategory::ApiCredits,
                expected_value: None,
            },
            metadata: None,
            idempotency_key: IdempotencyKey::new("idem_sel_test"),
        }
    }

    fn sample_profile() -> AgentProfile {
        AgentProfile {
            id: AgentProfileId::new(),
            name: "test-profile".to_string(),
            version: 1,
            max_per_transaction: Decimal::new(100000, 2),
            max_daily_spend: Decimal::new(500000, 2),
            max_weekly_spend: Decimal::new(2000000, 2),
            max_monthly_spend: Decimal::new(5000000, 2),
            allowed_categories: vec![],
            allowed_rails: vec![],
            geographic_restrictions: vec![],
            escalation_threshold: None,
            timezone: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_selector(
        caps: Vec<ProviderCapabilities>,
        health: Vec<ProviderHealth>,
    ) -> RouteSelector {
        let cap_map: HashMap<ProviderId, ProviderCapabilities> = caps
            .into_iter()
            .map(|c| (c.provider_id.clone(), c))
            .collect();
        let health_map: HashMap<ProviderId, ProviderHealth> = health
            .into_iter()
            .map(|h| (h.provider_id.clone(), h))
            .collect();

        RouteSelector::new(
            ProviderScorer::new(crate::config::ScoringWeights::default()),
            cap_map,
            Box::new(StaticHealthSource::new(health_map)),
        )
    }

    #[tokio::test]
    async fn single_provider_selected() {
        let selector = make_selector(
            vec![make_caps(
                "stripe",
                vec![RailPreference::Card],
                vec![Currency::SGD],
                "0.029",
            )],
            vec![make_health("stripe", 0.01, 150)],
        );

        let result = selector.select(&sample_request(), &sample_profile()).await;
        assert!(result.is_ok());
        let decision = result.unwrap();
        assert_eq!(decision.selected.as_str(), "stripe");
        assert_eq!(decision.reason, "only_viable_provider");
    }

    #[tokio::test]
    async fn highest_scored_selected() {
        let selector = make_selector(
            vec![
                make_caps(
                    "expensive",
                    vec![RailPreference::Card],
                    vec![Currency::SGD],
                    "0.05",
                ),
                make_caps(
                    "cheap",
                    vec![RailPreference::Card],
                    vec![Currency::SGD],
                    "0.01",
                ),
            ],
            vec![
                make_health("expensive", 0.3, 300),
                make_health("cheap", 0.01, 100),
            ],
        );

        let decision = selector
            .select(&sample_request(), &sample_profile())
            .await
            .unwrap();
        assert_eq!(decision.selected.as_str(), "cheap");
        assert_eq!(decision.candidates.len(), 2);
    }

    #[tokio::test]
    async fn no_providers_returns_error() {
        let selector = make_selector(vec![], vec![]);
        let result = selector.select(&sample_request(), &sample_profile()).await;
        assert!(matches!(result, Err(RoutingError::NoViableProvider)));
    }

    #[tokio::test]
    async fn all_circuit_broken_returns_error() {
        let mut health = make_health("stripe", 0.9, 100);
        health.circuit_state = CircuitState::Open;

        let selector = make_selector(
            vec![make_caps(
                "stripe",
                vec![RailPreference::Card],
                vec![Currency::SGD],
                "0.029",
            )],
            vec![health],
        );

        let result = selector.select(&sample_request(), &sample_profile()).await;
        assert!(matches!(result, Err(RoutingError::NoViableProvider)));
    }

    #[tokio::test]
    async fn currency_mismatch_returns_error() {
        let selector = make_selector(
            vec![make_caps(
                "usd_only",
                vec![RailPreference::Card],
                vec![Currency::USD],
                "0.029",
            )],
            vec![make_health("usd_only", 0.01, 100)],
        );

        // Request is for SGD
        let result = selector.select(&sample_request(), &sample_profile()).await;
        assert!(matches!(result, Err(RoutingError::NoViableProvider)));
    }

    #[tokio::test]
    async fn rail_restriction_filters() {
        let selector = make_selector(
            vec![
                make_caps(
                    "card",
                    vec![RailPreference::Card],
                    vec![Currency::SGD],
                    "0.029",
                ),
                make_caps(
                    "crypto",
                    vec![RailPreference::Stablecoin],
                    vec![Currency::SGD],
                    "0.001",
                ),
            ],
            vec![
                make_health("card", 0.01, 100),
                make_health("crypto", 0.01, 50),
            ],
        );

        let mut profile = sample_profile();
        profile.allowed_rails = vec![RailPreference::Card];

        let decision = selector.select(&sample_request(), &profile).await.unwrap();
        assert_eq!(decision.selected.as_str(), "card");
        assert_eq!(decision.candidates.len(), 1);
    }

    #[tokio::test]
    async fn health_failure_excludes_provider() {
        // Only register capabilities for "stripe" but no health data
        let cap_map: HashMap<ProviderId, ProviderCapabilities> = [(
            ProviderId::new("stripe"),
            make_caps(
                "stripe",
                vec![RailPreference::Card],
                vec![Currency::SGD],
                "0.029",
            ),
        )]
        .into_iter()
        .collect();

        // Empty health source — get_health will fail
        let selector = RouteSelector::new(
            ProviderScorer::new(crate::config::ScoringWeights::default()),
            cap_map,
            Box::new(StaticHealthSource::new(HashMap::new())),
        );

        let result = selector.select(&sample_request(), &sample_profile()).await;
        assert!(matches!(result, Err(RoutingError::NoViableProvider)));
    }
}
