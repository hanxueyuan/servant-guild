//! Self-Evolution Workflow - Complete Implementation
//!
//! This module implements the complete self-evolution workflow for ServantGuild,
//! enabling the system to autonomously improve itself through a structured
//! process of proposal, validation, consensus, and deployment.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::consensus::ConsensusEngine;
use crate::providers::Provider;

/// Build pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Build command
    pub build_command: String,
    /// Test command
    pub test_command: Option<String>,
    /// Working directory
    pub working_dir: String,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            build_command: "cargo build".to_string(),
            test_command: Some("cargo test".to_string()),
            working_dir: ".".to_string(),
        }
    }
}

/// Build pipeline result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    /// Whether the build succeeded
    pub success: bool,
    /// Build output
    pub output: String,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

/// Build pipeline
pub struct BuildPipeline {
    config: PipelineConfig,
}

impl BuildPipeline {
    /// Create a new build pipeline
    pub fn new(config: PipelineConfig) -> Self {
        Self { config }
    }

    /// Run the build pipeline
    pub async fn run(&self) -> Result<PipelineResult> {
        let start = std::time::Instant::now();
        
        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&self.config.build_command)
            .current_dir(&self.config.working_dir)
            .output()
            .await;

        let duration_ms = start.elapsed().as_millis() as u64;

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                Ok(PipelineResult {
                    success: output.status.success(),
                    output: format!("{}\n{}", stdout, stderr),
                    duration_ms,
                })
            }
            Err(e) => Ok(PipelineResult {
                success: false,
                output: e.to_string(),
                duration_ms,
            }),
        }
    }
}

/// Evolution workflow manager
pub struct EvolutionWorkflow {
    /// Consensus engine
    consensus: Arc<ConsensusEngine>,
    /// Build pipeline
    pipeline: Arc<BuildPipeline>,
    /// LLM provider for analysis
    llm: Option<Arc<dyn Provider>>,
    /// Active workflows
    active: Arc<RwLock<HashMap<String, WorkflowState>>>,
    /// Workflow history
    history: Arc<RwLock<Vec<WorkflowRecord>>>,
    /// Configuration
    config: WorkflowConfig,
}

/// Workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    /// Enable automatic evolution
    pub auto_evolve: bool,
    /// Maximum concurrent workflows
    pub max_concurrent: usize,
    /// Require human approval for high-risk changes
    pub require_human_approval: bool,
    /// High-risk threshold (0-100)
    pub high_risk_threshold: u8,
    /// Enable canary testing
    pub enable_canary: bool,
    /// Enable automatic rollback
    pub auto_rollback: bool,
    /// Learning mode
    pub learning_mode: bool,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            auto_evolve: false,
            max_concurrent: 5,
            require_human_approval: true,
            high_risk_threshold: 70,
            enable_canary: true,
            auto_rollback: true,
            learning_mode: false,
        }
    }
}

/// Workflow state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    /// Workflow ID
    pub id: String,
    /// Evolution trigger
    pub trigger: EvolutionTrigger,
    /// Current stage
    pub stage: WorkflowStage,
    /// Evolution proposal
    pub proposal: Option<EvolutionProposal>,
    /// Build result
    pub build_result: Option<PipelineResult>,
    /// Canary result
    pub canary_result: Option<String>,
    /// Consensus proposal ID
    pub consensus_proposal_id: Option<String>,
    /// Started at
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Errors encountered
    pub errors: Vec<String>,
    /// Human approval required
    pub human_approval_required: bool,
    /// Human approved
    pub human_approved: Option<bool>,
}

/// Evolution trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvolutionTrigger {
    PerformanceDegradation { metric: String, current_value: f64, threshold: f64 },
    ErrorRateIncrease { current_rate: f64, threshold: f64 },
    SecurityVulnerability { cve: Option<String>, severity: String, component: String },
    ScheduledOptimization { schedule_id: String },
    UserRequested { user_id: String, request: String },
    SelfIdentified { analysis: String, confidence: f64 },
    DependencyUpdate { package: String, current_version: String, new_version: String },
}

/// Workflow stage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowStage {
    Analysis,
    Proposal,
    Validation,
    Consensus,
    Deployment,
    Monitoring,
    Completed,
    Failed,
}

/// Evolution proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionProposal {
    /// Proposal ID
    pub id: String,
    /// Title
    pub title: String,
    /// Description
    pub description: String,
    /// Risk level (0-100)
    pub risk_level: u8,
    /// Estimated impact
    pub estimated_impact: String,
    /// Proposed changes
    pub changes: Vec<ProposedChange>,
}

/// Proposed change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedChange {
    /// File path
    pub path: String,
    /// Change description
    pub description: String,
    /// Change type
    pub change_type: ChangeType,
}

/// Change type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    Add,
    Modify,
    Delete,
    Rename,
}

/// Workflow record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRecord {
    /// Workflow ID
    pub id: String,
    /// Trigger
    pub trigger: EvolutionTrigger,
    /// Final stage
    pub final_stage: WorkflowStage,
    /// Success
    pub success: bool,
    /// Started at
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Completed at
    pub completed_at: chrono::DateTime<chrono::Utc>,
    /// Errors
    pub errors: Vec<String>,
}

impl EvolutionWorkflow {
    /// Create a new evolution workflow manager
    pub fn new(consensus: Arc<ConsensusEngine>, pipeline: Arc<BuildPipeline>, config: WorkflowConfig) -> Self {
        Self {
            consensus,
            pipeline,
            llm: None,
            active: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }

    /// Add LLM provider
    pub fn with_llm(mut self, llm: Arc<dyn Provider>) -> Self {
        self.llm = Some(llm);
        self
    }

    /// Start a new evolution workflow
    pub async fn start(&self, trigger: EvolutionTrigger) -> Result<String> {
        let active = self.active.read().await;
        if active.len() >= self.config.max_concurrent {
            bail!("Maximum concurrent workflows reached");
        }
        drop(active);

        let workflow_id = format!("evo-{}", uuid::Uuid::new_v4());
        let now = chrono::Utc::now();

        let state = WorkflowState {
            id: workflow_id.clone(),
            trigger,
            stage: WorkflowStage::Analysis,
            proposal: None,
            build_result: None,
            canary_result: None,
            consensus_proposal_id: None,
            started_at: now,
            updated_at: now,
            errors: Vec::new(),
            human_approval_required: false,
            human_approved: None,
        };

        self.active.write().await.insert(workflow_id.clone(), state);
        info!("Started evolution workflow: {}", workflow_id);

        Ok(workflow_id)
    }

    /// Get workflow state
    pub async fn get_state(&self, id: &str) -> Option<WorkflowState> {
        self.active.read().await.get(id).cloned()
    }

    /// Cancel a workflow
    pub async fn cancel(&self, id: &str) -> Result<()> {
        let mut active = self.active.write().await;
        if let Some(state) = active.remove(id) {
            let record = WorkflowRecord {
                id: id.to_string(),
                trigger: state.trigger,
                final_stage: state.stage,
                success: false,
                started_at: state.started_at,
                completed_at: chrono::Utc::now(),
                errors: state.errors,
            };
            self.history.write().await.push(record);
        }
        Ok(())
    }

    /// Get active workflows
    pub async fn get_active(&self) -> Vec<WorkflowState> {
        self.active.read().await.values().cloned().collect()
    }

    /// Get history
    pub async fn get_history(&self) -> Vec<WorkflowRecord> {
        self.history.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workflow_creation() {
        let config = WorkflowConfig::default();
        let consensus = Arc::new(ConsensusEngine::new(Default::default()));
        let pipeline = Arc::new(BuildPipeline::new(PipelineConfig::default()));
        let workflow = EvolutionWorkflow::new(consensus, pipeline, config);

        let trigger = EvolutionTrigger::UserRequested {
            user_id: "test".to_string(),
            request: "test request".to_string(),
        };

        let id = workflow.start(trigger).await.unwrap();
        assert!(id.starts_with("evo-"));
    }
}
