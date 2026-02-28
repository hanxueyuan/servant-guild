//! Canary Testing - Safe Deployment Validation
//!
//! This module implements canary testing for the Warden servant,
//! enabling safe deployment of updates by gradually rolling out
//! changes and monitoring for issues.
//!
//! Canary Strategy:
//! 1. Deploy to small subset (1-5%)
//! 2. Monitor metrics for anomalies
//! 3. Gradually increase rollout
//! 4. Automatic rollback on issues
//! 5. Full deployment on success

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Canary test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryConfig {
    /// Initial rollout percentage
    pub initial_percentage: f64,
    /// Percentage increment per step
    pub increment_percentage: f64,
    /// Duration per step (seconds)
    pub step_duration_secs: u64,
    /// Maximum percentage before full rollout
    pub max_canary_percentage: f64,
    /// Metrics to monitor
    pub monitored_metrics: Vec<String>,
    /// Anomaly thresholds
    pub anomaly_thresholds: HashMap<String, AnomalyThreshold>,
    /// Automatic rollback on failure
    pub auto_rollback: bool,
    /// Required success duration (seconds)
    pub required_success_duration_secs: u64,
}

impl Default for CanaryConfig {
    fn default() -> Self {
        let mut anomaly_thresholds = HashMap::new();
        anomaly_thresholds.insert("error_rate".to_string(), AnomalyThreshold::new(0.05, 0.1));
        anomaly_thresholds.insert("latency_p99".to_string(), AnomalyThreshold::new(100.0, 200.0));
        anomaly_thresholds.insert("cpu_usage".to_string(), AnomalyThreshold::new(70.0, 90.0));
        anomaly_thresholds.insert("memory_usage".to_string(), AnomalyThreshold::new(80.0, 95.0));
        
        Self {
            initial_percentage: 5.0,
            increment_percentage: 10.0,
            step_duration_secs: 300, // 5 minutes
            max_canary_percentage: 50.0,
            monitored_metrics: vec![
                "error_rate".to_string(),
                "latency_p99".to_string(),
                "cpu_usage".to_string(),
                "memory_usage".to_string(),
            ],
            anomaly_thresholds,
            auto_rollback: true,
            required_success_duration_secs: 600, // 10 minutes
        }
    }
}

/// Anomaly threshold
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyThreshold {
    /// Warning threshold
    pub warning: f64,
    /// Critical threshold (triggers rollback)
    pub critical: f64,
}

impl AnomalyThreshold {
    /// Create new threshold
    pub fn new(warning: f64, critical: f64) -> Self {
        Self { warning, critical }
    }
    
    /// Check if value is within acceptable range
    pub fn check(&self, value: f64) -> ThresholdStatus {
        if value >= self.critical {
            ThresholdStatus::Critical
        } else if value >= self.warning {
            ThresholdStatus::Warning
        } else {
            ThresholdStatus::Ok
        }
    }
}

/// Threshold status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThresholdStatus {
    /// Within acceptable range
    Ok,
    /// Above warning threshold
    Warning,
    /// Above critical threshold
    Critical,
}

/// Canary test status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryStatus {
    /// Test ID
    pub id: String,
    /// Module being tested
    pub module_id: String,
    /// Current rollout percentage
    pub rollout_percentage: f64,
    /// Current phase
    pub phase: CanaryPhase,
    /// Current metrics
    pub metrics: HashMap<String, f64>,
    /// Anomalies detected
    pub anomalies: Vec<Anomaly>,
    /// Started at
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Current step
    pub current_step: u32,
    /// Total steps
    pub total_steps: u32,
}

/// Canary phase
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CanaryPhase {
    /// Initializing
    Initializing,
    /// Running canary
    Running,
    /// Monitoring for issues
    Monitoring,
    /// Increasing rollout
    Increasing,
    /// Paused for investigation
    Paused,
    /// Completed successfully
    Completed,
    /// Failed and rolled back
    Failed,
    /// Aborted by user
    Aborted,
}

/// Anomaly detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    /// Anomaly ID
    pub id: String,
    /// Metric name
    pub metric: String,
    /// Detected value
    pub value: f64,
    /// Threshold exceeded
    pub threshold_type: ThresholdStatus,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Description
    pub description: String,
}

/// Canary test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryResult {
    /// Test ID
    pub id: String,
    /// Success flag
    pub success: bool,
    /// Final rollout percentage achieved
    pub final_percentage: f64,
    /// Total duration (seconds)
    pub duration_secs: u64,
    /// Steps completed
    pub steps_completed: u32,
    /// Metrics summary
    pub metrics_summary: HashMap<String, MetricSummary>,
    /// Anomalies detected
    pub anomalies: Vec<Anomaly>,
    /// Rollback occurred
    pub rolled_back: bool,
    /// Reason for failure (if failed)
    pub failure_reason: Option<String>,
}

/// Metric summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSummary {
    /// Minimum value
    pub min: f64,
    /// Maximum value
    pub max: f64,
    /// Average value
    pub avg: f64,
    /// Final value
    pub final_value: f64,
}

/// Canary tester
pub struct CanaryTester {
    /// Configuration
    config: CanaryConfig,
    /// Active canary tests
    active_tests: Arc<RwLock<HashMap<String, CanaryStatus>>>,
    /// Test history
    history: Arc<RwLock<Vec<CanaryResult>>>,
    /// Metrics collector
    metrics_collector: Arc<dyn MetricsCollector>,
}

/// Metrics collector trait
#[async_trait::async_trait]
pub trait MetricsCollector: Send + Sync {
    /// Collect current metrics
    async fn collect(&self, module_id: &str, metrics: &[String]) -> Result<HashMap<String, f64>>;
}

/// Default metrics collector (mock)
pub struct DefaultMetricsCollector;

#[async_trait::async_trait]
impl MetricsCollector for DefaultMetricsCollector {
    async fn collect(&self, _module_id: &str, metrics: &[String]) -> Result<HashMap<String, f64>> {
        // Return mock values
        Ok(metrics.iter().map(|m| (m.clone(), 0.0)).collect())
    }
}

impl CanaryTester {
    /// Create a new canary tester
    pub fn new(config: CanaryConfig, metrics_collector: Arc<dyn MetricsCollector>) -> Self {
        Self {
            config,
            active_tests: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            metrics_collector,
        }
    }

    /// Create with default configuration
    pub fn with_defaults(metrics_collector: Arc<dyn MetricsCollector>) -> Self {
        Self::new(CanaryConfig::default(), metrics_collector)
    }

    /// Start a canary test
    pub async fn start_test(&self, module_id: &str, new_version: &str) -> Result<String> {
        let test_id = format!("canary-{}", uuid::Uuid::new_v4());
        
        // Calculate total steps
        let total_steps = ((self.config.max_canary_percentage - self.config.initial_percentage) 
            / self.config.increment_percentage).ceil() as u32 + 1;
        
        let status = CanaryStatus {
            id: test_id.clone(),
            module_id: module_id.to_string(),
            rollout_percentage: 0.0,
            phase: CanaryPhase::Initializing,
            metrics: HashMap::new(),
            anomalies: Vec::new(),
            started_at: chrono::Utc::now(),
            current_step: 0,
            total_steps,
        };
        
        self.active_tests.write().await.insert(test_id.clone(), status);
        
        info!("Started canary test: {} for module {}", test_id, module_id);
        
        // Begin initial rollout
        self.execute_step(&test_id, self.config.initial_percentage).await?;
        
        Ok(test_id)
    }

    /// Execute a canary step
    async fn execute_step(&self, test_id: &str, percentage: f64) -> Result<()> {
        let mut tests = self.active_tests.write().await;
        let status = tests.get_mut(test_id)
            .context("Test not found")?;
        
        status.phase = CanaryPhase::Running;
        status.rollout_percentage = percentage;
        status.current_step += 1;
        
        info!(
            "Canary {} step {}: rolling out to {:.1}%",
            test_id, status.current_step, percentage
        );
        
        // Here would integrate with actual deployment system
        // to route percentage of traffic to new version
        
        Ok(())
    }

    /// Monitor active canary test
    pub async fn monitor(&self, test_id: &str) -> Result<CanaryStatus> {
        let mut tests = self.active_tests.write().await;
        let status = tests.get_mut(test_id)
            .context("Test not found")?;
        
        // Collect current metrics
        let metrics = self.metrics_collector
            .collect(&status.module_id, &self.config.monitored_metrics)
            .await?;
        
        status.metrics = metrics.clone();
        
        // Check for anomalies
        for (metric, value) in &metrics {
            if let Some(threshold) = self.config.anomaly_thresholds.get(metric) {
                let check_result = threshold.check(*value);
                
                if check_result != ThresholdStatus::Ok {
                    let anomaly = Anomaly {
                        id: format!("anomaly-{}", uuid::Uuid::new_v4()),
                        metric: metric.clone(),
                        value: *value,
                        threshold_type: check_result,
                        timestamp: chrono::Utc::now(),
                        description: format!(
                            "{} {:.2} exceeds {} threshold {:.2}",
                            metric, value, 
                            if check_result == ThresholdStatus::Critical { "critical" } else { "warning" },
                            if check_result == ThresholdStatus::Critical { threshold.critical } else { threshold.warning }
                        ),
                    };
                    
                    status.anomalies.push(anomaly);
                    
                    // Check for critical issues
                    if check_result == ThresholdStatus::Critical && self.config.auto_rollback {
                        warn!("Critical anomaly detected, triggering rollback");
                        return self.handle_failure(status, "Critical metric threshold exceeded").await;
                    }
                }
            }
        }
        
        status.phase = CanaryPhase::Monitoring;
        
        Ok(status.clone())
    }

    /// Advance to next step
    pub async fn advance(&self, test_id: &str) -> Result<CanaryStatus> {
        let mut tests = self.active_tests.write().await;
        let status = tests.get_mut(test_id)
            .context("Test not found")?;
        
        // Check if we've reached max
        if status.rollout_percentage >= self.config.max_canary_percentage {
            // Check if we should complete
            status.phase = CanaryPhase::Completed;
            status.rollout_percentage = 100.0;
            
            info!("Canary test {} completed successfully", test_id);
            
            return Ok(status.clone());
        }
        
        // Increment rollout
        let new_percentage = (status.rollout_percentage + self.config.increment_percentage)
            .min(self.config.max_canary_percentage);
        
        status.phase = CanaryPhase::Increasing;
        status.rollout_percentage = new_percentage;
        status.current_step += 1;
        
        info!(
            "Canary {} advancing to {:.1}%",
            test_id, new_percentage
        );
        
        Ok(status.clone())
    }

    /// Handle failure and rollback
    async fn handle_failure(&mut status: &mut CanaryStatus, reason: &str) -> Result<CanaryStatus> {
        status.phase = CanaryPhase::Failed;
        
        // Would integrate with deployment system to rollback
        status.rollout_percentage = 0.0;
        
        warn!(
            "Canary test {} failed: {}. Rolled back.",
            status.id, reason
        );
        
        Ok(status.clone())
    }

    /// Pause a canary test
    pub async fn pause_test(&self, test_id: &str) -> Result<()> {
        let mut tests = self.active_tests.write().await;
        let status = tests.get_mut(test_id)
            .context("Test not found")?;
        
        status.phase = CanaryPhase::Paused;
        
        info!("Canary test {} paused", test_id);
        
        Ok(())
    }

    /// Abort a canary test
    pub async fn abort_test(&self, test_id: &str) -> Result<()> {
        let mut tests = self.active_tests.write().await;
        let status = tests.get_mut(test_id)
            .context("Test not found")?;
        
        status.phase = CanaryPhase::Aborted;
        status.rollout_percentage = 0.0;
        
        info!("Canary test {} aborted", test_id);
        
        Ok(())
    }

    /// Get test status
    pub async fn get_status(&self, test_id: &str) -> Option<CanaryStatus> {
        self.active_tests.read().await.get(test_id).cloned()
    }

    /// List active tests
    pub async fn list_active(&self) -> Vec<CanaryStatus> {
        self.active_tests.read().await.values().cloned().collect()
    }

    /// Get test history
    pub async fn get_history(&self) -> Vec<CanaryResult> {
        self.history.read().await.clone()
    }

    /// Calculate health score for a test
    pub fn calculate_health_score(&self, status: &CanaryStatus) -> f64 {
        let mut score = 100.0;
        
        // Penalize for anomalies
        for anomaly in &status.anomalies {
            match anomaly.threshold_type {
                ThresholdStatus::Critical => score -= 30.0,
                ThresholdStatus::Warning => score -= 10.0,
                ThresholdStatus::Ok => {}
            }
        }
        
        // Penalize for high metric values
        for (metric, value) in &status.metrics {
            if let Some(threshold) = self.config.anomaly_thresholds.get(metric) {
                if *value > threshold.warning {
                    let excess = (value - threshold.warning) / threshold.warning;
                    score -= excess * 10.0;
                }
            }
        }
        
        // Reward for progress
        score += (status.rollout_percentage / self.config.max_canary_percentage) * 10.0;
        
        score.max(0.0).min(100.0)
    }
}

/// Canary test runner
pub struct CanaryRunner {
    tester: Arc<CanaryTester>,
}

impl CanaryRunner {
    /// Create a new runner
    pub fn new(tester: Arc<CanaryTester>) -> Self {
        Self { tester }
    }

    /// Run a complete canary test
    pub async fn run(&self, module_id: &str, new_version: &str) -> Result<CanaryResult> {
        let start_time = std::time::Instant::now();
        
        // Start the test
        let test_id = self.tester.start_test(module_id, new_version).await?;
        
        let mut final_percentage = self.tester.config.initial_percentage;
        let mut steps_completed = 1;
        let mut anomalies = Vec::new();
        let mut metrics_summary = HashMap::new();
        
        loop {
            // Wait for step duration
            tokio::time::sleep(std::time::Duration::from_secs(
                self.tester.config.step_duration_secs
            )).await;
            
            // Monitor
            let status = self.tester.monitor(&test_id).await?;
            
            // Collect metrics for summary
            for (metric, value) in &status.metrics {
                let entry = metrics_summary.entry(metric.clone()).or_insert(MetricSummary {
                    min: f64::MAX,
                    max: f64::MIN,
                    avg: 0.0,
                    final_value: 0.0,
                });
                entry.min = entry.min.min(*value);
                entry.max = entry.max.max(*value);
                entry.avg = (entry.avg + value) / 2.0;
                entry.final_value = *value;
            }
            
            anomalies = status.anomalies.clone();
            
            // Check if failed
            if status.phase == CanaryPhase::Failed {
                return Ok(CanaryResult {
                    id: test_id.clone(),
                    success: false,
                    final_percentage: 0.0,
                    duration_secs: start_time.elapsed().as_secs(),
                    steps_completed,
                    metrics_summary,
                    anomalies,
                    rolled_back: true,
                    failure_reason: Some("Critical threshold exceeded".to_string()),
                });
            }
            
            // Check if completed or aborted
            if status.phase == CanaryPhase::Completed || status.phase == CanaryPhase::Aborted {
                return Ok(CanaryResult {
                    id: test_id.clone(),
                    success: status.phase == CanaryPhase::Completed,
                    final_percentage: status.rollout_percentage,
                    duration_secs: start_time.elapsed().as_secs(),
                    steps_completed,
                    metrics_summary,
                    anomalies,
                    rolled_back: false,
                    failure_reason: if status.phase == CanaryPhase::Aborted {
                        Some("Aborted by user".to_string())
                    } else {
                        None
                    },
                });
            }
            
            // Advance to next step
            let new_status = self.tester.advance(&test_id).await?;
            
            if new_status.phase == CanaryPhase::Completed {
                return Ok(CanaryResult {
                    id: test_id.clone(),
                    success: true,
                    final_percentage: 100.0,
                    duration_secs: start_time.elapsed().as_secs(),
                    steps_completed,
                    metrics_summary,
                    anomalies,
                    rolled_back: false,
                    failure_reason: None,
                });
            }
            
            final_percentage = new_status.rollout_percentage;
            steps_completed += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canary_config_defaults() {
        let config = CanaryConfig::default();
        
        assert_eq!(config.initial_percentage, 5.0);
        assert_eq!(config.increment_percentage, 10.0);
        assert!(config.auto_rollback);
    }

    #[test]
    fn test_anomaly_threshold() {
        let threshold = AnomalyThreshold::new(50.0, 100.0);
        
        assert_eq!(threshold.check(25.0), ThresholdStatus::Ok);
        assert_eq!(threshold.check(75.0), ThresholdStatus::Warning);
        assert_eq!(threshold.check(150.0), ThresholdStatus::Critical);
    }

    #[tokio::test]
    async fn test_canary_tester() {
        let collector = Arc::new(DefaultMetricsCollector);
        let tester = CanaryTester::with_defaults(collector);
        
        let test_id = tester.start_test("test-module", "1.1.0").await.unwrap();
        
        assert!(test_id.starts_with("canary-"));
        
        let status = tester.get_status(&test_id).await.unwrap();
        assert_eq!(status.module_id, "test-module");
        assert_eq!(status.current_step, 1);
    }

    #[tokio::test]
    async fn test_canary_advance() {
        let collector = Arc::new(DefaultMetricsCollector);
        let tester = CanaryTester::with_defaults(collector);
        
        let test_id = tester.start_test("test-module", "1.1.0").await.unwrap();
        
        let status = tester.advance(&test_id).await.unwrap();
        assert_eq!(status.current_step, 2);
        assert!(status.rollout_percentage > 5.0);
    }
}
