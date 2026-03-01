//! Token Tracking Module
//!
//! Records and aggregates token usage across the system

use crate::economic::*;
use chrono::{DateTime, Datelike, Duration, Timelike, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Token tracker - records and analyzes token usage
pub struct TokenTracker {
    usage_log: Arc<RwLock<Vec<TokenUsage>>>,
    aggregator: Arc<RwLock<UsageAggregator>>,
}

/// Usage aggregator for quick statistics
struct UsageAggregator {
    /// Total tokens by time bucket
    hourly_tokens: HashMap<(String, u32), u64>,
    daily_tokens: HashMap<String, u64>,
    /// Total cost by time bucket
    hourly_cost: HashMap<(String, u32), f64>,
    daily_cost: HashMap<String, f64>,
    /// Usage by provider
    provider_usage: HashMap<LlmProvider, ProviderAccumulator>,
    /// Usage by agent
    agent_usage: HashMap<String, AgentAccumulator>,
    /// Cache hits
    cache_hits: u64,
    cache_misses: u64,
}

#[derive(Default)]
struct ProviderAccumulator {
    tokens: u64,
    cost: f64,
    requests: u64,
    latency_sum: u64,
}

#[derive(Default)]
struct AgentAccumulator {
    tokens: u64,
    cost: f64,
    requests: u64,
}

impl UsageAggregator {
    fn new() -> Self {
        Self {
            hourly_tokens: HashMap::new(),
            daily_tokens: HashMap::new(),
            hourly_cost: HashMap::new(),
            daily_cost: HashMap::new(),
            provider_usage: HashMap::new(),
            agent_usage: HashMap::new(),
            cache_hits: 0,
            cache_misses: 0,
        }
    }
}

impl TokenTracker {
    /// Create new token tracker
    pub fn new() -> Self {
        Self {
            usage_log: Arc::new(RwLock::new(Vec::new())),
            aggregator: Arc::new(RwLock::new(UsageAggregator::new())),
        }
    }

    /// Record token usage
    pub async fn record(&self, usage: TokenUsage) {
        let timestamp = usage.timestamp;
        let date = timestamp.format("%Y-%m-%d").to_string();
        let hour = timestamp.hour();
        let total_tokens = usage.total_tokens();
        let cost = usage.cost_usd;
        let provider = usage.provider;
        let agent = usage.agent.clone();
        let cached = usage.cached;

        // Update aggregator
        {
            let mut agg = self.aggregator.write().await;

            // Update time buckets
            *agg.hourly_tokens.entry((date.clone(), hour)).or_insert(0) += total_tokens;
            *agg.daily_tokens.entry(date.clone()).or_insert(0) += total_tokens;
            *agg.hourly_cost.entry((date.clone(), hour)).or_insert(0.0) += cost;
            *agg.daily_cost.entry(date).or_insert(0.0) += cost;

            // Update provider usage
            let provider_agg = agg.provider_usage.entry(provider).or_default();
            provider_agg.tokens += total_tokens;
            provider_agg.cost += cost;
            provider_agg.requests += 1;
            provider_agg.latency_sum += usage.latency_ms;

            // Update agent usage
            let agent_agg = agg.agent_usage.entry(agent).or_default();
            agent_agg.tokens += total_tokens;
            agent_agg.cost += cost;
            agent_agg.requests += 1;

            // Update cache stats
            if cached {
                agg.cache_hits += 1;
            } else {
                agg.cache_misses += 1;
            }
        }

        // Store usage record
        {
            let mut log = self.usage_log.write().await;
            log.push(usage);

            // Keep only last 10000 records
            if log.len() > 10000 {
                log.drain(0..log.len() - 10000);
            }
        }
    }

    /// Get statistics for a time period
    pub async fn stats(&self, period: TimePeriod) -> UsageStats {
        let agg = self.aggregator.read().await;
        let now = Utc::now();

        let (period_name, start_date) = match period {
            TimePeriod::Hour => {
                let date = now.format("%Y-%m-%d").to_string();
                let hour = now.hour();
                let tokens = agg
                    .hourly_tokens
                    .get(&(date.clone(), hour))
                    .copied()
                    .unwrap_or(0);
                let cost = agg.hourly_cost.get(&(date, hour)).copied().unwrap_or(0.0);
                return UsageStats {
                    period: format!("hour-{}", hour),
                    total_tokens: tokens,
                    total_cost_usd: cost,
                    request_count: 0,
                    avg_tokens_per_request: 0.0,
                    avg_cost_per_request: 0.0,
                    cache_hit_rate: 0.0,
                    by_provider: HashMap::new(),
                    by_agent: HashMap::new(),
                };
            }
            TimePeriod::Day => ("day".to_string(), now.format("%Y-%m-%d").to_string()),
            TimePeriod::Week => {
                let start = now - Duration::days(7);
                ("week".to_string(), start.format("%Y-%m-%d").to_string())
            }
            TimePeriod::Month => {
                let start = now - Duration::days(30);
                ("month".to_string(), start.format("%Y-%m-%d").to_string())
            }
        };

        // Aggregate daily stats
        let total_tokens = agg.daily_tokens.values().sum();
        let total_cost = agg.daily_cost.values().sum();
        let request_count: u64 = agg.provider_usage.values().map(|p| p.requests).sum();

        let avg_tokens_per_request = if request_count > 0 {
            total_tokens as f64 / request_count as f64
        } else {
            0.0
        };

        let avg_cost_per_request = if request_count > 0 {
            total_cost / request_count as f64
        } else {
            0.0
        };

        let cache_hit_rate = if agg.cache_hits + agg.cache_misses > 0 {
            agg.cache_hits as f64 / (agg.cache_hits + agg.cache_misses) as f64
        } else {
            0.0
        };

        // Provider stats
        let by_provider: HashMap<String, ProviderStats> = agg
            .provider_usage
            .iter()
            .map(|(provider, acc)| {
                (
                    provider.name().to_string(),
                    ProviderStats {
                        name: provider.name().to_string(),
                        tokens: acc.tokens,
                        cost_usd: acc.cost,
                        requests: acc.requests,
                        avg_latency_ms: if acc.requests > 0 {
                            acc.latency_sum as f64 / acc.requests as f64
                        } else {
                            0.0
                        },
                    },
                )
            })
            .collect();

        // Agent stats
        let by_agent: HashMap<String, AgentStats> = agg
            .agent_usage
            .iter()
            .map(|(agent, acc)| {
                (
                    agent.clone(),
                    AgentStats {
                        name: agent.clone(),
                        tokens: acc.tokens,
                        cost_usd: acc.cost,
                        requests: acc.requests,
                    },
                )
            })
            .collect();

        UsageStats {
            period: period_name,
            total_tokens,
            total_cost_usd: total_cost,
            request_count,
            avg_tokens_per_request,
            avg_cost_per_request,
            cache_hit_rate,
            by_provider,
            by_agent,
        }
    }

    /// Get recent usage records
    pub async fn recent_usage(&self, limit: usize) -> Vec<TokenUsage> {
        let log = self.usage_log.read().await;
        log.iter().rev().take(limit).cloned().collect()
    }

    /// Get usage for specific agent
    pub async fn agent_usage(&self, agent: &str) -> Option<AgentStats> {
        let agg = self.aggregator.read().await;
        agg.agent_usage.get(agent).map(|acc| AgentStats {
            name: agent.to_string(),
            tokens: acc.tokens,
            cost_usd: acc.cost,
            requests: acc.requests,
        })
    }

    /// Get usage for specific provider
    pub async fn provider_usage(&self, provider: LlmProvider) -> Option<ProviderStats> {
        let agg = self.aggregator.read().await;
        agg.provider_usage.get(&provider).map(|acc| ProviderStats {
            name: provider.name().to_string(),
            tokens: acc.tokens,
            cost_usd: acc.cost,
            requests: acc.requests,
            avg_latency_ms: if acc.requests > 0 {
                acc.latency_sum as f64 / acc.requests as f64
            } else {
                0.0
            },
        })
    }

    /// Clear all records
    pub async fn clear(&self) {
        let mut log = self.usage_log.write().await;
        log.clear();

        let mut agg = self.aggregator.write().await;
        *agg = UsageAggregator::new();
    }
}

impl Default for TokenTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_usage() {
        let tracker = TokenTracker::new();

        let mut usage = TokenUsage::new(
            "coordinator".to_string(),
            LlmProvider::DeepSeek,
            "deepseek-chat".to_string(),
        );
        usage.add_tokens(TokenType::Input, 1000);
        usage.add_tokens(TokenType::Output, 500);
        usage.cost_usd = 0.01;

        tracker.record(usage).await;

        let stats = tracker.stats(TimePeriod::Day).await;
        assert_eq!(stats.total_tokens, 1500);
        assert_eq!(stats.total_cost_usd, 0.01);
    }

    #[tokio::test]
    async fn test_agent_usage() {
        let tracker = TokenTracker::new();

        let mut usage = TokenUsage::new(
            "worker".to_string(),
            LlmProvider::DeepSeek,
            "deepseek-chat".to_string(),
        );
        usage.add_tokens(TokenType::Input, 500);
        usage.add_tokens(TokenType::Output, 250);
        usage.cost_usd = 0.005;

        tracker.record(usage).await;

        let agent_stats = tracker.agent_usage("worker").await;
        assert!(agent_stats.is_some());
        assert_eq!(agent_stats.unwrap().tokens, 750);
    }
}
