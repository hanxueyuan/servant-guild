//! Economic Model - Token Optimization and Cost Monitoring
//!
//! This module provides:
//! - Token usage tracking and optimization
//! - API cost monitoring and budgeting
//! - Rate limiting and throttling
//! - Cost-aware LLM provider selection

pub mod budget;
pub mod cache;
pub mod metrics;
pub mod optimizer;
pub mod pricing;
pub mod provider;
pub mod tracker;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};

pub use budget::BudgetManager;
pub use cache::TokenCache;
pub use metrics::EconomicMetrics;
pub use optimizer::TokenOptimizer;
pub use pricing::PricingEngine;
pub use provider::ProviderSelector;
pub use tracker::TokenTracker;

/// Token types for tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TokenType {
    /// Input/prompt tokens
    Input,
    /// Output/completion tokens
    Output,
    /// Cached tokens (reused from previous calls)
    Cached,
}

/// LLM Provider identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LlmProvider {
    OpenAI,
    Anthropic,
    DeepSeek,
    Kimi,
    Doubao,
}

impl LlmProvider {
    /// Get provider name
    pub fn name(&self) -> &'static str {
        match self {
            Self::OpenAI => "openai",
            Self::Anthropic => "anthropic",
            Self::DeepSeek => "deepseek",
            Self::Kimi => "kimi",
            Self::Doubao => "doubao",
        }
    }
    
    /// Check if provider supports caching
    pub fn supports_caching(&self) -> bool {
        matches!(self, Self::Anthropic | Self::DeepSeek)
    }
}

/// Token usage record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Unique usage ID
    pub id: uuid::Uuid,
    /// Agent that made the request
    pub agent: String,
    /// Task ID (if applicable)
    pub task_id: Option<uuid::Uuid>,
    /// LLM provider used
    pub provider: LlmProvider,
    /// Model identifier
    pub model: String,
    /// Token counts by type
    pub tokens: HashMap<TokenType, u64>,
    /// Cost in USD
    pub cost_usd: f64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Request latency in milliseconds
    pub latency_ms: u64,
    /// Whether request was cached
    pub cached: bool,
    /// Request metadata
    pub metadata: HashMap<String, String>,
}

impl TokenUsage {
    /// Create new token usage record
    pub fn new(
        agent: String,
        provider: LlmProvider,
        model: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            agent,
            task_id: None,
            provider,
            model,
            tokens: HashMap::new(),
            cost_usd: 0.0,
            timestamp: Utc::now(),
            latency_ms: 0,
            cached: false,
            metadata: HashMap::new(),
        }
    }
    
    /// Add token count
    pub fn add_tokens(&mut self, token_type: TokenType, count: u64) {
        *self.tokens.entry(token_type).or_insert(0) += count;
    }
    
    /// Get total tokens
    pub fn total_tokens(&self) -> u64 {
        self.tokens.values().sum()
    }
    
    /// Calculate and set cost
    pub fn calculate_cost(&mut self, pricing: &PricingEngine) {
        self.cost_usd = pricing.calculate_cost(
            self.provider,
            &self.model,
            self.tokens.get(&TokenType::Input).copied().unwrap_or(0),
            self.tokens.get(&TokenType::Output).copied().unwrap_or(0),
        );
    }
}

/// Budget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetConfig {
    /// Daily budget limit in USD
    pub daily_limit_usd: f64,
    /// Hourly budget limit in USD
    pub hourly_limit_usd: f64,
    /// Per-agent budget limit in USD
    pub per_agent_limit_usd: f64,
    /// Per-task budget limit in USD
    pub per_task_limit_usd: f64,
    /// Warning threshold (percentage of budget)
    pub warning_threshold: f64,
    /// Critical threshold (percentage of budget)
    pub critical_threshold: f64,
    /// Enable auto-throttling when budget is exceeded
    pub auto_throttle: bool,
    /// Throttle factor when budget is exceeded (0.0-1.0)
    pub throttle_factor: f64,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            daily_limit_usd: 50.0,
            hourly_limit_usd: 10.0,
            per_agent_limit_usd: 20.0,
            per_task_limit_usd: 2.0,
            warning_threshold: 0.7,
            critical_threshold: 0.9,
            auto_throttle: true,
            throttle_factor: 0.5,
        }
    }
}

/// Cost optimization strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationStrategy {
    /// Enable prompt caching
    pub enable_caching: bool,
    /// Cache TTL in seconds
    pub cache_ttl_secs: u64,
    /// Enable provider auto-selection
    pub auto_provider_selection: bool,
    /// Prefer cheaper providers when quality is acceptable
    pub prefer_cheaper: bool,
    /// Quality threshold for provider selection (0.0-1.0)
    pub quality_threshold: f64,
    /// Enable token compression
    pub enable_compression: bool,
    /// Compression target ratio
    pub compression_target: f64,
    /// Enable request batching
    pub enable_batching: bool,
    /// Minimum batch size
    pub min_batch_size: usize,
}

impl Default for OptimizationStrategy {
    fn default() -> Self {
        Self {
            enable_caching: true,
            cache_ttl_secs: 3600,
            auto_provider_selection: true,
            prefer_cheaper: true,
            quality_threshold: 0.8,
            enable_compression: true,
            compression_target: 0.7,
            enable_batching: true,
            min_batch_size: 3,
        }
    }
}

/// Economic model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicConfig {
    /// Budget configuration
    pub budget: BudgetConfig,
    /// Optimization strategy
    pub optimization: OptimizationStrategy,
    /// Provider priorities (higher = preferred)
    pub provider_priorities: HashMap<LlmProvider, u8>,
    /// Model aliases (short name -> full name)
    pub model_aliases: HashMap<String, String>,
}

impl Default for EconomicConfig {
    fn default() -> Self {
        let mut provider_priorities = HashMap::new();
        provider_priorities.insert(LlmProvider::DeepSeek, 5);  // Cheapest, good quality
        provider_priorities.insert(LlmProvider::Doubao, 4);
        provider_priorities.insert(LlmProvider::Kimi, 3);
        provider_priorities.insert(LlmProvider::Anthropic, 2);
        provider_priorities.insert(LlmProvider::OpenAI, 1);    // Most expensive
        
        let mut model_aliases = HashMap::new();
        model_aliases.insert("gpt-4".to_string(), "gpt-4-turbo-preview".to_string());
        model_aliases.insert("claude-3".to_string(), "claude-3-sonnet-20240229".to_string());
        
        Self {
            budget: BudgetConfig::default(),
            optimization: OptimizationStrategy::default(),
            provider_priorities,
            model_aliases,
        }
    }
}

/// Economic model manager
pub struct EconomicModel {
    config: EconomicConfig,
    tracker: Arc<TokenTracker>,
    budget_manager: Arc<BudgetManager>,
    optimizer: Arc<TokenOptimizer>,
    cache: Arc<TokenCache>,
    metrics: Arc<EconomicMetrics>,
}

impl EconomicModel {
    /// Create new economic model manager
    pub fn new(config: EconomicConfig) -> Self {
        let tracker = Arc::new(TokenTracker::new());
        let budget_manager = Arc::new(BudgetManager::new(config.budget.clone()));
        let optimizer = Arc::new(TokenOptimizer::new(config.optimization.clone()));
        let cache = Arc::new(TokenCache::new(config.optimization.cache_ttl_secs));
        let metrics = Arc::new(EconomicMetrics::new());
        
        Self {
            config,
            tracker,
            budget_manager,
            optimizer,
            cache,
            metrics,
        }
    }
    
    /// Record token usage
    pub async fn record_usage(&self, usage: TokenUsage) -> Result<(), String> {
        // Check budget
        if !self.budget_manager.can_spend(usage.cost_usd, &usage.agent).await {
            return Err("Budget limit exceeded".to_string());
        }
        
        // Record usage
        self.tracker.record(usage.clone()).await;
        
        // Update budget
        self.budget_manager.spend(usage.cost_usd, &usage.agent).await;
        
        // Update metrics
        self.metrics.record_usage(&usage);
        
        Ok(())
    }
    
    /// Check if request can be made
    pub async fn can_make_request(&self, estimated_cost: f64, agent: &str) -> bool {
        self.budget_manager.can_spend(estimated_cost, agent).await
    }
    
    /// Get optimal provider for request
    pub fn get_optimal_provider(&self, requirements: &ProviderRequirements) -> LlmProvider {
        self.optimizer.select_provider(requirements, &self.config.provider_priorities)
    }
    
    /// Check cache for similar request
    pub async fn check_cache(&self, prompt_hash: &str) -> Option<String> {
        self.cache.get(prompt_hash).await
    }
    
    /// Store response in cache
    pub async fn cache_response(&self, prompt_hash: &str, response: &str) {
        self.cache.set(prompt_hash, response).await;
    }
    
    /// Get current budget status
    pub async fn budget_status(&self) -> BudgetStatus {
        self.budget_manager.status().await
    }
    
    /// Get usage statistics
    pub async fn usage_stats(&self, period: TimePeriod) -> UsageStats {
        self.tracker.stats(period).await
    }
    
    /// Get optimization recommendations
    pub async fn optimization_recommendations(&self) -> Vec<OptimizationRecommendation> {
        self.optimizer.recommendations(&self.tracker.stats(TimePeriod::Day).await).await
    }
}

/// Provider selection requirements
#[derive(Debug, Clone)]
pub struct ProviderRequirements {
    /// Minimum quality score required
    pub min_quality: f64,
    /// Maximum latency acceptable (ms)
    pub max_latency_ms: u64,
    /// Required capabilities
    pub capabilities: Vec<String>,
    /// Preferred context length
    pub context_length: usize,
    /// Whether caching is needed
    pub needs_caching: bool,
}

/// Budget status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetStatus {
    /// Daily spent
    pub daily_spent: f64,
    /// Daily limit
    pub daily_limit: f64,
    /// Daily percentage used
    pub daily_percentage: f64,
    /// Hourly spent
    pub hourly_spent: f64,
    /// Hourly limit
    pub hourly_limit: f64,
    /// Whether budget is exceeded
    pub exceeded: bool,
    /// Whether budget is in warning state
    pub warning: bool,
    /// Current throttle factor
    pub throttle_factor: f64,
}

/// Time period for statistics
#[derive(Debug, Clone, Copy)]
pub enum TimePeriod {
    Hour,
    Day,
    Week,
    Month,
}

/// Usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    /// Period
    pub period: String,
    /// Total tokens
    pub total_tokens: u64,
    /// Total cost
    pub total_cost_usd: f64,
    /// Requests count
    pub request_count: u64,
    /// Average tokens per request
    pub avg_tokens_per_request: f64,
    /// Average cost per request
    pub avg_cost_per_request: f64,
    /// Cache hit rate
    pub cache_hit_rate: f64,
    /// Usage by provider
    pub by_provider: HashMap<String, ProviderStats>,
    /// Usage by agent
    pub by_agent: HashMap<String, AgentStats>,
}

/// Statistics by provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStats {
    /// Provider name
    pub name: String,
    /// Total tokens
    pub tokens: u64,
    /// Total cost
    pub cost_usd: f64,
    /// Request count
    pub requests: u64,
    /// Average latency
    pub avg_latency_ms: f64,
}

/// Statistics by agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStats {
    /// Agent name
    pub name: String,
    /// Total tokens
    pub tokens: u64,
    /// Total cost
    pub cost_usd: f64,
    /// Request count
    pub requests: u64,
}

/// Optimization recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecommendation {
    /// Recommendation type
    pub recommendation_type: RecommendationType,
    /// Description
    pub description: String,
    /// Estimated savings (USD)
    pub estimated_savings_usd: f64,
    /// Priority (1-5, higher = more important)
    pub priority: u8,
}

/// Recommendation type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationType {
    EnableCaching,
    SwitchProvider,
    CompressPrompts,
    BatchRequests,
    ReduceFrequency,
    AdjustBudget,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_token_usage() {
        let mut usage = TokenUsage::new(
            "coordinator".to_string(),
            LlmProvider::DeepSeek,
            "deepseek-chat".to_string(),
        );
        
        usage.add_tokens(TokenType::Input, 1000);
        usage.add_tokens(TokenType::Output, 500);
        
        assert_eq!(usage.total_tokens(), 1500);
    }
    
    #[test]
    fn test_budget_config_default() {
        let config = BudgetConfig::default();
        
        assert_eq!(config.daily_limit_usd, 50.0);
        assert_eq!(config.warning_threshold, 0.7);
        assert!(config.auto_throttle);
    }
    
    #[test]
    fn test_llm_provider_caching() {
        assert!(LlmProvider::Anthropic.supports_caching());
        assert!(LlmProvider::DeepSeek.supports_caching());
        assert!(!LlmProvider::OpenAI.supports_caching());
    }
}
