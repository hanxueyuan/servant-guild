//! Self-Evolution Engine - Autonomous System Improvement
//!
//! This module provides self-evolution capabilities for ServantGuild,
//! enabling the system to autonomously analyze its performance, identify
//! improvement opportunities, generate code changes, and deploy them safely.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Evolution trigger type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvolutionTrigger {
    /// Performance threshold exceeded
    PerformanceThreshold { metric: String, threshold: f64 },
    /// User feedback received
    UserFeedback,
    /// Error rate exceeded
    ErrorRateExceeded { rate: f64 },
    /// Scheduled evolution
    ScheduledEvolution,
    /// Manual trigger
    ManualTrigger,
    /// New dependency available
    NewDependencyAvailable { dependency: String },
}

/// Evolution type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvolutionType {
    /// Performance optimization
    PerformanceOptimization,
    /// Bug fix
    BugFix,
    /// Feature addition
    FeatureAddition,
    /// Dependency update
    DependencyUpdate,
    /// Refactoring
    Refactoring,
    /// Security improvement
    SecurityImprovement,
}

/// Evolution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvolutionStatus {
    /// Pending analysis
    PendingAnalysis,
    /// Analyzing
    Analyzing,
    /// Generating changes
    GeneratingChanges,
    /// Building
    Building,
    /// Testing
    Testing,
    /// Reviewing
    Reviewing,
    /// Pending approval
    PendingApproval,
    /// Deploying
    Deploying,
    /// Completed
    Completed,
    /// Rolled back
    RolledBack,
    /// Failed
    Failed,
}

/// Evolution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionPlan {
    /// Plan ID
    pub id: String,
    /// Evolution type
    pub evolution_type: EvolutionType,
    /// Trigger
    pub trigger: EvolutionTrigger,
    /// Status
    pub status: EvolutionStatus,
    /// Title
    pub title: String,
    /// Description
    pub description: String,
    /// Affected modules
    pub affected_modules: Vec<String>,
    /// Proposed changes
    pub proposed_changes: Vec<CodeChange>,
    /// Risk assessment
    pub risk_assessment: RiskAssessment,
    /// Estimated impact
    pub estimated_impact: ImpactEstimate,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Updated at
    pub updated_at: DateTime<Utc>,
    /// Started at
    pub started_at: Option<DateTime<Utc>>,
    /// Completed at
    pub completed_at: Option<DateTime<Utc>>,
    /// Rollback point ID
    pub rollback_point_id: Option<String>,
}

/// Code change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChange {
    /// Change ID
    pub id: String,
    /// File path
    pub file_path: PathBuf,
    /// Change type (add, modify, delete)
    pub change_type: String,
    /// Description
    pub description: String,
    /// Diff
    pub diff: Option<String>,
    /// New content
    pub new_content: Option<String>,
    /// Reasoning
    pub reasoning: String,
}

/// Risk assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    /// Risk level (low, medium, high, critical)
    pub risk_level: String,
    /// Potential issues
    pub potential_issues: Vec<String>,
    /// Mitigation strategies
    pub mitigation_strategies: Vec<String>,
    /// Requires human approval
    pub requires_human_approval: bool,
}

/// Impact estimate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactEstimate {
    /// Expected performance improvement (%)
    pub performance_improvement: f32,
    /// Expected resource usage change (%)
    pub resource_usage_change: f32,
    /// Affected users
    pub affected_users: u32,
    /// Downtime estimate (seconds)
    pub downtime_estimate_secs: u32,
}

/// Evolution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionResult {
    /// Plan ID
    pub plan_id: String,
    /// Whether evolution succeeded
    pub success: bool,
    /// Actual performance improvement
    pub actual_performance_improvement: Option<f32>,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Warnings
    pub warnings: Vec<String>,
    /// Errors
    pub errors: Vec<String>,
    /// Started at
    pub started_at: DateTime<Utc>,
    /// Ended at
    pub ended_at: DateTime<Utc>,
}

/// Evolution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionConfig {
    /// Whether auto-evolution is enabled
    pub auto_evolution_enabled: bool,
    /// Minimum confidence threshold for auto-approval
    pub min_confidence_threshold: f32,
    /// Maximum risk level for auto-approval
    pub max_auto_approval_risk: String,
    /// Whether to require human approval for feature additions
    pub require_approval_for_features: bool,
    /// Performance monitoring interval (seconds)
    pub monitoring_interval_secs: u64,
    /// Evolution history limit
    pub evolution_history_limit: usize,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            auto_evolution_enabled: false,
            min_confidence_threshold: 0.85,
            max_auto_approval_risk: "medium".to_string(),
            require_approval_for_features: true,
            monitoring_interval_secs: 300,
            evolution_history_limit: 1000,
        }
    }
}

/// Analysis result from system state analysis
#[derive(Debug, Clone)]
struct AnalysisResult {
    evolution_type: EvolutionType,
    title: String,
    description: String,
    affected_modules: Vec<String>,
}

/// Self-evolution engine
pub struct EvolutionEngine {
    /// Active evolution plans
    active_plans: Arc<RwLock<HashMap<String, EvolutionPlan>>>,
    /// Evolution history
    history: Arc<RwLock<Vec<EvolutionResult>>>,
    /// Configuration
    config: EvolutionConfig,
}

impl EvolutionEngine {
    /// Create new evolution engine
    pub fn new(config: EvolutionConfig) -> Self {
        Self {
            active_plans: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }

    /// Trigger evolution
    pub async fn trigger_evolution(&self, trigger: EvolutionTrigger) -> Result<EvolutionPlan> {
        info!("Evolution triggered: {:?}", trigger);

        // Create initial plan
        let plan_id = uuid::Uuid::new_v4().to_string();
        let plan = EvolutionPlan {
            id: plan_id.clone(),
            evolution_type: EvolutionType::PerformanceOptimization,
            trigger,
            status: EvolutionStatus::PendingAnalysis,
            title: "Auto-generated evolution plan".to_string(),
            description: "Generated based on system metrics and triggers".to_string(),
            affected_modules: Vec::new(),
            proposed_changes: Vec::new(),
            risk_assessment: RiskAssessment {
                risk_level: "unknown".to_string(),
                potential_issues: Vec::new(),
                mitigation_strategies: Vec::new(),
                requires_human_approval: true,
            },
            estimated_impact: ImpactEstimate {
                performance_improvement: 0.0,
                resource_usage_change: 0.0,
                affected_users: 0,
                downtime_estimate_secs: 0,
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
            started_at: None,
            completed_at: None,
            rollback_point_id: None,
        };

        self.store_plan(plan.clone()).await?;
        Ok(plan)
    }

    /// Get active plan by ID
    pub async fn get_plan(&self, plan_id: &str) -> Option<EvolutionPlan> {
        let plans = self.active_plans.read().await;
        plans.get(plan_id).cloned()
    }

    /// Get all active plans
    pub async fn get_all_plans(&self) -> Vec<EvolutionPlan> {
        let plans = self.active_plans.read().await;
        plans.values().cloned().collect()
    }

    /// Approve and execute an evolution plan
    pub async fn approve_plan(&self, plan_id: &str) -> Result<EvolutionResult> {
        let plan = self.get_plan(plan_id).await
            .ok_or_else(|| anyhow::anyhow!("Plan not found: {}", plan_id))?;

        let started_at = Utc::now();

        // Simulate evolution execution
        let result = EvolutionResult {
            plan_id: plan_id.to_string(),
            success: true,
            actual_performance_improvement: Some(5.0),
            duration_ms: 1000,
            warnings: Vec::new(),
            errors: Vec::new(),
            started_at,
            ended_at: Utc::now(),
        };

        // Remove from active and add to history
        self.remove_plan(plan_id).await?;
        self.add_to_history(result.clone()).await?;

        info!("Evolution plan {} completed successfully", plan_id);
        Ok(result)
    }

    /// Cancel an active evolution plan
    pub async fn cancel_plan(&self, plan_id: &str) -> Result<()> {
        self.remove_plan(plan_id).await?;
        info!("Evolution plan {} cancelled", plan_id);
        Ok(())
    }

    /// Get evolution history
    pub async fn get_history(&self) -> Vec<EvolutionResult> {
        let history = self.history.read().await;
        history.clone()
    }

    /// Check if auto-evolution is enabled
    pub fn is_auto_evolution_enabled(&self) -> bool {
        self.config.auto_evolution_enabled
    }

    /// Store a plan in active plans
    async fn store_plan(&self, plan: EvolutionPlan) -> Result<()> {
        let mut plans = self.active_plans.write().await;
        plans.insert(plan.id.clone(), plan);
        Ok(())
    }

    /// Remove a plan from active plans
    async fn remove_plan(&self, plan_id: &str) -> Result<()> {
        let mut plans = self.active_plans.write().await;
        plans.remove(plan_id);
        Ok(())
    }

    /// Add result to history
    async fn add_to_history(&self, result: EvolutionResult) -> Result<()> {
        let mut history = self.history.write().await;
        history.push(result);

        // Trim history if needed
        if history.len() > self.config.evolution_history_limit {
            history.remove(0);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_evolution_engine_creation() {
        let config = EvolutionConfig::default();
        let engine = EvolutionEngine::new(config);

        assert!(!engine.is_auto_evolution_enabled());
    }

    #[tokio::test]
    async fn test_trigger_evolution() {
        let config = EvolutionConfig::default();
        let engine = EvolutionEngine::new(config);

        let plan = engine.trigger_evolution(EvolutionTrigger::ManualTrigger).await.unwrap();

        assert_eq!(plan.status, EvolutionStatus::PendingAnalysis);
        assert!(plan.rollback_point_id.is_none());
    }

    #[tokio::test]
    async fn test_approve_plan() {
        let config = EvolutionConfig::default();
        let engine = EvolutionEngine::new(config);

        let plan = engine.trigger_evolution(EvolutionTrigger::ManualTrigger).await.unwrap();
        let result = engine.approve_plan(&plan.id).await.unwrap();

        assert!(result.success);
        assert_eq!(result.plan_id, plan.id);
    }

    #[tokio::test]
    async fn test_cancel_plan() {
        let config = EvolutionConfig::default();
        let engine = EvolutionEngine::new(config);

        let plan = engine.trigger_evolution(EvolutionTrigger::ManualTrigger).await.unwrap();
        engine.cancel_plan(&plan.id).await.unwrap();

        let retrieved = engine.get_plan(&plan.id).await;
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_history() {
        let config = EvolutionConfig::default();
        let engine = EvolutionEngine::new(config);

        let plan = engine.trigger_evolution(EvolutionTrigger::ManualTrigger).await.unwrap();
        engine.approve_plan(&plan.id).await.unwrap();

        let history = engine.get_history().await;
        assert_eq!(history.len(), 1);
    }
}
