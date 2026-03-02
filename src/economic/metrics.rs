//! Economic Metrics Module
//!
//! Provides Prometheus-compatible metrics for token usage and costs

use crate::economic::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

struct AtomicF64 {
    bits: AtomicU64,
}

impl AtomicF64 {
    fn new(value: f64) -> Self {
        Self {
            bits: AtomicU64::new(value.to_bits()),
        }
    }

    fn load(&self, order: Ordering) -> f64 {
        f64::from_bits(self.bits.load(order))
    }

    fn store(&self, value: f64, order: Ordering) {
        self.bits.store(value.to_bits(), order);
    }

    fn fetch_add(&self, value: f64, order: Ordering) -> f64 {
        let mut current_bits = self.bits.load(order);
        loop {
            let current = f64::from_bits(current_bits);
            let next_bits = (current + value).to_bits();
            match self.bits.compare_exchange_weak(
                current_bits,
                next_bits,
                order,
                Ordering::Relaxed,
            ) {
                Ok(_) => return current,
                Err(observed) => current_bits = observed,
            }
        }
    }
}

/// Economic metrics collector
pub struct EconomicMetrics {
    // Counters
    tokens_total: Arc<AtomicU64>,
    cost_total_usd: Arc<AtomicF64>,
    requests_total: Arc<AtomicU64>,
    cache_hits: Arc<AtomicU64>,
    cache_misses: Arc<AtomicU64>,

    // Gauges
    current_hourly_cost: Arc<AtomicF64>,
    current_daily_cost: Arc<AtomicF64>,
    budget_remaining_usd: Arc<AtomicF64>,

    // Histograms (simplified as counters for now)
    latency_sum: Arc<AtomicU64>,
    latency_count: Arc<AtomicU64>,
}

impl EconomicMetrics {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self {
            tokens_total: Arc::new(AtomicU64::new(0)),
            cost_total_usd: Arc::new(AtomicF64::new(0.0)),
            requests_total: Arc::new(AtomicU64::new(0)),
            cache_hits: Arc::new(AtomicU64::new(0)),
            cache_misses: Arc::new(AtomicU64::new(0)),
            current_hourly_cost: Arc::new(AtomicF64::new(0.0)),
            current_daily_cost: Arc::new(AtomicF64::new(0.0)),
            budget_remaining_usd: Arc::new(AtomicF64::new(0.0)),
            latency_sum: Arc::new(AtomicU64::new(0)),
            latency_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Record a token usage event
    pub fn record_usage(&self, usage: &TokenUsage) {
        self.tokens_total
            .fetch_add(usage.total_tokens(), Ordering::Relaxed);
        self.cost_total_usd
            .fetch_add(usage.cost_usd, Ordering::Relaxed);
        self.requests_total.fetch_add(1, Ordering::Relaxed);

        if usage.cached {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
        }

        self.latency_sum
            .fetch_add(usage.latency_ms, Ordering::Relaxed);
        self.latency_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Update current cost gauges
    pub fn update_costs(&self, hourly: f64, daily: f64, remaining: f64) {
        self.current_hourly_cost.store(hourly, Ordering::Relaxed);
        self.current_daily_cost.store(daily, Ordering::Relaxed);
        self.budget_remaining_usd
            .store(remaining, Ordering::Relaxed);
    }

    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let tokens = self.tokens_total.load(Ordering::Relaxed);
        let cost = self.cost_total_usd.load(Ordering::Relaxed);
        let requests = self.requests_total.load(Ordering::Relaxed);
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let hourly_cost = self.current_hourly_cost.load(Ordering::Relaxed);
        let daily_cost = self.current_daily_cost.load(Ordering::Relaxed);
        let remaining = self.budget_remaining_usd.load(Ordering::Relaxed);
        let latency_sum = self.latency_sum.load(Ordering::Relaxed);
        let latency_count = self.latency_count.load(Ordering::Relaxed);

        let avg_latency = if latency_count > 0 {
            latency_sum as f64 / latency_count as f64
        } else {
            0.0
        };

        let cache_hit_rate = if hits + misses > 0 {
            hits as f64 / (hits + misses) as f64 * 100.0
        } else {
            0.0
        };

        format!(
            r#"# HELP servant_guild_tokens_total Total tokens used
# TYPE servant_guild_tokens_total counter
servant_guild_tokens_total {}

# HELP servant_guild_cost_usd_total Total cost in USD
# TYPE servant_guild_cost_usd_total counter
servant_guild_cost_usd_total {:.6}

# HELP servant_guild_requests_total Total API requests
# TYPE servant_guild_requests_total counter
servant_guild_requests_total {}

# HELP servant_guild_cache_hits_total Total cache hits
# TYPE servant_guild_cache_hits_total counter
servant_guild_cache_hits_total {}

# HELP servant_guild_cache_misses_total Total cache misses
# TYPE servant_guild_cache_misses_total counter
servant_guild_cache_misses_total {}

# HELP servant_guild_cache_hit_rate_percent Cache hit rate percentage
# TYPE servant_guild_cache_hit_rate_percent gauge
servant_guild_cache_hit_rate_percent {:.2}

# HELP servant_guild_hourly_cost_usd Current hourly cost in USD
# TYPE servant_guild_hourly_cost_usd gauge
servant_guild_hourly_cost_usd {:.6}

# HELP servant_guild_daily_cost_usd Current daily cost in USD
# TYPE servant_guild_daily_cost_usd gauge
servant_guild_daily_cost_usd {:.6}

# HELP servant_guild_budget_remaining_usd Remaining budget in USD
# TYPE servant_guild_budget_remaining_usd gauge
servant_guild_budget_remaining_usd {:.6}

# HELP servant_guild_avg_latency_ms Average request latency in milliseconds
# TYPE servant_guild_avg_latency_ms gauge
servant_guild_avg_latency_ms {:.2}
"#,
            tokens,
            cost,
            requests,
            hits,
            misses,
            cache_hit_rate,
            hourly_cost,
            daily_cost,
            remaining,
            avg_latency
        )
    }

    /// Get metrics as a structured format
    pub fn get_metrics(&self) -> EconomicMetricValues {
        EconomicMetricValues {
            tokens_total: self.tokens_total.load(Ordering::Relaxed),
            cost_total_usd: self.cost_total_usd.load(Ordering::Relaxed),
            requests_total: self.requests_total.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
            current_hourly_cost: self.current_hourly_cost.load(Ordering::Relaxed),
            current_daily_cost: self.current_daily_cost.load(Ordering::Relaxed),
            budget_remaining_usd: self.budget_remaining_usd.load(Ordering::Relaxed),
        }
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.tokens_total.store(0, Ordering::Relaxed);
        self.cost_total_usd.store(0.0, Ordering::Relaxed);
        self.requests_total.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
        self.current_hourly_cost.store(0.0, Ordering::Relaxed);
        self.current_daily_cost.store(0.0, Ordering::Relaxed);
        self.budget_remaining_usd.store(0.0, Ordering::Relaxed);
        self.latency_sum.store(0, Ordering::Relaxed);
        self.latency_count.store(0, Ordering::Relaxed);
    }
}

impl Default for EconomicMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of metric values
#[derive(Debug, Clone, serde::Serialize)]
pub struct EconomicMetricValues {
    pub tokens_total: u64,
    pub cost_total_usd: f64,
    pub requests_total: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub current_hourly_cost: f64,
    pub current_daily_cost: f64,
    pub budget_remaining_usd: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_usage() {
        let metrics = EconomicMetrics::new();

        let mut usage = TokenUsage::new(
            "test".to_string(),
            LlmProvider::DeepSeek,
            "deepseek-chat".to_string(),
        );
        usage.add_tokens(TokenType::Input, 1000);
        usage.add_tokens(TokenType::Output, 500);
        usage.cost_usd = 0.01;
        usage.latency_ms = 500;

        metrics.record_usage(&usage);

        let values = metrics.get_metrics();
        assert_eq!(values.tokens_total, 1500);
        assert_eq!(values.cost_total_usd, 0.01);
        assert_eq!(values.requests_total, 1);
    }

    #[test]
    fn test_prometheus_export() {
        let metrics = EconomicMetrics::new();
        metrics.update_costs(1.0, 5.0, 45.0);

        let output = metrics.export_prometheus();

        assert!(output.contains("servant_guild_tokens_total"));
        assert!(output.contains("servant_guild_cost_usd_total"));
        assert!(output.contains("servant_guild_hourly_cost_usd 1.000000"));
    }
}
