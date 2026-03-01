//! Pricing Engine Module
//!
//! Calculates costs for different LLM providers and models

use crate::economic::LlmProvider;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pricing engine - calculates token costs
pub struct PricingEngine {
    /// Model pricing tables
    pricing: HashMap<LlmProvider, Vec<ModelPricing>>,
    /// Cache pricing (for providers that support it)
    cache_pricing: HashMap<LlmProvider, CachePricing>,
}

/// Pricing for a specific model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Model identifier
    pub model: String,
    /// Input token price per million tokens (USD)
    pub input_price_per_million: f64,
    /// Output token price per million tokens (USD)
    pub output_price_per_million: f64,
    /// Context window size
    pub context_window: usize,
    /// Quality score (0.0-1.0)
    pub quality_score: f64,
    /// Average latency in milliseconds
    pub avg_latency_ms: u64,
}

/// Cache pricing details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePricing {
    /// Cache write price per million tokens
    pub write_price_per_million: f64,
    /// Cache read price per million tokens
    pub read_price_per_million: f64,
    /// Cache TTL in seconds
    pub ttl_seconds: u64,
}

impl PricingEngine {
    /// Create pricing engine with default prices
    pub fn new() -> Self {
        let mut pricing = HashMap::new();

        // OpenAI pricing (as of 2024)
        pricing.insert(
            LlmProvider::OpenAI,
            vec![
                ModelPricing {
                    model: "gpt-4-turbo-preview".to_string(),
                    input_price_per_million: 10.00,
                    output_price_per_million: 30.00,
                    context_window: 128000,
                    quality_score: 0.95,
                    avg_latency_ms: 2000,
                },
                ModelPricing {
                    model: "gpt-4".to_string(),
                    input_price_per_million: 30.00,
                    output_price_per_million: 60.00,
                    context_window: 8192,
                    quality_score: 0.95,
                    avg_latency_ms: 2500,
                },
                ModelPricing {
                    model: "gpt-3.5-turbo".to_string(),
                    input_price_per_million: 0.50,
                    output_price_per_million: 1.50,
                    context_window: 16385,
                    quality_score: 0.80,
                    avg_latency_ms: 500,
                },
            ],
        );

        // Anthropic pricing
        pricing.insert(
            LlmProvider::Anthropic,
            vec![
                ModelPricing {
                    model: "claude-3-opus-20240229".to_string(),
                    input_price_per_million: 15.00,
                    output_price_per_million: 75.00,
                    context_window: 200000,
                    quality_score: 0.97,
                    avg_latency_ms: 3000,
                },
                ModelPricing {
                    model: "claude-3-sonnet-20240229".to_string(),
                    input_price_per_million: 3.00,
                    output_price_per_million: 15.00,
                    context_window: 200000,
                    quality_score: 0.90,
                    avg_latency_ms: 1500,
                },
                ModelPricing {
                    model: "claude-3-haiku-20240307".to_string(),
                    input_price_per_million: 0.25,
                    output_price_per_million: 1.25,
                    context_window: 200000,
                    quality_score: 0.80,
                    avg_latency_ms: 300,
                },
            ],
        );

        // DeepSeek pricing (very competitive)
        pricing.insert(
            LlmProvider::DeepSeek,
            vec![
                ModelPricing {
                    model: "deepseek-chat".to_string(),
                    input_price_per_million: 0.14,
                    output_price_per_million: 0.28,
                    context_window: 64000,
                    quality_score: 0.88,
                    avg_latency_ms: 800,
                },
                ModelPricing {
                    model: "deepseek-reasoner".to_string(),
                    input_price_per_million: 0.55,
                    output_price_per_million: 2.19,
                    context_window: 64000,
                    quality_score: 0.92,
                    avg_latency_ms: 5000,
                },
            ],
        );

        // Kimi pricing
        pricing.insert(
            LlmProvider::Kimi,
            vec![
                ModelPricing {
                    model: "moonshot-v1-8k".to_string(),
                    input_price_per_million: 12.00,
                    output_price_per_million: 12.00,
                    context_window: 8192,
                    quality_score: 0.82,
                    avg_latency_ms: 1000,
                },
                ModelPricing {
                    model: "moonshot-v1-32k".to_string(),
                    input_price_per_million: 24.00,
                    output_price_per_million: 24.00,
                    context_window: 32768,
                    quality_score: 0.82,
                    avg_latency_ms: 1500,
                },
                ModelPricing {
                    model: "moonshot-v1-128k".to_string(),
                    input_price_per_million: 60.00,
                    output_price_per_million: 60.00,
                    context_window: 131072,
                    quality_score: 0.82,
                    avg_latency_ms: 2500,
                },
            ],
        );

        // Doubao pricing
        pricing.insert(
            LlmProvider::Doubao,
            vec![
                ModelPricing {
                    model: "doubao-pro-32k".to_string(),
                    input_price_per_million: 0.80,
                    output_price_per_million: 2.00,
                    context_window: 32768,
                    quality_score: 0.85,
                    avg_latency_ms: 600,
                },
                ModelPricing {
                    model: "doubao-pro-128k".to_string(),
                    input_price_per_million: 5.00,
                    output_price_per_million: 9.00,
                    context_window: 131072,
                    quality_score: 0.85,
                    avg_latency_ms: 1000,
                },
            ],
        );

        // Cache pricing for providers that support it
        let mut cache_pricing = HashMap::new();

        cache_pricing.insert(
            LlmProvider::Anthropic,
            CachePricing {
                write_price_per_million: 3.75,
                read_price_per_million: 0.30,
                ttl_seconds: 300, // 5 minutes
            },
        );

        cache_pricing.insert(
            LlmProvider::DeepSeek,
            CachePricing {
                write_price_per_million: 0.14,
                read_price_per_million: 0.014,
                ttl_seconds: 300,
            },
        );

        Self {
            pricing,
            cache_pricing,
        }
    }

    /// Calculate cost for a request
    pub fn calculate_cost(
        &self,
        provider: LlmProvider,
        model: &str,
        input_tokens: u64,
        output_tokens: u64,
    ) -> f64 {
        let model_pricing = self.get_model_pricing(provider, model);

        let input_cost =
            (input_tokens as f64 / 1_000_000.0) * model_pricing.input_price_per_million;
        let output_cost =
            (output_tokens as f64 / 1_000_000.0) * model_pricing.output_price_per_million;

        input_cost + output_cost
    }

    /// Calculate cache cost
    pub fn calculate_cache_cost(
        &self,
        provider: LlmProvider,
        cache_tokens: u64,
        is_write: bool,
    ) -> f64 {
        let cache_pricing = match self.cache_pricing.get(&provider) {
            Some(p) => p,
            None => return 0.0, // No cache pricing for this provider
        };

        let price = if is_write {
            cache_pricing.write_price_per_million
        } else {
            cache_pricing.read_price_per_million
        };

        (cache_tokens as f64 / 1_000_000.0) * price
    }

    /// Get pricing for a specific model
    pub fn get_model_pricing(&self, provider: LlmProvider, model: &str) -> ModelPricing {
        self.pricing
            .get(&provider)
            .and_then(|models| {
                models
                    .iter()
                    .find(|m| m.model == model || m.model.starts_with(model))
            })
            .cloned()
            .unwrap_or_else(|| ModelPricing {
                model: model.to_string(),
                input_price_per_million: 1.0,
                output_price_per_million: 2.0,
                context_window: 4096,
                quality_score: 0.5,
                avg_latency_ms: 1000,
            })
    }

    /// Get all models for a provider
    pub fn get_provider_models(&self, provider: LlmProvider) -> Vec<ModelPricing> {
        self.pricing.get(&provider).cloned().unwrap_or_default()
    }

    /// Estimate cost for a task
    pub fn estimate_cost(
        &self,
        provider: LlmProvider,
        model: &str,
        estimated_input_tokens: u64,
        estimated_output_tokens: u64,
    ) -> f64 {
        self.calculate_cost(
            provider,
            model,
            estimated_input_tokens,
            estimated_output_tokens,
        )
    }

    /// Compare costs across providers
    pub fn compare_costs(
        &self,
        input_tokens: u64,
        output_tokens: u64,
    ) -> HashMap<LlmProvider, f64> {
        let mut costs = HashMap::new();

        for provider in [
            LlmProvider::OpenAI,
            LlmProvider::Anthropic,
            LlmProvider::DeepSeek,
            LlmProvider::Kimi,
            LlmProvider::Doubao,
        ] {
            if let Some(models) = self.pricing.get(&provider) {
                // Use cheapest model for each provider
                if let Some(cheapest) = models.iter().min_by(|a, b| {
                    let cost_a = (input_tokens as f64 / 1_000_000.0) * a.input_price_per_million
                        + (output_tokens as f64 / 1_000_000.0) * a.output_price_per_million;
                    let cost_b = (input_tokens as f64 / 1_000_000.0) * b.input_price_per_million
                        + (output_tokens as f64 / 1_000_000.0) * b.output_price_per_million;
                    cost_a.partial_cmp(&cost_b).unwrap()
                }) {
                    let cost = (input_tokens as f64 / 1_000_000.0)
                        * cheapest.input_price_per_million
                        + (output_tokens as f64 / 1_000_000.0) * cheapest.output_price_per_million;
                    costs.insert(provider, cost);
                }
            }
        }

        costs
    }

    /// Get the cheapest provider for given requirements
    pub fn get_cheapest_provider(
        &self,
        input_tokens: u64,
        output_tokens: u64,
        min_quality: f64,
    ) -> Option<(LlmProvider, ModelPricing, f64)> {
        let mut best: Option<(LlmProvider, ModelPricing, f64)> = None;

        for (provider, models) in &self.pricing {
            for model in models {
                if model.quality_score >= min_quality {
                    let cost = (input_tokens as f64 / 1_000_000.0) * model.input_price_per_million
                        + (output_tokens as f64 / 1_000_000.0) * model.output_price_per_million;

                    if best.is_none() || cost < best.as_ref().unwrap().2 {
                        best = Some((*provider, model.clone(), cost));
                    }
                }
            }
        }

        best
    }
}

impl Default for PricingEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_cost() {
        let engine = PricingEngine::new();

        let cost = engine.calculate_cost(LlmProvider::DeepSeek, "deepseek-chat", 1000, 500);

        // 1000 input * 0.14/1M + 500 output * 0.28/1M
        // = 0.00014 + 0.00014 = 0.00028
        assert!(cost > 0.0);
        assert!(cost < 0.001);
    }

    #[test]
    fn test_get_cheapest_provider() {
        let engine = PricingEngine::new();

        let result = engine.get_cheapest_provider(1000, 500, 0.8);
        assert!(result.is_some());

        let (provider, _, cost) = result.unwrap();
        assert_eq!(provider, LlmProvider::DeepSeek);
        assert!(cost < 0.001);
    }

    #[test]
    fn test_compare_costs() {
        let engine = PricingEngine::new();

        let costs = engine.compare_costs(1000, 500);

        assert!(costs.contains_key(&LlmProvider::OpenAI));
        assert!(costs.contains_key(&LlmProvider::DeepSeek));

        // DeepSeek should be cheaper than OpenAI
        assert!(costs[&LlmProvider::DeepSeek] < costs[&LlmProvider::OpenAI]);
    }
}
