//! Provider Selection Module
//!
//! Intelligent LLM provider selection based on requirements

use crate::economic::pricing::{ModelPricing, PricingEngine};
use crate::economic::*;

/// Provider selector - chooses optimal LLM provider
pub struct ProviderSelector {
    pricing_engine: PricingEngine,
    config: SelectionConfig,
}

/// Provider selection configuration
#[derive(Debug, Clone)]
pub struct SelectionConfig {
    /// Prefer providers with caching support
    pub prefer_caching: bool,
    /// Minimum quality score required
    pub min_quality_score: f64,
    /// Maximum acceptable latency (ms)
    pub max_latency_ms: u64,
    /// Cost priority (0-1, higher = prefer cheaper)
    pub cost_priority: f64,
    /// Quality priority (0-1, higher = prefer quality)
    pub quality_priority: f64,
    /// Speed priority (0-1, higher = prefer faster)
    pub speed_priority: f64,
}

impl Default for SelectionConfig {
    fn default() -> Self {
        Self {
            prefer_caching: true,
            min_quality_score: 0.8,
            max_latency_ms: 5000,
            cost_priority: 0.5,
            quality_priority: 0.3,
            speed_priority: 0.2,
        }
    }
}

impl ProviderSelector {
    /// Create new provider selector
    pub fn new() -> Self {
        Self {
            pricing_engine: PricingEngine::new(),
            config: SelectionConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: SelectionConfig) -> Self {
        Self {
            pricing_engine: PricingEngine::new(),
            config,
        }
    }

    /// Select best provider for requirements
    pub fn select(&self, requirements: &ProviderRequirements) -> SelectionResult {
        let candidates = self.get_candidates(requirements);

        if candidates.is_empty() {
            return SelectionResult {
                provider: LlmProvider::DeepSeek,
                model: "deepseek-chat".to_string(),
                pricing: self
                    .pricing_engine
                    .get_model_pricing(LlmProvider::DeepSeek, "deepseek-chat"),
                score: 0.0,
                estimated_cost: 0.0,
            };
        }

        // Score each candidate
        let mut scored: Vec<_> = candidates
            .into_iter()
            .map(|(provider, model, cost)| {
                let score = self.calculate_score(&model, cost, requirements);
                (provider, model, cost, score)
            })
            .collect();

        // Sort by score (descending)
        scored.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));

        let (provider, model, cost, score) = scored.into_iter().next().unwrap();

        SelectionResult {
            provider,
            model: model.model.clone(),
            pricing: model,
            score,
            estimated_cost: cost,
        }
    }

    /// Get candidate providers/models
    fn get_candidates(
        &self,
        requirements: &ProviderRequirements,
    ) -> Vec<(LlmProvider, ModelPricing, f64)> {
        let mut candidates = Vec::new();

        for provider in [
            LlmProvider::OpenAI,
            LlmProvider::Anthropic,
            LlmProvider::DeepSeek,
            LlmProvider::Kimi,
            LlmProvider::Doubao,
        ] {
            // Check caching requirement
            if requirements.needs_caching && !provider.supports_caching() {
                continue;
            }

            for model in self.pricing_engine.get_provider_models(provider) {
                // Check quality requirement
                if model.quality_score < requirements.min_quality {
                    continue;
                }

                // Check latency requirement
                if model.avg_latency_ms > requirements.max_latency_ms as u64 {
                    continue;
                }

                // Check context length
                if model.context_window < requirements.context_length {
                    continue;
                }

                let cost = self.pricing_engine.estimate_cost(
                    provider,
                    &model.model,
                    1000, // Assume 1K input tokens
                    500,  // Assume 500 output tokens
                );

                candidates.push((provider, model, cost));
            }
        }

        candidates
    }

    /// Calculate selection score for a model
    fn calculate_score(
        &self,
        model: &ModelPricing,
        cost: f64,
        requirements: &ProviderRequirements,
    ) -> f64 {
        // Normalize cost (lower is better, assume max $0.1 per 1K tokens)
        let cost_score = 1.0 - (cost / 0.1).min(1.0);

        // Quality score (higher is better)
        let quality_score = model.quality_score;

        // Speed score (lower latency is better)
        let speed_score =
            1.0 - (model.avg_latency_ms as f64 / requirements.max_latency_ms as f64).min(1.0);

        // Weighted average
        cost_score * self.config.cost_priority
            + quality_score * self.config.quality_priority
            + speed_score * self.config.speed_priority
    }

    /// Get fallback provider for critical operations
    pub fn fallback_provider(&self) -> (LlmProvider, &'static str) {
        // Use Anthropic for critical operations (highest reliability)
        (LlmProvider::Anthropic, "claude-3-sonnet-20240229")
    }

    /// Get cheapest provider for bulk operations
    pub fn cheapest_provider(&self) -> (LlmProvider, &'static str) {
        (LlmProvider::DeepSeek, "deepseek-chat")
    }

    /// Get fastest provider for real-time operations
    pub fn fastest_provider(&self) -> (LlmProvider, &'static str) {
        (LlmProvider::Anthropic, "claude-3-haiku-20240307")
    }
}

impl Default for ProviderSelector {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of provider selection
#[derive(Debug, Clone)]
pub struct SelectionResult {
    /// Selected provider
    pub provider: LlmProvider,
    /// Selected model
    pub model: String,
    /// Model pricing info
    pub pricing: ModelPricing,
    /// Selection score (higher is better)
    pub score: f64,
    /// Estimated cost per request
    pub estimated_cost: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_provider() {
        let selector = ProviderSelector::new();

        let requirements = ProviderRequirements {
            min_quality: 0.8,
            max_latency_ms: 5000,
            capabilities: vec![],
            context_length: 8000,
            needs_caching: false,
        };

        let result = selector.select(&requirements);

        assert!(result.score > 0.0);
        assert!(result.pricing.quality_score >= 0.8);
    }

    #[test]
    fn test_select_with_caching() {
        let selector = ProviderSelector::new();

        let requirements = ProviderRequirements {
            min_quality: 0.8,
            max_latency_ms: 5000,
            capabilities: vec![],
            context_length: 8000,
            needs_caching: true,
        };

        let result = selector.select(&requirements);

        // Should select a provider that supports caching
        assert!(result.provider.supports_caching());
    }

    #[test]
    fn test_fallback_provider() {
        let selector = ProviderSelector::new();
        let (provider, model) = selector.fallback_provider();

        assert_eq!(provider, LlmProvider::Anthropic);
    }
}
