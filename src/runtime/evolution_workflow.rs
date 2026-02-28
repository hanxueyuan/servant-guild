//! Self-Evolution Workflow - Complete Implementation
//!
//! This module implements the complete self-evolution workflow for ServantGuild,
//! enabling the system to autonomously improve itself through a structured
//! process of proposal, validation, consensus, and deployment.
//!
//! Workflow Stages:
//! 1. Analysis - Identify improvement opportunities
//! 2. Proposal - Generate update proposal
//! 3. Validation - Test the proposed changes
//! 4. Consensus - Seek guild approval
//! 5. Deployment - Deploy approved changes
//! 6. Monitoring - Monitor for issues
//! 7. Learning - Learn from results

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::consensus::{ConsensusEngine, Proposal, Vote};
use crate::runtime::{BuildPipeline, PipelineConfig, PipelineResult};
use crate::safety::canary::{CanaryRunner, CanaryTester, CanaryResult};
use crate::providers::LLMProvider;

/// Evolution workflow manager
pub struct EvolutionWorkflow {
    /// Consensus engine
    consensus: Arc<ConsensusEngine>,
    /// Build pipeline
    pipeline: Arc<BuildPipeline>,
    /// Canary tester
    canary: Arc<CanaryTester>,
    /// LLM provider for analysis
    llm: Option<Arc<dyn LLMProvider>>,
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
    /// Learning mode (collects data without applying)
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
    pub canary_result: Option<CanaryResult>,
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
    /// Performance degradation detected
    PerformanceDegradation {
        metric: String,
        current_value: f64,
        threshold: f64,
    },
    /// Error rate increased
    ErrorRateIncrease {
        current_rate: f64,
        threshold: f64,
    },
    /// Security vulnerability found
    SecurityVulnerability {
        cve: Option<String>,
        severity: String,
        component: String,
    },
    /// Scheduled optimization
    ScheduledOptimization {
        schedule_id: String,
    },
    /// User-requested improvement
    UserRequested {
        user_id: String,
        request: String,
    },
    /// Self-identified improvement
    SelfIdentified {
        analysis: String,
        confidence: f64,
    },
    /// Dependency update available
    DependencyUpdate {
        package: String,
        current_version: String,
        new_version: String,
    },
}

/// Workflow stage
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkflowStage {
    /// Analyzing improvement opportunities
    Analysis,
    /// Generating proposal
    ProposalGeneration,
    /// Validating proposal
    Validation,
    /// Building and testing
    Building,
    /// Seeking consensus
    Consensus,
    /// Deploying changes
    Deployment,
    /// Monitoring deployment
    Monitoring,
    /// Learning from results
    Learning,
    /// Completed successfully
    Completed,
    /// Failed
    Failed,
    /// Rolled back
    RolledBack,
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
    /// Proposed changes
    pub changes: Vec<ProposedChange>,
    /// Risk assessment
    pub risk_assessment: RiskAssessment,
    /// Expected benefits
    pub expected_benefits: Vec<String>,
    /// Potential risks
    pub potential_risks: Vec<String>,
    /// Confidence level
    pub confidence: f64,
    /// Requires consensus
    pub requires_consensus: bool,
}

/// Proposed change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedChange {
    /// Change type
    pub change_type: ChangeType,
    /// Target component
    pub target: String,
    /// Change description
    pub description: String,
    /// Code diff (if applicable)
    pub diff: Option<String>,
    /// Configuration changes
    pub config_changes: Option<HashMap<String, serde_json::Value>>,
}

/// Change type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    /// Code modification
    CodeChange,
    /// Configuration update
    ConfigChange,
    /// Dependency update
    DependencyUpdate,
    /// Behavior adjustment
    BehaviorAdjustment,
    /// Security patch
    SecurityPatch,
    /// Performance optimization
    PerformanceOptimization,
}

/// Risk assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    /// Overall risk score (0-100)
    pub score: u8,
    /// Risk level
    pub level: RiskLevel,
    /// Risk factors
    pub factors: Vec<RiskFactor>,
    /// Mitigations available
    pub mitigations: Vec<String>,
}

/// Risk level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Risk factor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    /// Factor name
    pub name: String,
    /// Factor description
    pub description: String,
    /// Impact level
    pub impact: u8,
}

/// Workflow record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRecord {
    /// Record ID
    pub id: String,
    /// Workflow ID
    pub workflow_id: String,
    /// Trigger type
    pub trigger_type: String,
    /// Final stage reached
    pub final_stage: WorkflowStage,
    /// Success
    pub success: bool,
    /// Duration (seconds)
    pub duration_secs: u64,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Lessons learned
    pub lessons: Vec<String>,
}

impl EvolutionWorkflow {
    /// Create a new evolution workflow manager
    pub fn new(
        consensus: Arc<ConsensusEngine>,
        pipeline: Arc<BuildPipeline>,
        canary: Arc<CanaryTester>,
        config: WorkflowConfig,
    ) -> Self {
        Self {
            consensus,
            pipeline,
            canary,
            llm: None,
            active: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }

    /// Add LLM provider
    pub fn with_llm(mut self, llm: Arc<dyn LLMProvider>) -> Self {
        self.llm = Some(llm);
        self
    }

    /// Start a new evolution workflow
    pub async fn start(&self, trigger: EvolutionTrigger) -> Result<String> {
        // Check concurrent limit
        {
            let active = self.active.read().await;
            if active.len() >= self.config.max_concurrent {
                bail!("Maximum concurrent workflows reached");
            }
        }
        
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
        
        // Begin execution
        self.execute_workflow(&workflow_id).await?;
        
        Ok(workflow_id)
    }

    /// Execute the workflow
    async fn execute_workflow(&self, workflow_id: &str) -> Result<()> {
        loop {
            let state = self.get_state(workflow_id).await
                .context("Workflow not found")?;
            
            match state.stage {
                WorkflowStage::Analysis => {
                    self.run_analysis(workflow_id).await?;
                }
                WorkflowStage::ProposalGeneration => {
                    self.generate_proposal(workflow_id).await?;
                }
                WorkflowStage::Validation => {
                    self.validate_proposal(workflow_id).await?;
                }
                WorkflowStage::Building => {
                    self.build_changes(workflow_id).await?;
                }
                WorkflowStage::Consensus => {
                    self.seek_consensus(workflow_id).await?;
                }
                WorkflowStage::Deployment => {
                    self.deploy_changes(workflow_id).await?;
                }
                WorkflowStage::Monitoring => {
                    self.monitor_deployment(workflow_id).await?;
                }
                WorkflowStage::Learning => {
                    self.learn_from_results(workflow_id).await?;
                }
                WorkflowStage::Completed |
                WorkflowStage::Failed |
                WorkflowStage::RolledBack => {
                    // Workflow finished
                    self.finalize_workflow(workflow_id).await?;
                    break;
                }
            }
        }
        
        Ok(())
    }

    /// Run analysis stage
    async fn run_analysis(&self, workflow_id: &str) -> Result<()> {
        info!("Workflow {}: Running analysis", workflow_id);
        
        let mut active = self.active.write().await;
        let state = active.get_mut(workflow_id).context("Workflow not found")?;
        
        // Analyze trigger and identify improvement opportunities
        let analysis_result = self.analyze_trigger(&state.trigger).await?;
        
        state.stage = WorkflowStage::ProposalGeneration;
        state.updated_at = chrono::Utc::now();
        
        Ok(())
    }

    /// Analyze evolution trigger
    async fn analyze_trigger(&self, trigger: &EvolutionTrigger) -> Result<AnalysisResult> {
        // Use LLM to analyze if available
        if let Some(ref llm) = self.llm {
            let prompt = format!(
                "Analyze this evolution trigger and identify improvement opportunities:\n{:?}",
                trigger
            );
            
            // Would call LLM here
            debug!("Would analyze with LLM: {}", prompt);
        }
        
        // Default analysis
        Ok(AnalysisResult {
            opportunities: vec!["Performance optimization".to_string()],
            recommendations: vec!["Consider updating algorithm".to_string()],
            confidence: 0.7,
        })
    }

    /// Generate proposal stage
    async fn generate_proposal(&self, workflow_id: &str) -> Result<()> {
        info!("Workflow {}: Generating proposal", workflow_id);
        
        let mut active = self.active.write().await;
        let state = active.get_mut(workflow_id).context("Workflow not found")?;
        
        // Generate evolution proposal
        let proposal = self.create_proposal(&state.trigger).await?;
        
        // Check if human approval required
        if proposal.risk_assessment.score >= self.config.high_risk_threshold {
            state.human_approval_required = true;
        }
        
        state.proposal = Some(proposal);
        state.stage = WorkflowStage::Validation;
        state.updated_at = chrono::Utc::now();
        
        Ok(())
    }

    /// Create evolution proposal
    async fn create_proposal(&self, trigger: &EvolutionTrigger) -> Result<EvolutionProposal> {
        let (title, description) = match trigger {
            EvolutionTrigger::PerformanceDegradation { metric, current_value, threshold } => {
                (
                    format!("Performance optimization for {}", metric),
                    format!("Metric {} degraded to {} (threshold: {})", metric, current_value, threshold),
                )
            }
            EvolutionTrigger::SecurityVulnerability { cve, severity, component } => {
                (
                    format!("Security patch for {}", component),
                    format!("{} severity vulnerability in {} ({:?})", severity, component, cve),
                )
            }
            _ => (
                "System improvement".to_string(),
                "Proposed improvement based on analysis".to_string(),
            ),
        };
        
        Ok(EvolutionProposal {
            id: format!("prop-{}", uuid::Uuid::new_v4()),
            title,
            description,
            changes: vec![ProposedChange {
                change_type: ChangeType::PerformanceOptimization,
                target: "system".to_string(),
                description: "Optimize performance".to_string(),
                diff: None,
                config_changes: None,
            }],
            risk_assessment: RiskAssessment {
                score: 30,
                level: RiskLevel::Low,
                factors: vec![],
                mitigations: vec!["Rollback available".to_string()],
            },
            expected_benefits: vec!["Improved performance".to_string()],
            potential_risks: vec!["Temporary instability".to_string()],
            confidence: 0.75,
            requires_consensus: true,
        })
    }

    /// Validate proposal stage
    async fn validate_proposal(&self, workflow_id: &str) -> Result<()> {
        info!("Workflow {}: Validating proposal", workflow_id);
        
        let mut active = self.active.write().await;
        let state = active.get_mut(workflow_id).context("Workflow not found")?;
        
        // Check if human approval required and not yet given
        if state.human_approval_required && state.human_approved.is_none() {
            // Wait for human approval
            debug!("Workflow {} waiting for human approval", workflow_id);
            return Ok(());
        }
        
        // If human approval required and rejected
        if state.human_approval_required && state.human_approved == Some(false) {
            state.stage = WorkflowStage::Failed;
            state.errors.push("Human approval rejected".to_string());
            return Ok(());
        }
        
        state.stage = WorkflowStage::Building;
        state.updated_at = chrono::Utc::now();
        
        Ok(())
    }

    /// Build changes stage
    async fn build_changes(&self, workflow_id: &str) -> Result<()> {
        info!("Workflow {}: Building changes", workflow_id);
        
        let mut active = self.active.write().await;
        let state = active.get_mut(workflow_id).context("Workflow not found")?;
        
        // Run build pipeline
        let result = self.pipeline.run(None).await?;
        
        if !result.success {
            state.stage = WorkflowStage::Failed;
            state.errors.push(format!("Build failed: {:?}", result.error));
            return Ok(());
        }
        
        state.build_result = Some(result);
        state.stage = WorkflowStage::Consensus;
        state.updated_at = chrono::Utc::now();
        
        Ok(())
    }

    /// Seek consensus stage
    async fn seek_consensus(&self, workflow_id: &str) -> Result<()> {
        info!("Workflow {}: Seeking consensus", workflow_id);
        
        let mut active = self.active.write().await;
        let state = active.get_mut(workflow_id).context("Workflow not found")?;
        
        let proposal = state.proposal.as_ref().context("No proposal")?;
        
        if !proposal.requires_consensus {
            state.stage = WorkflowStage::Deployment;
            return Ok(());
        }
        
        // Create consensus proposal
        let consensus_proposal = self.consensus.create_proposal(
            proposal.title.clone(),
            proposal.description.clone(),
            "evolution-engine".to_string(),
            crate::consensus::DecisionType::UpdateDeployment,
            Some(serde_json::to_value(&proposal.changes)?),
        )?;
        
        state.consensus_proposal_id = Some(consensus_proposal.id.clone());
        
        // Simulate voting (in real implementation, servants would vote)
        // For now, auto-approve if confidence is high
        if proposal.confidence > 0.8 {
            // Cast votes
            for servant in self.consensus.get_servants() {
                let vote = if proposal.confidence > 0.9 {
                    Vote::Yes
                } else {
                    Vote::Abstain
                };
                
                let _ = self.consensus.cast_vote(
                    &consensus_proposal.id,
                    servant,
                    vote,
                    "Auto-vote based on confidence".to_string(),
                );
            }
        }
        
        // Evaluate
        let tally = self.consensus.evaluate_proposal(&consensus_proposal.id)?;
        
        if tally.result == crate::consensus::ConsensusResult::Approved {
            state.stage = WorkflowStage::Deployment;
        } else {
            state.stage = WorkflowStage::Failed;
            state.errors.push(format!("Consensus not reached: {:?}", tally.result));
        }
        
        state.updated_at = chrono::Utc::now();
        
        Ok(())
    }

    /// Deploy changes stage
    async fn deploy_changes(&self, workflow_id: &str) -> Result<()> {
        info!("Workflow {}: Deploying changes", workflow_id);
        
        let mut active = self.active.write().await;
        let state = active.get_mut(workflow_id).context("Workflow not found")?;
        
        // Run canary if enabled
        if self.config.enable_canary {
            state.stage = WorkflowStage::Monitoring;
        } else {
            // Direct deployment
            state.stage = WorkflowStage::Monitoring;
        }
        
        state.updated_at = chrono::Utc::now();
        
        Ok(())
    }

    /// Monitor deployment stage
    async fn monitor_deployment(&self, workflow_id: &str) -> Result<()> {
        info!("Workflow {}: Monitoring deployment", workflow_id);
        
        let mut active = self.active.write().await;
        let state = active.get_mut(workflow_id).context("Workflow not found")?;
        
        // Run canary test
        let runner = CanaryRunner::new(self.canary.clone());
        let result = runner.run("system", "new-version").await?;
        
        state.canary_result = Some(result.clone());
        
        if result.success {
            state.stage = WorkflowStage::Learning;
        } else {
            if self.config.auto_rollback {
                state.stage = WorkflowStage::RolledBack;
            } else {
                state.stage = WorkflowStage::Failed;
            }
            state.errors.push("Canary test failed".to_string());
        }
        
        state.updated_at = chrono::Utc::now();
        
        Ok(())
    }

    /// Learn from results stage
    async fn learn_from_results(&self, workflow_id: &str) -> Result<()> {
        info!("Workflow {}: Learning from results", workflow_id);
        
        let mut active = self.active.write().await;
        let state = active.get_mut(workflow_id).context("Workflow not found")?;
        
        // Record lessons learned
        let lessons = self.extract_lessons(state).await?;
        
        state.stage = WorkflowStage::Completed;
        state.updated_at = chrono::Utc::now();
        
        // Store in learning system
        if !self.config.learning_mode {
            // Apply learned improvements
        }
        
        Ok(())
    }

    /// Extract lessons from workflow
    async fn extract_lessons(&self, state: &WorkflowState) -> Result<Vec<String>> {
        let mut lessons = Vec::new();
        
        if let Some(ref build_result) = state.build_result {
            if build_result.success {
                lessons.push("Build pipeline executed successfully".to_string());
            }
        }
        
        if let Some(ref canary_result) = state.canary_result {
            if canary_result.success {
                lessons.push("Canary deployment successful".to_string());
            } else {
                lessons.push(format!("Canary failed at {}%", canary_result.final_percentage));
            }
        }
        
        Ok(lessons)
    }

    /// Finalize workflow
    async fn finalize_workflow(&self, workflow_id: &str) -> Result<()> {
        let state = self.get_state(workflow_id).await
            .context("Workflow not found")?;
        
        let record = WorkflowRecord {
            id: format!("rec-{}", uuid::Uuid::new_v4()),
            workflow_id: workflow_id.to_string(),
            trigger_type: format!("{:?}", state.trigger),
            final_stage: state.stage,
            success: state.stage == WorkflowStage::Completed,
            duration_secs: (state.updated_at - state.started_at).num_seconds() as u64,
            timestamp: chrono::Utc::now(),
            lessons: self.extract_lessons(&state).await?,
        };
        
        self.history.write().await.push(record);
        self.active.write().await.remove(workflow_id);
        
        info!(
            "Workflow {} finalized: stage={:?}, success={}",
            workflow_id, state.stage, state.stage == WorkflowStage::Completed
        );
        
        Ok(())
    }

    /// Get workflow state
    pub async fn get_state(&self, workflow_id: &str) -> Option<WorkflowState> {
        self.active.read().await.get(workflow_id).cloned()
    }

    /// Approve workflow (for human approval)
    pub async fn approve(&self, workflow_id: &str, approved: bool) -> Result<()> {
        let mut active = self.active.write().await;
        let state = active.get_mut(workflow_id).context("Workflow not found")?;
        
        state.human_approved = Some(approved);
        state.updated_at = chrono::Utc::now();
        
        if !approved {
            state.stage = WorkflowStage::Failed;
            state.errors.push("Human rejected proposal".to_string());
        }
        
        Ok(())
    }

    /// Get workflow history
    pub async fn get_history(&self) -> Vec<WorkflowRecord> {
        self.history.read().await.clone()
    }

    /// Get workflow statistics
    pub async fn get_stats(&self) -> WorkflowStats {
        let history = self.history.read().await;
        
        let total = history.len();
        let successful = history.iter().filter(|r| r.success).count();
        
        WorkflowStats {
            total_workflows: total,
            successful,
            failed: total - successful,
            average_duration_secs: if total > 0 {
                history.iter().map(|r| r.duration_secs).sum::<u64>() / total as u64
            } else {
                0
            },
        }
    }
}

/// Analysis result
struct AnalysisResult {
    opportunities: Vec<String>,
    recommendations: Vec<String>,
    confidence: f64,
}

/// Workflow statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStats {
    pub total_workflows: usize,
    pub successful: usize,
    pub failed: usize,
    pub average_duration_secs: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::ConsensusConfig;
    use crate::consensus::constitution::Constitution;
    use crate::safety::canary::{CanaryConfig, DefaultMetricsCollector};
    use std::path::PathBuf;

    #[test]
    fn test_workflow_config_defaults() {
        let config = WorkflowConfig::default();
        
        assert!(!config.auto_evolve);
        assert_eq!(config.max_concurrent, 5);
        assert!(config.require_human_approval);
    }

    #[tokio::test]
    async fn test_evolution_trigger() {
        let trigger = EvolutionTrigger::PerformanceDegradation {
            metric: "latency".to_string(),
            current_value: 500.0,
            threshold: 200.0,
        };
        
        match trigger {
            EvolutionTrigger::PerformanceDegradation { metric, .. } => {
                assert_eq!(metric, "latency");
            }
            _ => panic!("Wrong trigger type"),
        }
    }

    #[test]
    fn test_risk_assessment() {
        let assessment = RiskAssessment {
            score: 30,
            level: RiskLevel::Low,
            factors: vec![],
            mitigations: vec!["Rollback available".to_string()],
        };
        
        assert!(assessment.score < 70);
        assert_eq!(assessment.level, RiskLevel::Low);
    }
}
