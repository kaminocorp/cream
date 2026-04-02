use rust_decimal::Decimal;

use cream_models::prelude::{
    AgentProfile, CircuitState, Currency, PaymentRequest, ProviderHealth, ProviderId,
    RailPreference, RoutingCandidate,
};

use crate::config::ScoringWeights;
use crate::error::RoutingError;

// ---------------------------------------------------------------------------
// Provider capabilities (scaffold — hardcoded in production phases 12-14)
// ---------------------------------------------------------------------------

/// Static capabilities for a registered provider.
///
/// In production (Phases 12-14), these are loaded from provider configuration
/// or queried from provider APIs. In the scaffold, they are constructed by
/// the caller with placeholder data.
#[derive(Debug, Clone)]
pub struct ProviderCapabilities {
    pub provider_id: ProviderId,
    /// Rails this provider supports.
    pub supported_rails: Vec<RailPreference>,
    /// Currencies this provider supports.
    pub supported_currencies: Vec<Currency>,
    /// Estimated fee as a fraction of the transaction amount (e.g., 0.029 = 2.9%).
    pub fee_percentage: Decimal,
    /// Flat fee per transaction in USD equivalent.
    pub flat_fee_usd: Decimal,
}

/// Input bundle for scoring a single provider.
#[derive(Debug, Clone)]
pub struct ScoredProviderInput {
    pub capabilities: ProviderCapabilities,
    pub health: ProviderHealth,
}

// ---------------------------------------------------------------------------
// Provider scorer
// ---------------------------------------------------------------------------

/// Scores candidate providers for a given payment context.
///
/// Each provider is assigned a composite score based on cost, speed, health,
/// and rail preference match. Binary filters (circuit breaker, currency,
/// rail policy) exclude non-viable providers before scoring.
#[derive(Debug)]
pub struct ProviderScorer {
    weights: ScoringWeights,
}

impl ProviderScorer {
    pub fn new(weights: ScoringWeights) -> Result<Self, RoutingError> {
        weights.validate()?;
        Ok(Self { weights })
    }

    /// Score all viable providers and return ranked candidates (highest score first).
    ///
    /// Providers are excluded if:
    /// - Circuit breaker is in `Open` state
    /// - Provider does not support the requested currency
    /// - Provider's supported rails have no overlap with profile's `allowed_rails`
    ///   (when the profile's list is non-empty)
    pub fn score_candidates(
        &self,
        providers: &[ScoredProviderInput],
        request: &PaymentRequest,
        profile: &AgentProfile,
    ) -> Vec<RoutingCandidate> {
        // Phase 1: filter to viable providers
        let viable: Vec<&ScoredProviderInput> = providers
            .iter()
            .filter(|p| {
                // Exclude circuit-broken providers
                if p.health.circuit_state == CircuitState::Open {
                    tracing::debug!(
                        provider = %p.capabilities.provider_id,
                        "excluded: circuit breaker open"
                    );
                    return false;
                }

                // Exclude providers that don't support the currency
                if !p
                    .capabilities
                    .supported_currencies
                    .contains(&request.currency)
                {
                    tracing::debug!(
                        provider = %p.capabilities.provider_id,
                        currency = ?request.currency,
                        "excluded: currency not supported"
                    );
                    return false;
                }

                // Exclude providers whose rails don't overlap with profile restrictions
                if !profile.allowed_rails.is_empty() {
                    let has_overlap = p
                        .capabilities
                        .supported_rails
                        .iter()
                        .any(|r| profile.allowed_rails.contains(r));
                    if !has_overlap {
                        tracing::debug!(
                            provider = %p.capabilities.provider_id,
                            "excluded: no rail overlap with profile restrictions"
                        );
                        return false;
                    }
                }

                true
            })
            .collect();

        if viable.is_empty() {
            return Vec::new();
        }

        // Phase 2: compute estimated fees for normalization
        let fees: Vec<Decimal> = viable
            .iter()
            .map(|p| estimate_fee(&p.capabilities, request.amount))
            .collect();

        let max_fee = fees.iter().copied().max().unwrap_or(Decimal::ZERO);
        let max_latency = viable
            .iter()
            .map(|p| p.health.p50_latency_ms)
            .max()
            .unwrap_or(1);

        // Phase 3: score each viable provider
        let mut candidates: Vec<RoutingCandidate> = viable
            .iter()
            .zip(fees.iter())
            .map(|(p, &fee)| {
                let cost_score = if max_fee.is_zero() {
                    1.0
                } else {
                    let fee_f64 = decimal_to_f64(fee);
                    let max_f64 = decimal_to_f64(max_fee);
                    1.0 - (fee_f64 / max_f64)
                };

                let speed_score = if max_latency == 0 {
                    1.0
                } else {
                    1.0 - (p.health.p50_latency_ms as f64 / max_latency as f64)
                };

                let health_score = (1.0 - p.health.error_rate_5m).max(0.0);

                let preference_score = if request.preferred_rail == RailPreference::Auto
                    || p.capabilities
                        .supported_rails
                        .contains(&request.preferred_rail)
                {
                    1.0
                } else {
                    0.0
                };

                let score = (self.weights.cost * cost_score)
                    + (self.weights.speed * speed_score)
                    + (self.weights.health * health_score)
                    + (self.weights.preference * preference_score);

                // Select the best matching rail for this candidate
                let selected_rail = select_rail(
                    &p.capabilities.supported_rails,
                    request.preferred_rail,
                    &profile.allowed_rails,
                );

                RoutingCandidate {
                    provider_id: p.capabilities.provider_id.clone(),
                    rail: selected_rail,
                    estimated_fee: fee,
                    estimated_latency_ms: p.health.p50_latency_ms,
                    score,
                }
            })
            .collect();

        // Sort by score descending (highest score = best provider)
        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        candidates
    }
}

/// Estimate the total fee for a transaction amount given provider capabilities.
fn estimate_fee(caps: &ProviderCapabilities, amount: Decimal) -> Decimal {
    caps.flat_fee_usd + (amount * caps.fee_percentage)
}

/// Select the best rail from a provider's supported rails.
fn select_rail(
    supported: &[RailPreference],
    preferred: RailPreference,
    allowed: &[RailPreference],
) -> RailPreference {
    // If the agent's preferred rail is supported (and allowed), use it
    if preferred != RailPreference::Auto
        && supported.contains(&preferred)
        && (allowed.is_empty() || allowed.contains(&preferred))
    {
        return preferred;
    }

    // Otherwise pick the first supported rail that's allowed
    for rail in supported {
        if allowed.is_empty() || allowed.contains(rail) {
            return *rail;
        }
    }

    // Fallback — should not reach here if filtering was correct
    RailPreference::Auto
}

/// Convert a `Decimal` to `f64` for scoring purposes only.
/// Scores are non-financial, so f64 precision is acceptable.
/// Uses `rust_decimal`'s native `to_f64()` instead of string round-tripping.
fn decimal_to_f64(d: Decimal) -> f64 {
    use rust_decimal::prelude::ToPrimitive;
    d.to_f64().unwrap_or(0.0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use cream_models::prelude::*;
    use std::str::FromStr;

    fn make_health(provider_id: &str, error_rate: f64, p50: u64) -> ProviderHealth {
        ProviderHealth {
            provider_id: ProviderId::new(provider_id),
            is_healthy: true,
            error_rate_5m: error_rate,
            p50_latency_ms: p50,
            p99_latency_ms: p50 * 3,
            last_checked_at: Utc::now(),
            circuit_state: CircuitState::Closed,
        }
    }

    fn make_caps(
        provider_id: &str,
        rails: Vec<RailPreference>,
        currencies: Vec<Currency>,
        fee_pct: &str,
    ) -> ProviderCapabilities {
        ProviderCapabilities {
            provider_id: ProviderId::new(provider_id),
            supported_rails: rails,
            supported_currencies: currencies,
            fee_percentage: Decimal::from_str(fee_pct).unwrap(),
            flat_fee_usd: Decimal::from_str("0.30").unwrap(),
        }
    }

    fn make_input(caps: ProviderCapabilities, health: ProviderHealth) -> ScoredProviderInput {
        ScoredProviderInput {
            capabilities: caps,
            health,
        }
    }

    fn sample_request() -> PaymentRequest {
        PaymentRequest {
            agent_id: AgentId::new(),
            amount: Decimal::new(10000, 2), // 100.00
            currency: Currency::SGD,
            recipient: Recipient {
                recipient_type: RecipientType::Merchant,
                identifier: "merch_123".to_string(),
                name: None,
                country: Some(CountryCode::new("SG")),
            },
            preferred_rail: RailPreference::Auto,
            justification: Justification {
                summary: "Test payment for API credits purchase via automated workflow".to_string(),
                task_id: None,
                category: PaymentCategory::ApiCredits,
                expected_value: None,
            },
            metadata: None,
            idempotency_key: IdempotencyKey::new("idem_test"),
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

    #[test]
    fn single_provider_returns_candidate() {
        let scorer = ProviderScorer::new(ScoringWeights::default()).unwrap();
        let providers = vec![make_input(
            make_caps(
                "stripe",
                vec![RailPreference::Card],
                vec![Currency::SGD],
                "0.029",
            ),
            make_health("stripe", 0.01, 150),
        )];

        let result = scorer.score_candidates(&providers, &sample_request(), &sample_profile());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].provider_id.as_str(), "stripe");
        assert!(result[0].score > 0.0);
    }

    #[test]
    fn faster_provider_ranked_higher_with_speed_weight() {
        let weights = ScoringWeights {
            cost: 0.0,
            speed: 1.0,
            health: 0.0,
            preference: 0.0,
        };
        let scorer = ProviderScorer::new(weights).unwrap();
        let providers = vec![
            make_input(
                make_caps(
                    "slow",
                    vec![RailPreference::Card],
                    vec![Currency::SGD],
                    "0.029",
                ),
                make_health("slow", 0.01, 500),
            ),
            make_input(
                make_caps(
                    "fast",
                    vec![RailPreference::Card],
                    vec![Currency::SGD],
                    "0.029",
                ),
                make_health("fast", 0.01, 100),
            ),
        ];

        let result = scorer.score_candidates(&providers, &sample_request(), &sample_profile());
        assert_eq!(result[0].provider_id.as_str(), "fast");
    }

    #[test]
    fn cheaper_provider_ranked_higher_with_cost_weight() {
        let weights = ScoringWeights {
            cost: 1.0,
            speed: 0.0,
            health: 0.0,
            preference: 0.0,
        };
        let scorer = ProviderScorer::new(weights).unwrap();
        let providers = vec![
            make_input(
                make_caps(
                    "expensive",
                    vec![RailPreference::Card],
                    vec![Currency::SGD],
                    "0.05",
                ),
                make_health("expensive", 0.01, 100),
            ),
            make_input(
                make_caps(
                    "cheap",
                    vec![RailPreference::Card],
                    vec![Currency::SGD],
                    "0.01",
                ),
                make_health("cheap", 0.01, 100),
            ),
        ];

        let result = scorer.score_candidates(&providers, &sample_request(), &sample_profile());
        assert_eq!(result[0].provider_id.as_str(), "cheap");
    }

    #[test]
    fn unhealthy_provider_demoted() {
        let weights = ScoringWeights {
            cost: 0.0,
            speed: 0.0,
            health: 1.0,
            preference: 0.0,
        };
        let scorer = ProviderScorer::new(weights).unwrap();
        let providers = vec![
            make_input(
                make_caps(
                    "sick",
                    vec![RailPreference::Card],
                    vec![Currency::SGD],
                    "0.01",
                ),
                make_health("sick", 0.8, 100),
            ),
            make_input(
                make_caps(
                    "healthy",
                    vec![RailPreference::Card],
                    vec![Currency::SGD],
                    "0.05",
                ),
                make_health("healthy", 0.02, 100),
            ),
        ];

        let result = scorer.score_candidates(&providers, &sample_request(), &sample_profile());
        assert_eq!(result[0].provider_id.as_str(), "healthy");
    }

    #[test]
    fn open_circuit_excluded() {
        let scorer = ProviderScorer::new(ScoringWeights::default()).unwrap();
        let mut health = make_health("broken", 0.9, 100);
        health.circuit_state = CircuitState::Open;

        let providers = vec![make_input(
            make_caps(
                "broken",
                vec![RailPreference::Card],
                vec![Currency::SGD],
                "0.01",
            ),
            health,
        )];

        let result = scorer.score_candidates(&providers, &sample_request(), &sample_profile());
        assert!(result.is_empty());
    }

    #[test]
    fn unsupported_currency_excluded() {
        let scorer = ProviderScorer::new(ScoringWeights::default()).unwrap();
        let providers = vec![make_input(
            make_caps(
                "usd_only",
                vec![RailPreference::Card],
                vec![Currency::USD],
                "0.029",
            ),
            make_health("usd_only", 0.01, 100),
        )];

        // Request is for SGD, provider only supports USD
        let result = scorer.score_candidates(&providers, &sample_request(), &sample_profile());
        assert!(result.is_empty());
    }

    #[test]
    fn rail_restriction_filters_providers() {
        let scorer = ProviderScorer::new(ScoringWeights::default()).unwrap();
        let providers = vec![
            make_input(
                make_caps(
                    "card_only",
                    vec![RailPreference::Card],
                    vec![Currency::SGD],
                    "0.029",
                ),
                make_health("card_only", 0.01, 100),
            ),
            make_input(
                make_caps(
                    "crypto",
                    vec![RailPreference::Stablecoin],
                    vec![Currency::SGD],
                    "0.001",
                ),
                make_health("crypto", 0.01, 50),
            ),
        ];

        // Profile only allows Card
        let mut profile = sample_profile();
        profile.allowed_rails = vec![RailPreference::Card];

        let result = scorer.score_candidates(&providers, &sample_request(), &profile);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].provider_id.as_str(), "card_only");
    }

    #[test]
    fn preference_match_boosts_score() {
        let weights = ScoringWeights {
            cost: 0.0,
            speed: 0.0,
            health: 0.0,
            preference: 1.0,
        };
        let scorer = ProviderScorer::new(weights).unwrap();

        let providers = vec![
            make_input(
                make_caps(
                    "card",
                    vec![RailPreference::Card],
                    vec![Currency::SGD],
                    "0.029",
                ),
                make_health("card", 0.01, 100),
            ),
            make_input(
                make_caps(
                    "swift",
                    vec![RailPreference::Swift],
                    vec![Currency::SGD],
                    "0.01",
                ),
                make_health("swift", 0.01, 100),
            ),
        ];

        let mut req = sample_request();
        req.preferred_rail = RailPreference::Card;

        let result = scorer.score_candidates(&providers, &req, &sample_profile());
        assert_eq!(result[0].provider_id.as_str(), "card");
    }

    #[test]
    fn auto_preference_matches_all() {
        let weights = ScoringWeights {
            cost: 0.0,
            speed: 0.0,
            health: 0.0,
            preference: 1.0,
        };
        let scorer = ProviderScorer::new(weights).unwrap();

        let providers = vec![
            make_input(
                make_caps(
                    "a",
                    vec![RailPreference::Card],
                    vec![Currency::SGD],
                    "0.029",
                ),
                make_health("a", 0.01, 100),
            ),
            make_input(
                make_caps(
                    "b",
                    vec![RailPreference::Swift],
                    vec![Currency::SGD],
                    "0.01",
                ),
                make_health("b", 0.01, 100),
            ),
        ];

        let req = sample_request(); // preferred_rail = Auto
        let result = scorer.score_candidates(&providers, &req, &sample_profile());
        // Both should have preference_score = 1.0, so scores should be equal
        assert_eq!(result.len(), 2);
        assert!((result[0].score - result[1].score).abs() < f64::EPSILON);
    }

    #[test]
    fn empty_providers_returns_empty() {
        let scorer = ProviderScorer::new(ScoringWeights::default()).unwrap();
        let result = scorer.score_candidates(&[], &sample_request(), &sample_profile());
        assert!(result.is_empty());
    }

    #[test]
    fn all_zero_weights_rejected_by_scorer() {
        let weights = ScoringWeights {
            cost: 0.0,
            speed: 0.0,
            health: 0.0,
            preference: 0.0,
        };
        let result = ProviderScorer::new(weights);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("non-zero"));
    }

    #[test]
    fn half_open_circuit_not_excluded() {
        let scorer = ProviderScorer::new(ScoringWeights::default()).unwrap();
        let mut health = make_health("recovering", 0.3, 200);
        health.circuit_state = CircuitState::HalfOpen;

        let providers = vec![make_input(
            make_caps(
                "recovering",
                vec![RailPreference::Card],
                vec![Currency::SGD],
                "0.029",
            ),
            health,
        )];

        let result = scorer.score_candidates(&providers, &sample_request(), &sample_profile());
        assert_eq!(result.len(), 1);
    }

    // -----------------------------------------------------------------------
    // Phase 7.2: ProviderScorer rejects invalid config
    // -----------------------------------------------------------------------

    #[test]
    fn scorer_rejects_nan_weight() {
        let weights = ScoringWeights {
            cost: f64::NAN,
            ..Default::default()
        };
        let result = ProviderScorer::new(weights);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cost"));
    }

    #[test]
    fn scorer_rejects_negative_weight() {
        let weights = ScoringWeights {
            speed: -0.1,
            ..Default::default()
        };
        let result = ProviderScorer::new(weights);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("speed"));
    }
}
