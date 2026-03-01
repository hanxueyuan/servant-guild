//! Canary testing module
//!
//! This module provides canary deployment testing capabilities for ServantGuild,
//! enabling safe rollout of new versions with automated rollback on anomaly detection.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Canary configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryConfig {
    /// Initial traffic percentage for canary
    pub initial_weight: u32,
    /// Maximum traffic percentage for canary
    pub max_weight: u32,
    /// Increment step for traffic percentage
    pub increment_step: u32,
    /// Duration between each increment
    pub increment_interval: Duration,
    /// Anomaly detection thresholds
    pub thresholds: AnomalyThreshold,
    /// Enable automatic rollback on anomaly
    pub auto_rollback: bool,
}

impl Default for CanaryConfig {
    fn default() -> Self {
        Self {
            initial_weight: 5,
            max_weight: 50,
            increment_step: 5,
            increment_interval: Duration::from_secs(300), // 5 minutes
            thresholds: AnomalyThreshold::default(),
            auto_rollback: true,
        }
    }
}

/// Anomaly detection thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyThreshold {
    /// Maximum acceptable error rate increase (percentage points)
    pub error_rate_increase: f64,
    /// Maximum acceptable latency increase (percentage)
    pub latency_increase_percent: f64,
    /// Minimum sample size for statistical significance
    pub min_sample_size: u64,
    /// P-value threshold for anomaly detection
    pub p_value_threshold: f64,
}

impl Default for AnomalyThreshold {
    fn default() -> Self {
        Self {
            error_rate_increase: 5.0,
            latency_increase_percent: 20.0,
            min_sample_size: 100,
            p_value_threshold: 0.05,
        }
    }
}

/// Canary deployment phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CanaryPhase {
    /// Canary not started
    NotStarted,
    /// Initial deployment with minimal traffic
    Initial,
    /// Progressively increasing traffic
    Progressing,
    /// Canary is stable at target weight
    Stable,
    /// Promoting canary to full deployment
    Promoting,
    /// Rolling back canary
    RollingBack,
    /// Canary completed (success or failure)
    Completed,
}

/// Canary status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryStatus {
    /// Current phase
    pub phase: CanaryPhase,
    /// Current traffic weight (0-100)
    pub weight: u32,
    /// Time in current phase
    pub phase_duration: Duration,
    /// Total duration of canary
    pub total_duration: Duration,
    /// Number of increments completed
    pub increments_completed: u32,
    /// Detected anomalies
    pub anomalies: Vec<Anomaly>,
}

impl Default for CanaryStatus {
    fn default() -> Self {
        Self {
            phase: CanaryPhase::NotStarted,
            weight: 0,
            phase_duration: Duration::ZERO,
            total_duration: Duration::ZERO,
            increments_completed: 0,
            anomalies: Vec::new(),
        }
    }
}

/// Anomaly detected during canary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    /// Type of anomaly
    pub anomaly_type: AnomalyType,
    /// Severity (0.0-1.0)
    pub severity: f64,
    /// Description
    pub description: String,
    /// Metric value that triggered the anomaly
    pub metric_value: f64,
    /// Expected baseline value
    pub baseline_value: f64,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Types of anomalies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnomalyType {
    /// Error rate exceeded threshold
    ErrorRateSpike,
    /// Latency exceeded threshold
    LatencySpike,
    /// Resource usage exceeded threshold
    ResourceExhaustion,
    /// Custom metric anomaly
    CustomMetric,
}

/// Canary test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryResult {
    /// Whether canary was successful
    pub success: bool,
    /// Final status
    pub final_status: CanaryStatus,
    /// Total duration
    pub total_duration: Duration,
    /// Reason for failure (if any)
    pub failure_reason: Option<String>,
    /// Metrics collected during canary
    pub metrics: MetricSummary,
}

/// Summary of metrics collected during canary
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetricSummary {
    /// Average error rate during canary
    pub avg_error_rate: f64,
    /// Average latency during canary (ms)
    pub avg_latency_ms: f64,
    /// P99 latency during canary (ms)
    pub p99_latency_ms: f64,
    /// Total requests during canary
    pub total_requests: u64,
    /// Total errors during canary
    pub total_errors: u64,
}

/// Threshold status for a specific metric
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThresholdStatus {
    /// Within acceptable range
    Normal,
    /// Approaching threshold
    Warning,
    /// Exceeded threshold
    Exceeded,
}

/// Canary tester trait for custom test implementations
pub trait CanaryTester: Send + Sync {
    /// Run pre-deployment checks
    fn pre_deployment_check(&self) -> Result<bool, String>;

    /// Collect metrics for analysis
    fn collect_metrics(&self) -> Result<MetricSummary, String>;

    /// Check for anomalies
    fn check_anomalies(&self, metrics: &MetricSummary) -> Vec<Anomaly>;

    /// Run post-deployment verification
    fn post_deployment_verify(&self) -> Result<bool, String>;
}

/// Canary runner that orchestrates the canary deployment
pub struct CanaryRunner {
    config: CanaryConfig,
    status: CanaryStatus,
    tester: Option<Box<dyn CanaryTester>>,
    metrics_history: Vec<MetricSummary>,
}

impl CanaryRunner {
    /// Create a new canary runner with the given configuration
    pub fn new(config: CanaryConfig) -> Self {
        Self {
            config,
            status: CanaryStatus::default(),
            tester: None,
            metrics_history: Vec::new(),
        }
    }

    /// Set a custom canary tester
    pub fn with_tester(mut self, tester: Box<dyn CanaryTester>) -> Self {
        self.tester = Some(tester);
        self
    }

    /// Start the canary deployment
    pub fn start(&mut self) -> Result<(), String> {
        if self.status.phase != CanaryPhase::NotStarted {
            return Err("Canary already started".to_string());
        }

        self.status.phase = CanaryPhase::Initial;
        self.status.weight = self.config.initial_weight;
        Ok(())
    }

    /// Progress to the next canary phase
    pub fn progress(&mut self) -> Result<(), String> {
        match self.status.phase {
            CanaryPhase::Initial | CanaryPhase::Progressing => {
                let new_weight = self.status.weight + self.config.increment_step;
                if new_weight >= self.config.max_weight {
                    self.status.phase = CanaryPhase::Stable;
                    self.status.weight = self.config.max_weight;
                } else {
                    self.status.phase = CanaryPhase::Progressing;
                    self.status.weight = new_weight;
                }
                self.status.increments_completed += 1;
                Ok(())
            }
            CanaryPhase::Stable => {
                self.status.phase = CanaryPhase::Promoting;
                Ok(())
            }
            _ => Err("Cannot progress from current phase".to_string()),
        }
    }

    /// Rollback the canary deployment
    pub fn rollback(&mut self, reason: &str) -> Result<(), String> {
        self.status.phase = CanaryPhase::RollingBack;
        self.status.anomalies.push(Anomaly {
            anomaly_type: AnomalyType::CustomMetric,
            severity: 1.0,
            description: format!("Rollback triggered: {}", reason),
            metric_value: 0.0,
            baseline_value: 0.0,
            timestamp: chrono::Utc::now(),
        });
        self.status.weight = 0;
        self.status.phase = CanaryPhase::Completed;
        Ok(())
    }

    /// Get current canary status
    pub fn status(&self) -> &CanaryStatus {
        &self.status
    }

    /// Get canary configuration
    pub fn config(&self) -> &CanaryConfig {
        &self.config
    }

    /// Check if canary is complete
    pub fn is_complete(&self) -> bool {
        self.status.phase == CanaryPhase::Completed
    }

    /// Record metrics for the current interval
    pub fn record_metrics(&mut self, metrics: MetricSummary) {
        self.metrics_history.push(metrics.clone());

        // Check for anomalies
        if let Some(ref tester) = self.tester {
            let anomalies = tester.check_anomalies(&metrics);
            if !anomalies.is_empty() && self.config.auto_rollback {
                self.status.anomalies.extend(anomalies);
                let _ = self.rollback("Anomaly detected");
            }
        }
    }

    /// Complete the canary successfully
    pub fn complete_success(&mut self) -> CanaryResult {
        self.status.phase = CanaryPhase::Completed;
        self.status.weight = 100;

        CanaryResult {
            success: true,
            final_status: self.status.clone(),
            total_duration: self.status.total_duration,
            failure_reason: None,
            metrics: self
                .metrics_history
                .iter()
                .fold(MetricSummary::default(), |acc, m| MetricSummary {
                    avg_error_rate: acc.avg_error_rate + m.avg_error_rate,
                    avg_latency_ms: acc.avg_latency_ms + m.avg_latency_ms,
                    p99_latency_ms: acc.p99_latency_ms.max(m.p99_latency_ms),
                    total_requests: acc.total_requests + m.total_requests,
                    total_errors: acc.total_errors + m.total_errors,
                }),
        }
    }

    /// Complete the canary with failure
    pub fn complete_failure(&mut self, reason: &str) -> CanaryResult {
        self.status.phase = CanaryPhase::Completed;

        CanaryResult {
            success: false,
            final_status: self.status.clone(),
            total_duration: self.status.total_duration,
            failure_reason: Some(reason.to_string()),
            metrics: self
                .metrics_history
                .iter()
                .fold(MetricSummary::default(), |acc, m| MetricSummary {
                    avg_error_rate: acc.avg_error_rate + m.avg_error_rate,
                    avg_latency_ms: acc.avg_latency_ms + m.avg_latency_ms,
                    p99_latency_ms: acc.p99_latency_ms.max(m.p99_latency_ms),
                    total_requests: acc.total_requests + m.total_requests,
                    total_errors: acc.total_errors + m.total_errors,
                }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canary_config_default() {
        let config = CanaryConfig::default();
        assert_eq!(config.initial_weight, 5);
        assert_eq!(config.max_weight, 50);
        assert!(config.auto_rollback);
    }

    #[test]
    fn test_canary_runner_start() {
        let config = CanaryConfig::default();
        let mut runner = CanaryRunner::new(config);

        assert!(runner.start().is_ok());
        assert_eq!(runner.status().phase, CanaryPhase::Initial);
        assert!(runner.start().is_err()); // Already started
    }

    #[test]
    fn test_canary_runner_progress() {
        let config = CanaryConfig {
            initial_weight: 5,
            max_weight: 15,
            increment_step: 5,
            ..Default::default()
        };
        let mut runner = CanaryRunner::new(config);

        runner.start().unwrap();
        assert_eq!(runner.status().weight, 5);

        runner.progress().unwrap();
        assert_eq!(runner.status().weight, 10);

        runner.progress().unwrap();
        assert_eq!(runner.status().weight, 15);
        assert_eq!(runner.status().phase, CanaryPhase::Stable);
    }
}
