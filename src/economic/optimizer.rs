//! Token Optimization Module
//!
//! Provides strategies for reducing token usage and costs

use crate::economic::*;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Token optimizer - implements cost reduction strategies
pub struct TokenOptimizer {
    config: OptimizationStrategy,
    compression_rules: Vec<CompressionRule>,
}

/// Compression rule for prompt optimization
#[derive(Debug, Clone)]
pub struct CompressionRule {
    /// Rule name
    pub name: String,
    /// Pattern to match
    pub pattern: String,
    /// Replacement
    pub replacement: String,
    /// Estimated token savings percentage
    pub savings_percent: f64,
}

impl TokenOptimizer {
    /// Create new token optimizer
    pub fn new(config: OptimizationStrategy) -> Self {
        let compression_rules = Self::default_compression_rules();

        Self {
            config,
            compression_rules,
        }
    }

    /// Get default compression rules
    fn default_compression_rules() -> Vec<CompressionRule> {
        vec![
            CompressionRule {
                name: "remove_redundant_whitespace".to_string(),
                pattern: r"\s+".to_string(),
                replacement: " ".to_string(),
                savings_percent: 5.0,
            },
            CompressionRule {
                name: "shorten_common_phrases".to_string(),
                pattern: r"please\s+".to_string(),
                replacement: "".to_string(),
                savings_percent: 3.0,
            },
            CompressionRule {
                name: "remove_please_thank".to_string(),
                pattern: r"(?i)(please|thank you|thanks)\s*".to_string(),
                replacement: "".to_string(),
                savings_percent: 2.0,
            },
        ]
    }

    /// Optimize a prompt
    pub fn optimize_prompt(&self, prompt: &str) -> OptimizedPrompt {
        let mut optimized = prompt.to_string();
        let mut savings = 0.0;

        if self.config.enable_compression {
            for rule in &self.compression_rules {
                let before_len = optimized.len();
                optimized = optimized.replace(&rule.pattern, &rule.replacement);
                if before_len > 0 {
                    savings += (before_len - optimized.len()) as f64 / before_len as f64
                        * rule.savings_percent
                        / 100.0;
                }
            }
        }

        OptimizedPrompt {
            original: prompt.to_string(),
            optimized,
            estimated_savings_percent: savings * 100.0,
            original_tokens: Self::estimate_tokens(prompt),
            optimized_tokens: Self::estimate_tokens(&optimized),
        }
    }

    /// Estimate token count for text
    fn estimate_tokens(text: &str) -> u64 {
        // Rough estimation: ~4 characters per token for English
        (text.len() / 4) as u64
    }

    /// Select optimal provider based on requirements
    pub fn select_provider(
        &self,
        requirements: &ProviderRequirements,
        priorities: &HashMap<LlmProvider, u8>,
    ) -> LlmProvider {
        // If quality is critical, prefer high-quality providers
        if requirements.min_quality >= 0.9 {
            return LlmProvider::Anthropic;
        }

        // If caching is needed, prefer providers that support it
        if requirements.needs_caching {
            return LlmProvider::DeepSeek;
        }

        // Otherwise, use priority-based selection
        priorities
            .iter()
            .filter(|(_, priority)| **priority > 0)
            .max_by_key(|(_, priority)| *priority)
            .map(|(provider, _)| *provider)
            .unwrap_or(LlmProvider::DeepSeek)
    }

    /// Generate optimization recommendations
    pub async fn recommendations(&self, stats: &UsageStats) -> Vec<OptimizationRecommendation> {
        let mut recs = Vec::new();

        // Check cache hit rate
        if stats.cache_hit_rate < 0.3 {
            recs.push(OptimizationRecommendation {
                recommendation_type: RecommendationType::EnableCaching,
                description:
                    "Enable prompt caching to reduce API costs. Current cache hit rate is low."
                        .to_string(),
                estimated_savings_usd: stats.total_cost_usd * 0.2,
                priority: 5,
            });
        }

        // Check for expensive provider usage
        if let Some(openai_stats) = stats.by_provider.get("openai") {
            if openai_stats.cost_usd > stats.total_cost_usd * 0.5 {
                recs.push(OptimizationRecommendation {
                    recommendation_type: RecommendationType::SwitchProvider,
                    description: "Consider switching from OpenAI to DeepSeek for cost savings."
                        .to_string(),
                    estimated_savings_usd: openai_stats.cost_usd * 0.8, // DeepSeek is ~80% cheaper
                    priority: 4,
                });
            }
        }

        // Check average tokens per request
        if stats.avg_tokens_per_request > 2000.0 {
            recs.push(OptimizationRecommendation {
                recommendation_type: RecommendationType::CompressPrompts,
                description: "Average tokens per request is high. Consider prompt compression."
                    .to_string(),
                estimated_savings_usd: stats.total_cost_usd * 0.15,
                priority: 3,
            });
        }

        // Check request frequency
        if stats.request_count > 1000 {
            recs.push(OptimizationRecommendation {
                recommendation_type: RecommendationType::BatchRequests,
                description: "High request count. Consider batching to reduce overhead."
                    .to_string(),
                estimated_savings_usd: stats.total_cost_usd * 0.1,
                priority: 2,
            });
        }

        recs.sort_by(|a, b| b.priority.cmp(&a.priority));
        recs
    }
}

/// Optimized prompt result
#[derive(Debug, Clone)]
pub struct OptimizedPrompt {
    /// Original prompt
    pub original: String,
    /// Optimized prompt
    pub optimized: String,
    /// Estimated savings percentage
    pub estimated_savings_percent: f64,
    /// Original token count estimate
    pub original_tokens: u64,
    /// Optimized token count estimate
    pub optimized_tokens: u64,
}

impl OptimizedPrompt {
    /// Get token savings
    pub fn token_savings(&self) -> u64 {
        self.original_tokens.saturating_sub(self.optimized_tokens)
    }
}

/// Token cache for response reuse
pub struct TokenCache {
    ttl_seconds: u64,
    cache: std::sync::Arc<tokio::sync::RwLock<HashMap<String, CacheEntry>>>,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    response: String,
    created_at: std::time::Instant,
    hits: u64,
}

impl TokenCache {
    /// Create new token cache
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            ttl_seconds,
            cache: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Compute hash for a prompt
    pub fn hash_prompt(prompt: &str, model: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(prompt.as_bytes());
        hasher.update(model.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Get cached response
    pub async fn get(&self, key: &str) -> Option<String> {
        let mut cache = self.cache.write().await;

        if let Some(entry) = cache.get_mut(key) {
            if entry.created_at.elapsed().as_secs() < self.ttl_seconds {
                entry.hits += 1;
                return Some(entry.response.clone());
            } else {
                cache.remove(key);
            }
        }

        None
    }

    /// Store response in cache
    pub async fn set(&self, key: &str, response: &str) {
        let mut cache = self.cache.write().await;

        cache.insert(
            key.to_string(),
            CacheEntry {
                response: response.to_string(),
                created_at: std::time::Instant::now(),
                hits: 0,
            },
        );

        // Evict old entries if cache is too large
        if cache.len() > 10000 {
            let now = std::time::Instant::now();
            cache.retain(|_, entry| {
                now.duration_since(entry.created_at).as_secs() < self.ttl_seconds
            });
        }
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        let total_entries = cache.len();
        let total_hits: u64 = cache.values().map(|e| e.hits).sum();

        CacheStats {
            entries: total_entries,
            hits: total_hits,
            ttl_seconds: self.ttl_seconds,
        }
    }

    /// Clear cache
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub hits: u64,
    pub ttl_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimize_prompt() {
        let optimizer = TokenOptimizer::new(OptimizationStrategy::default());

        let prompt = "Please help me with this task. Thank you very much!";
        let optimized = optimizer.optimize_prompt(prompt);

        // Should be shorter after optimization
        assert!(optimized.optimized.len() <= prompt.len());
    }

    #[test]
    fn test_select_provider() {
        let optimizer = TokenOptimizer::new(OptimizationStrategy::default());
        let mut priorities = HashMap::new();
        priorities.insert(LlmProvider::DeepSeek, 5);
        priorities.insert(LlmProvider::OpenAI, 1);

        let requirements = ProviderRequirements {
            min_quality: 0.8,
            max_latency_ms: 5000,
            capabilities: vec![],
            context_length: 8000,
            needs_caching: false,
        };

        let provider = optimizer.select_provider(&requirements, &priorities);
        assert_eq!(provider, LlmProvider::DeepSeek);
    }

    #[tokio::test]
    async fn test_token_cache() {
        let cache = TokenCache::new(3600);
        let key = TokenCache::hash_prompt("test prompt", "gpt-4");

        cache.set(&key, "test response").await;

        let result = cache.get(&key).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "test response");
    }
}
