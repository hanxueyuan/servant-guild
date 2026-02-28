//! Update Proposal - Self-Evolution Decision Making
//!
//! This module defines update proposals for ServantGuild's self-evolution
//! capabilities. Update proposals are submitted to the consensus engine
//! for collective decision making.
//!
//! Proposal Types:
//! - ModuleUpdate: Update a servant module
//! - ConfigChange: Modify system configuration
//! - BehaviorEvolution: Evolve servant behavior
//! - SecurityPolicy: Update security policies
//! - IntegrationAdd: Add new integration

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::consensus::{DecisionType, Proposal, ProposalStatus};

/// Update proposal type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateType {
    /// Update a servant module
    ModuleUpdate {
        module_id: String,
        from_version: String,
        to_version: String,
    },
    /// Modify system configuration
    ConfigChange {
        config_path: String,
        old_value: Option<serde_json::Value>,
        new_value: serde_json::Value,
    },
    /// Evolve servant behavior
    BehaviorEvolution {
        servant_id: String,
        behavior_changes: Vec<BehaviorChange>,
    },
    /// Update security policies
    SecurityPolicy {
        policy_name: String,
        changes: Vec<PolicyChange>,
    },
    /// Add new integration
    IntegrationAdd {
        integration_name: String,
        config: serde_json::Value,
    },
    /// Rollback to previous version
    Rollback {
        target_snapshot: String,
        reason: String,
    },
}

/// Behavior change specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorChange {
    /// Behavior name
    pub name: String,
    /// Change description
    pub description: String,
    /// Old behavior (if applicable)
    pub old_behavior: Option<String>,
    /// New behavior
    pub new_behavior: String,
    /// Impact assessment
    pub impact: ImpactLevel,
    /// Risk assessment
    pub risk: RiskLevel,
}

/// Impact level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImpactLevel {
    /// Low impact, minor change
    Low,
    /// Medium impact, moderate change
    Medium,
    /// High impact, significant change
    High,
    /// Critical impact, major change
    Critical,
}

/// Risk level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    /// Minimal risk
    Minimal,
    /// Low risk
    Low,
    /// Medium risk
    Medium,
    /// High risk
    High,
    /// Unknown risk, requires investigation
    Unknown,
}

/// Policy change specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyChange {
    /// Policy rule name
    pub rule: String,
    /// Old value
    pub old_value: Option<String>,
    /// New value
    pub new_value: String,
    /// Justification
    pub justification: String,
}

/// Update proposal with full context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProposal {
    /// Base proposal from consensus engine
    pub base: Proposal,
    /// Update type
    pub update_type: UpdateType,
    /// Proposed by servant
    pub proposer_servant: String,
    /// Rationale for the update
    pub rationale: String,
    /// Expected benefits
    pub benefits: Vec<String>,
    /// Potential risks
    pub risks: Vec<String>,
    /// Rollback plan
    pub rollback_plan: Option<RollbackPlan>,
    /// Test results (if any)
    pub test_results: Option<TestResults>,
    /// Confidence score (0-100)
    pub confidence: u8,
    /// Requires immediate execution
    pub urgent: bool,
}

/// Rollback plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackPlan {
    /// Snapshot ID to rollback to
    pub snapshot_id: String,
    /// Rollback steps
    pub steps: Vec<RollbackStep>,
    /// Estimated rollback time (seconds)
    pub estimated_time_secs: u64,
    /// Data loss risk
    pub data_loss_risk: bool,
}

/// Rollback step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackStep {
    /// Step ID
    pub id: String,
    /// Step description
    pub description: String,
    /// Step order
    pub order: u8,
    /// Automated or manual
    pub automated: bool,
}

/// Test results for the proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResults {
    /// Unit test pass rate
    pub unit_test_pass_rate: f64,
    /// Integration test pass rate
    pub integration_test_pass_rate: f64,
    /// Performance benchmark comparison
    pub performance_comparison: Option<PerformanceComparison>,
    /// Security scan results
    pub security_scan: Option<SecurityScanResult>,
    /// Total tests run
    pub total_tests: usize,
    /// Failed tests
    pub failed_tests: Vec<String>,
}

/// Performance comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceComparison {
    /// Metric name
    pub metric: String,
    /// Old value
    pub old_value: f64,
    /// New value
    pub new_value: f64,
    /// Change percentage
    pub change_percent: f64,
    /// Is improvement
    pub improvement: bool,
}

/// Security scan result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScanResult {
    /// Scan passed
    pub passed: bool,
    /// Vulnerabilities found
    pub vulnerabilities: Vec<Vulnerability>,
    /// Scan timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Vulnerability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    /// CVE ID (if applicable)
    pub cve: Option<String>,
    /// Severity
    pub severity: String,
    /// Description
    pub description: String,
    /// Affected component
    pub component: String,
}

impl UpdateProposal {
    /// Create a new update proposal
    pub fn new(
        title: String,
        description: String,
        proposer: String,
        update_type: UpdateType,
    ) -> Self {
        let base = Proposal::new(
            format!("upd-{}", uuid::Uuid::new_v4()),
            title,
            description,
            proposer.clone(),
            DecisionType::UpdateDeployment,
        );
        
        Self {
            base,
            update_type,
            proposer_servant: proposer,
            rationale: String::new(),
            benefits: Vec::new(),
            risks: Vec::new(),
            rollback_plan: None,
            test_results: None,
            confidence: 50,
            urgent: false,
        }
    }

    /// Add rationale
    pub fn with_rationale(mut self, rationale: String) -> Self {
        self.rationale = rationale;
        self
    }

    /// Add benefits
    pub fn with_benefits(mut self, benefits: Vec<String>) -> Self {
        self.benefits = benefits;
        self
    }

    /// Add risks
    pub fn with_risks(mut self, risks: Vec<String>) -> Self {
        self.risks = risks;
        self
    }

    /// Add rollback plan
    pub fn with_rollback_plan(mut self, plan: RollbackPlan) -> Self {
        self.rollback_plan = Some(plan);
        self
    }

    /// Add test results
    pub fn with_test_results(mut self, results: TestResults) -> Self {
        self.test_results = Some(results);
        self
    }

    /// Set confidence
    pub fn with_confidence(mut self, confidence: u8) -> Self {
        self.confidence = confidence.min(100);
        self
    }

    /// Mark as urgent
    pub fn mark_urgent(mut self) -> Self {
        self.urgent = true;
        self
    }

    /// Calculate overall risk score
    pub fn calculate_risk_score(&self) -> u8 {
        let mut score = 0u8;
        
        // Base score from update type
        score += match &self.update_type {
            UpdateType::ModuleUpdate { .. } => 20,
            UpdateType::ConfigChange { .. } => 30,
            UpdateType::BehaviorEvolution { changes } => {
                let max_impact = changes.iter()
                    .map(|c| match c.impact {
                        ImpactLevel::Low => 10,
                        ImpactLevel::Medium => 20,
                        ImpactLevel::High => 30,
                        ImpactLevel::Critical => 40,
                    })
                    .max()
                    .unwrap_or(10);
                max_impact
            }
            UpdateType::SecurityPolicy { .. } => 40,
            UpdateType::IntegrationAdd { .. } => 25,
            UpdateType::Rollback { .. } => 35,
        };
        
        // Add risk from risks list
        score += (self.risks.len() * 5).min(30) as u8;
        
        // Subtract confidence
        score = score.saturating_sub(self.confidence / 5);
        
        // Check rollback plan
        if self.rollback_plan.is_none() {
            score += 10;
        } else if self.rollback_plan.as_ref().map(|r| r.data_loss_risk).unwrap_or(false) {
            score += 15;
        }
        
        // Check test results
        if let Some(ref results) = self.test_results {
            if results.unit_test_pass_rate < 0.95 {
                score += 10;
            }
            if results.integration_test_pass_rate < 0.90 {
                score += 15;
            }
            if !results.failed_tests.is_empty() {
                score += 5;
            }
        } else {
            score += 20; // No tests = higher risk
        }
        
        score.min(100)
    }

    /// Check if proposal is safe to execute
    pub fn is_safe_to_execute(&self) -> bool {
        let risk_score = self.calculate_risk_score();
        
        // High risk requires more approval
        if risk_score > 70 {
            return false;
        }
        
        // Critical updates need test results
        if matches!(self.update_type, UpdateType::SecurityPolicy { .. } | UpdateType::BehaviorEvolution { .. }) {
            if self.test_results.is_none() {
                return false;
            }
        }
        
        // Rollback required for high-risk updates
        if risk_score > 50 && self.rollback_plan.is_none() {
            return false;
        }
        
        true
    }

    /// Get voting weight multiplier based on confidence
    pub fn get_confidence_weight(&self) -> f64 {
        match self.confidence {
            0..=30 => 0.5,
            31..=50 => 0.75,
            51..=70 => 1.0,
            71..=85 => 1.25,
            86..=100 => 1.5,
        }
    }

    /// Convert to consensus proposal
    pub fn into_proposal(self) -> Proposal {
        let payload = serde_json::to_value(&self.update_type).ok();
        
        Proposal {
            id: self.base.id,
            title: self.base.title,
            description: self.base.description,
            proposer: self.base.proposer,
            decision_type: DecisionType::UpdateDeployment,
            payload,
            created_at: self.base.created_at,
            expires_at: self.base.expires_at,
            status: self.base.status,
            votes: self.base.votes,
            tags: self.generate_tags(),
            priority: self.calculate_priority(),
        }
    }

    /// Generate tags based on update type
    fn generate_tags(&self) -> Vec<String> {
        let mut tags = vec!["update".to_string()];
        
        match &self.update_type {
            UpdateType::ModuleUpdate { module_id, .. } => {
                tags.push("module".to_string());
                tags.push(module_id.clone());
            }
            UpdateType::ConfigChange { .. } => {
                tags.push("config".to_string());
            }
            UpdateType::BehaviorEvolution { servant_id, .. } => {
                tags.push("evolution".to_string());
                tags.push(servant_id.clone());
            }
            UpdateType::SecurityPolicy { .. } => {
                tags.push("security".to_string());
            }
            UpdateType::IntegrationAdd { integration_name, .. } => {
                tags.push("integration".to_string());
                tags.push(integration_name.clone());
            }
            UpdateType::Rollback { .. } => {
                tags.push("rollback".to_string());
            }
        }
        
        if self.urgent {
            tags.push("urgent".to_string());
        }
        
        tags
    }

    /// Calculate priority based on various factors
    fn calculate_priority(&self) -> u8 {
        let mut priority = 5u8; // Normal priority
        
        if self.urgent {
            priority = 10;
        } else if matches!(self.update_type, UpdateType::SecurityPolicy { .. }) {
            priority = 9;
        } else if matches!(self.update_type, UpdateType::Rollback { .. }) {
            priority = 8;
        } else if self.confidence > 80 {
            priority = 6;
        } else if self.confidence < 30 {
            priority = 3;
        }
        
        priority
    }
}

/// Proposal builder for convenient construction
pub struct UpdateProposalBuilder {
    title: String,
    description: String,
    proposer: String,
    update_type: UpdateType,
    rationale: Option<String>,
    benefits: Vec<String>,
    risks: Vec<String>,
    rollback_plan: Option<RollbackPlan>,
    test_results: Option<TestResults>,
    confidence: u8,
    urgent: bool,
}

impl UpdateProposalBuilder {
    /// Create a new proposal builder
    pub fn new(title: String, description: String, proposer: String, update_type: UpdateType) -> Self {
        Self {
            title,
            description,
            proposer,
            update_type,
            rationale: None,
            benefits: Vec::new(),
            risks: Vec::new(),
            rollback_plan: None,
            test_results: None,
            confidence: 50,
            urgent: false,
        }
    }

    /// Add rationale
    pub fn rationale(mut self, rationale: String) -> Self {
        self.rationale = Some(rationale);
        self
    }

    /// Add benefit
    pub fn benefit(mut self, benefit: String) -> Self {
        self.benefits.push(benefit);
        self
    }

    /// Add risk
    pub fn risk(mut self, risk: String) -> Self {
        self.risks.push(risk);
        self
    }

    /// Add rollback plan
    pub fn rollback_plan(mut self, plan: RollbackPlan) -> Self {
        self.rollback_plan = Some(plan);
        self
    }

    /// Add test results
    pub fn test_results(mut self, results: TestResults) -> Self {
        self.test_results = Some(results);
        self
    }

    /// Set confidence
    pub fn confidence(mut self, confidence: u8) -> Self {
        self.confidence = confidence.min(100);
        self
    }

    /// Mark as urgent
    pub fn urgent(mut self) -> Self {
        self.urgent = true;
        self
    }

    /// Build the proposal
    pub fn build(self) -> UpdateProposal {
        let mut proposal = UpdateProposal::new(
            self.title,
            self.description,
            self.proposer,
            self.update_type,
        );
        
        if let Some(r) = self.rationale {
            proposal = proposal.with_rationale(r);
        }
        if !self.benefits.is_empty() {
            proposal = proposal.with_benefits(self.benefits);
        }
        if !self.risks.is_empty() {
            proposal = proposal.with_risks(self.risks);
        }
        if let Some(rp) = self.rollback_plan {
            proposal = proposal.with_rollback_plan(rp);
        }
        if let Some(tr) = self.test_results {
            proposal = proposal.with_test_results(tr);
        }
        proposal = proposal.with_confidence(self.confidence);
        if self.urgent {
            proposal = proposal.mark_urgent();
        }
        
        proposal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_proposal_creation() {
        let proposal = UpdateProposal::new(
            "Update Coordinator Module".to_string(),
            "Performance improvements".to_string(),
            "warden-1".to_string(),
            UpdateType::ModuleUpdate {
                module_id: "coordinator".to_string(),
                from_version: "1.0.0".to_string(),
                to_version: "1.1.0".to_string(),
            },
        );
        
        assert_eq!(proposal.proposer_servant, "warden-1");
        assert!(!proposal.urgent);
    }

    #[test]
    fn test_risk_calculation() {
        let low_risk = UpdateProposal::new(
            "Minor Update".to_string(),
            "Small fix".to_string(),
            "worker-1".to_string(),
            UpdateType::ModuleUpdate {
                module_id: "worker".to_string(),
                from_version: "1.0.0".to_string(),
                to_version: "1.0.1".to_string(),
            },
        )
        .with_confidence(90);
        
        let score = low_risk.calculate_risk_score();
        assert!(score < 30);
        
        let high_risk = UpdateProposal::new(
            "Major Behavior Change".to_string(),
            "Complete refactor".to_string(),
            "coordinator-1".to_string(),
            UpdateType::BehaviorEvolution {
                servant_id: "coordinator".to_string(),
                behavior_changes: vec![BehaviorChange {
                    name: "scheduling".to_string(),
                    description: "New scheduling algorithm".to_string(),
                    old_behavior: Some("round-robin".to_string()),
                    new_behavior: "priority-based".to_string(),
                    impact: ImpactLevel::Critical,
                    risk: RiskLevel::High,
                }],
            },
        );
        
        let high_score = high_risk.calculate_risk_score();
        assert!(high_score > 50);
    }

    #[test]
    fn test_safety_check() {
        let safe_proposal = UpdateProposal::new(
            "Safe Update".to_string(),
            "Well tested update".to_string(),
            "warden-1".to_string(),
            UpdateType::ModuleUpdate {
                module_id: "speaker".to_string(),
                from_version: "1.0.0".to_string(),
                to_version: "1.1.0".to_string(),
            },
        )
        .with_confidence(85)
        .with_test_results(TestResults {
            unit_test_pass_rate: 0.98,
            integration_test_pass_rate: 0.95,
            performance_comparison: None,
            security_scan: None,
            total_tests: 100,
            failed_tests: vec![],
        });
        
        assert!(safe_proposal.is_safe_to_execute());
        
        let unsafe_proposal = UpdateProposal::new(
            "Risky Update".to_string(),
            "Untested change".to_string(),
            "warden-1".to_string(),
            UpdateType::SecurityPolicy {
                policy_name: "network".to_string(),
                changes: vec![],
            },
        );
        
        assert!(!unsafe_proposal.is_safe_to_execute());
    }

    #[test]
    fn test_proposal_builder() {
        let proposal = UpdateProposalBuilder::new(
            "Add Slack Integration".to_string(),
            "Enable Slack notifications".to_string(),
            "speaker-1".to_string(),
            UpdateType::IntegrationAdd {
                integration_name: "slack".to_string(),
                config: serde_json::json!({}),
            },
        )
        .rationale("Users need Slack notifications".to_string())
        .benefit("Improved user experience".to_string())
        .risk("API rate limits".to_string())
        .confidence(80)
        .build();
        
        assert_eq!(proposal.benefits.len(), 1);
        assert_eq!(proposal.risks.len(), 1);
        assert_eq!(proposal.confidence, 80);
    }
}
