//! Prudent Agency - Safe and Auditable Agent Actions
//!
//! Prudent Agency ensures that all agent actions are:
//! 1. **Audited** - Every action is logged with full context
//! 2. **Approved** - High-risk actions require guild consensus
//! 3. **Snapshotable** - System state is captured before changes
//! 4. **Rollbackable** - Actions can be safely undone
//!
//! ## The Prudent Agency Flow
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Prudent Agency Flow                       │
//! └─────────────────────────────────────────────────────────────┘
//!
//! 1. REQUEST ──► 2. RISK CHECK ──► 3. DECISION
//!                     │                   │
//!                     ▼                   ▼
//!               ┌───────────┐      ┌─────────────┐
//!               │ Low Risk  │      │ High Risk   │
//!               │(Auto-Apro)│      │(Need Vote)  │
//!               └─────┬─────┘      └──────┬──────┘
//!                     │                   │
//!                     ▼                   ▼
//! 4. SNAPSHOT ◄── 4. EXECUTE ◄── 4. CONSENSUS
//!                     │
//!                     ▼
//! 5. AUDIT ◄──────── 5. RESULT
//!                     │
//!                     ▼
//! 6. ROLLBACK (if needed)
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::consensus::{ConsensusEngine, ConsensusResult, DecisionType, Vote};
use crate::safety::{AuditLog, RollbackManager, Snapshot};
use crate::guild::Guild;

/// Prudent Agency Manager
pub struct PrudentAgency {
    /// Consensus engine for approvals
    consensus: Arc<ConsensusEngine>,
    /// Audit log for recording actions
    audit_log: Arc<AuditLog>,
    /// Rollback manager for safe operations
    rollback_manager: Arc<RollbackManager>,
    /// Pending prudent actions
    pending_actions: RwLock<HashMap<String, PendingAction>>,
    /// Action history
    history: RwLock<Vec<ActionRecord>>,
    /// Configuration
    config: PrudentConfig,
}

/// Configuration for Prudent Agency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrudentConfig {
    /// Risk threshold for requiring approval (1-10)
    pub approval_threshold: u8,
    /// Whether to always create snapshots
    pub always_snapshot: bool,
    /// Maximum actions to keep in history
    pub max_history: usize,
    /// Auto-approve routine operations
    pub auto_approve_routine: bool,
    /// Require unanimous approval for critical actions
    pub unanimous_for_critical: bool,
}

impl Default for PrudentConfig {
    fn default() -> Self {
        Self {
            approval_threshold: 5,
            always_snapshot: false,
            max_history: 1000,
            auto_approve_routine: true,
            unanimous_for_critical: true,
        }
    }
}

/// A pending action awaiting approval or execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingAction {
    /// Action ID
    pub id: String,
    /// Type of action
    pub action_type: ActionType,
    /// Risk level (1-10)
    pub risk_level: u8,
    /// Description
    pub description: String,
    /// The actual operation to execute
    pub operation: serde_json::Value,
    /// Who initiated the action
    pub initiator: String,
    /// When created
    pub created_at: DateTime<Utc>,
    /// Current status
    pub status: ActionStatus,
    /// Associated proposal ID (if needed approval)
    pub proposal_id: Option<String>,
    /// Associated snapshot ID
    pub snapshot_id: Option<String>,
    /// Approval status
    pub approval: ApprovalStatus,
}

/// Types of actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    /// File read operation
    FileRead,
    /// File write operation
    FileWrite,
    /// File delete operation
    FileDelete,
    /// Command execution
    CommandExec,
    /// HTTP request
    HttpRequest,
    /// Database query
    DatabaseQuery,
    /// Database write
    DatabaseWrite,
    /// Configuration change
    ConfigChange,
    /// System update
    SystemUpdate,
    /// Custom action
    Custom(String),
}

/// Status of an action
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActionStatus {
    /// Pending risk assessment
    Assessing,
    /// Pending approval
    PendingApproval,
    /// Creating snapshot
    Snapshotting,
    /// Executing
    Executing,
    /// Completed successfully
    Completed,
    /// Failed
    Failed,
    /// Rolled back
    RolledBack,
    /// Rejected
    Rejected,
    /// Cancelled
    Cancelled,
}

/// Approval status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalStatus {
    /// Not required
    NotRequired,
    /// Pending votes
    Pending,
    /// Approved
    Approved,
    /// Rejected
    Rejected,
    /// Owner vetoed
    Vetoed,
}

/// Record of a completed action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRecord {
    /// Action ID
    pub id: String,
    /// Type of action
    pub action_type: ActionType,
    /// Risk level
    pub risk_level: u8,
    /// Description
    pub description: String,
    /// Initiator
    pub initiator: String,
    /// When started
    pub started_at: DateTime<Utc>,
    /// When completed
    pub completed_at: DateTime<Utc>,
    /// Final status
    pub status: ActionStatus,
    /// Result (if successful)
    pub result: Option<serde_json::Value>,
    /// Error (if failed)
    pub error: Option<String>,
    /// Whether rolled back
    pub was_rolled_back: bool,
    /// Audit entry ID
    pub audit_id: String,
}

impl PrudentAgency {
    /// Create a new Prudent Agency manager
    pub fn new(
        consensus: Arc<ConsensusEngine>,
        audit_log: Arc<AuditLog>,
        rollback_manager: Arc<RollbackManager>,
    ) -> Self {
        Self {
            consensus,
            audit_log,
            rollback_manager,
            pending_actions: RwLock::new(HashMap::new()),
            history: RwLock::new(Vec::new()),
            config: PrudentConfig::default(),
        }
    }
    
    /// Configure the Prudent Agency
    pub fn with_config(mut self, config: PrudentConfig) -> Self {
        self.config = config;
        self
    }
    
    /// Initiate a prudent action
    pub async fn initiate(
        &self,
        action_type: ActionType,
        operation: serde_json::Value,
        initiator: String,
        description: String,
    ) -> Result<String, PrudentError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        
        // Calculate risk level
        let risk_level = self.calculate_risk_level(&action_type, &operation);
        
        // Create pending action
        let mut action = PendingAction {
            id: id.clone(),
            action_type,
            risk_level,
            description,
            operation,
            initiator,
            created_at: now,
            status: ActionStatus::Assessing,
            proposal_id: None,
            snapshot_id: None,
            approval: ApprovalStatus::NotRequired,
        };
        
        // Log the initiation
        self.audit_log.log(
            crate::safety::AuditEventType::Custom("action_initiated".to_string()),
            serde_json::json!({
                "action_id": &id,
                "action_type": &action.action_type,
                "risk_level": risk_level,
                "initiator": &action.initiator,
            }),
        )?;
        
        // Determine if approval is needed
        let needs_approval = risk_level >= self.config.approval_threshold;
        
        if needs_approval {
            // Create proposal for voting
            let decision_type = self.action_to_decision_type(&action.action_type);
            
            let proposal = self.consensus.create_proposal(
                format!("Action: {}", action.description),
                action.description.clone(),
                action.initiator.clone(),
                decision_type,
                Some(action.operation.clone()),
            )?;
            
            action.proposal_id = Some(proposal.id.clone());
            action.status = ActionStatus::PendingApproval;
            action.approval = ApprovalStatus::Pending;
        } else {
            // Auto-approve
            action.status = ActionStatus::Snapshotting;
            action.approval = ApprovalStatus::Approved;
        }
        
        self.pending_actions.write().insert(id.clone(), action);
        
        Ok(id)
    }
    
    /// Check if an action is approved and ready to execute
    pub async fn check_approval(&self, action_id: &str) -> Result<ApprovalStatus, PrudentError> {
        let mut pending = self.pending_actions.write();
        let action = pending
            .get_mut(action_id)
            .ok_or_else(|| PrudentError::ActionNotFound(action_id.to_string()))?;
        
        if let Some(proposal_id) = &action.proposal_id {
            let tally = self.consensus.evaluate_proposal(proposal_id)?;
            
            match tally.result {
                ConsensusResult::Passed => {
                    action.approval = ApprovalStatus::Approved;
                    action.status = ActionStatus::Snapshotting;
                }
                ConsensusResult::Rejected => {
                    action.approval = ApprovalStatus::Rejected;
                    action.status = ActionStatus::Rejected;
                }
                ConsensusResult::Vetoed => {
                    action.approval = ApprovalStatus::Vetoed;
                    action.status = ActionStatus::Rejected;
                }
                ConsensusResult::Expired => {
                    action.status = ActionStatus::Cancelled;
                }
                ConsensusResult::Pending => {
                    // Still waiting
                }
            }
        }
        
        Ok(action.approval.clone())
    }
    
    /// Execute an approved action
    pub async fn execute(
        &self,
        action_id: &str,
        executor: impl FnOnce(&serde_json::Value) -> Result<serde_json::Value, String>,
    ) -> Result<serde_json::Value, PrudentError> {
        let mut pending = self.pending_actions.write();
        let action = pending
            .get_mut(action_id)
            .ok_or_else(|| PrudentError::ActionNotFound(action_id.to_string()))?;
        
        // Verify approval
        match action.approval {
            ApprovalStatus::Approved => {}
            ApprovalStatus::NotRequired => {}
            _ => return Err(PrudentError::NotApproved),
        }
        
        // Create snapshot for risky operations
        if action.risk_level >= 5 || self.config.always_snapshot {
            let snapshot_id = self.rollback_manager.create_snapshot(&action.description)?;
            action.snapshot_id = Some(snapshot_id);
        }
        
        action.status = ActionStatus::Executing;
        
        // Log execution start
        self.audit_log.log(
            crate::safety::AuditEventType::Custom("action_executing".to_string()),
            serde_json::json!({
                "action_id": action_id,
                "operation": &action.operation,
            }),
        )?;
        
        // Execute the operation
        let started_at = Utc::now();
        let result = executor(&action.operation);
        
        let completed_at = Utc::now();
        
        match result {
            Ok(data) => {
                action.status = ActionStatus::Completed;
                
                // Log success
                self.audit_log.log(
                    crate::safety::AuditEventType::Custom("action_completed".to_string()),
                    serde_json::json!({
                        "action_id": action_id,
                        "duration_ms": (completed_at - started_at).num_milliseconds(),
                    }),
                )?;
                
                // Move to history
                let record = ActionRecord {
                    id: action.id.clone(),
                    action_type: action.action_type.clone(),
                    risk_level: action.risk_level,
                    description: action.description.clone(),
                    initiator: action.initiator.clone(),
                    started_at,
                    completed_at,
                    status: ActionStatus::Completed,
                    result: Some(data.clone()),
                    error: None,
                    was_rolled_back: false,
                    audit_id: action_id.to_string(),
                };
                
                self.add_to_history(record);
                pending.remove(action_id);
                
                Ok(data)
            }
            Err(error) => {
                action.status = ActionStatus::Failed;
                
                // Log failure
                self.audit_log.log(
                    crate::safety::AuditEventType::Custom("action_failed".to_string()),
                    serde_json::json!({
                        "action_id": action_id,
                        "error": &error,
                    }),
                )?;
                
                // Attempt rollback if we have a snapshot
                if let Some(snapshot_id) = &action.snapshot_id {
                    if self.rollback_manager.rollback(snapshot_id).is_ok() {
                        action.status = ActionStatus::RolledBack;
                        
                        self.audit_log.log(
                            crate::safety::AuditEventType::Custom("action_rolled_back".to_string()),
                            serde_json::json!({
                                "action_id": action_id,
                                "snapshot_id": snapshot_id,
                            }),
                        )?;
                    }
                }
                
                // Move to history
                let record = ActionRecord {
                    id: action.id.clone(),
                    action_type: action.action_type.clone(),
                    risk_level: action.risk_level,
                    description: action.description.clone(),
                    initiator: action.initiator.clone(),
                    started_at,
                    completed_at,
                    status: action.status.clone(),
                    result: None,
                    error: Some(error.clone()),
                    was_rolled_back: action.status == ActionStatus::RolledBack,
                    audit_id: action_id.to_string(),
                };
                
                self.add_to_history(record);
                pending.remove(action_id);
                
                Err(PrudentError::ExecutionFailed(error))
            }
        }
    }
    
    /// Rollback an action (if possible)
    pub async fn rollback(&self, action_id: &str) -> Result<(), PrudentError> {
        // Check history for snapshot
        let history = self.history.read();
        let record = history
            .iter()
            .find(|r| r.id == action_id)
            .ok_or_else(|| PrudentError::ActionNotFound(action_id.to_string()))?;
        
        // Can only rollback if we have a snapshot and haven't already rolled back
        if record.was_rolled_back {
            return Err(PrudentError::AlreadyRolledBack);
        }
        
        // Find the snapshot from pending actions
        drop(history);
        let pending = self.pending_actions.read();
        if let Some(action) = pending.get(action_id) {
            if let Some(snapshot_id) = &action.snapshot_id {
                self.rollback_manager.rollback(snapshot_id)?;
                
                self.audit_log.log(
                    crate::safety::AuditEventType::Custom("manual_rollback".to_string()),
                    serde_json::json!({
                        "action_id": action_id,
                        "snapshot_id": snapshot_id,
                    }),
                )?;
            }
        }
        
        Ok(())
    }
    
    /// Get pending actions
    pub fn get_pending_actions(&self) -> Vec<PendingAction> {
        self.pending_actions.read().values().cloned().collect()
    }
    
    /// Get action history
    pub fn get_history(&self) -> Vec<ActionRecord> {
        self.history.read().clone()
    }
    
    /// Calculate risk level for an action
    fn calculate_risk_level(&self, action_type: &ActionType, operation: &serde_json::Value) -> u8 {
        match action_type {
            ActionType::FileRead => 1,
            ActionType::FileWrite => {
                // Check if it's a critical file
                if let Some(path) = operation.get("path").and_then(|p| p.as_str()) {
                    if path.contains(".env") || path.contains("secret") || path.contains("credential") {
                        return 9;
                    }
                }
                5
            }
            ActionType::FileDelete => {
                // Check if it's a critical file
                if let Some(path) = operation.get("path").and_then(|p| p.as_str()) {
                    if path.contains(".env") || path.contains("secret") || path.contains("credential") {
                        return 10;
                    }
                }
                8
            }
            ActionType::CommandExec => {
                // Check if it's a dangerous command
                if let Some(cmd) = operation.get("command").and_then(|c| c.as_str()) {
                    let dangerous = ["rm", "dd", "mkfs", "fdisk", "shutdown", "reboot", "sudo"];
                    for d in dangerous {
                        if cmd.contains(d) {
                            return 10;
                        }
                    }
                }
                7
            }
            ActionType::HttpRequest => 4,
            ActionType::DatabaseQuery => 3,
            ActionType::DatabaseWrite => 6,
            ActionType::ConfigChange => 5,
            ActionType::SystemUpdate => 9,
            ActionType::Custom(_) => 5,
        }
    }
    
    /// Convert action type to decision type
    fn action_to_decision_type(&self, action_type: &ActionType) -> DecisionType {
        match action_type {
            ActionType::FileRead => DecisionType::RoutineOperation,
            ActionType::FileWrite => DecisionType::CodeChange,
            ActionType::FileDelete => DecisionType::CodeChange,
            ActionType::CommandExec => DecisionType::CodeChange,
            ActionType::HttpRequest => DecisionType::RoutineOperation,
            ActionType::DatabaseQuery => DecisionType::RoutineOperation,
            ActionType::DatabaseWrite => DecisionType::ConfigChange,
            ActionType::ConfigChange => DecisionType::ConfigChange,
            ActionType::SystemUpdate => DecisionType::SystemUpdate,
            ActionType::Custom(s) => DecisionType::Custom(s.clone()),
        }
    }
    
    /// Add a record to history (with size limit)
    fn add_to_history(&self, record: ActionRecord) {
        let mut history = self.history.write();
        history.push(record);
        
        // Trim if over limit
        while history.len() > self.config.max_history {
            history.remove(0);
        }
    }
}

/// Errors in Prudent Agency
#[derive(Debug, thiserror::Error)]
pub enum PrudentError {
    #[error("Action not found: {0}")]
    ActionNotFound(String),
    
    #[error("Action not approved")]
    NotApproved,
    
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Already rolled back")]
    AlreadyRolledBack,
    
    #[error("Safety error: {0}")]
    Safety(#[from] crate::safety::SafetyError),
    
    #[error("Consensus error: {0}")]
    Consensus(#[from] anyhow::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_prudent() -> PrudentAgency {
        let consensus = Arc::new(ConsensusEngine::new());
        consensus.register_servant("coordinator".to_string());
        consensus.register_servant("worker".to_string());
        consensus.register_servant("warden".to_string());
        consensus.register_servant("speaker".to_string());
        consensus.register_servant("contractor".to_string());
        consensus.set_owner("coordinator".to_string());
        
        let audit_log = Arc::new(AuditLog::new().unwrap());
        let rollback_manager = Arc::new(RollbackManager::new().unwrap());
        
        PrudentAgency::new(consensus, audit_log, rollback_manager)
    }

    #[tokio::test]
    async fn test_initiate_low_risk_action() {
        let prudent = setup_prudent();
        
        let action_id = prudent.initiate(
            ActionType::FileRead,
            serde_json::json!({"path": "/test.txt"}),
            "worker".to_string(),
            "Read test file".to_string(),
        ).await.unwrap();
        
        let pending = prudent.get_pending_actions();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, action_id);
        assert!(pending[0].risk_level < 5);
    }
    
    #[tokio::test]
    async fn test_initiate_high_risk_action() {
        let prudent = setup_prudent();
        
        let action_id = prudent.initiate(
            ActionType::FileDelete,
            serde_json::json!({"path": "/important.txt"}),
            "worker".to_string(),
            "Delete important file".to_string(),
        ).await.unwrap();
        
        let pending = prudent.get_pending_actions();
        assert_eq!(pending[0].risk_level, 8);
        assert!(pending[0].proposal_id.is_some());
    }
    
    #[tokio::test]
    async fn test_execute_approved_action() {
        let prudent = setup_prudent();
        
        let action_id = prudent.initiate(
            ActionType::FileRead,
            serde_json::json!({"path": "/test.txt"}),
            "worker".to_string(),
            "Read test file".to_string(),
        ).await.unwrap();
        
        let result = prudent.execute(&action_id, |op| {
            Ok(serde_json::json!({
                "content": "Hello, World!",
                "path": op["path"]
            }))
        }).await;
        
        assert!(result.is_ok());
        assert!(prudent.get_pending_actions().is_empty());
        assert!(!prudent.get_history().is_empty());
    }
    
    #[tokio::test]
    async fn test_execute_failed_with_rollback() {
        let prudent = setup_prudent();
        
        let action_id = prudent.initiate(
            ActionType::FileWrite,
            serde_json::json!({"path": "/test.txt", "content": "test"}),
            "worker".to_string(),
            "Write test file".to_string(),
        ).await.unwrap();
        
        let result = prudent.execute(&action_id, |_| {
            Err("Simulated failure".to_string())
        }).await;
        
        assert!(result.is_err());
        
        let history = prudent.get_history();
        assert_eq!(history[0].status, ActionStatus::RolledBack);
        assert!(history[0].was_rolled_back);
    }
    
    #[tokio::test]
    async fn test_risk_level_calculation() {
        let prudent = setup_prudent();
        
        // Low risk
        let id = prudent.initiate(
            ActionType::FileRead,
            serde_json::json!({"path": "/safe.txt"}),
            "worker".to_string(),
            "Read".to_string(),
        ).await.unwrap();
        assert_eq!(prudent.get_pending_actions()[0].risk_level, 1);
        
        // Medium risk
        let id = prudent.initiate(
            ActionType::FileWrite,
            serde_json::json!({"path": "/safe.txt"}),
            "worker".to_string(),
            "Write".to_string(),
        ).await.unwrap();
        assert_eq!(prudent.get_pending_actions().iter().find(|a| a.id == id).unwrap().risk_level, 5);
        
        // Critical
        let id = prudent.initiate(
            ActionType::FileDelete,
            serde_json::json!({"path": "/project/.env"}),
            "worker".to_string(),
            "Delete env".to_string(),
        ).await.unwrap();
        assert_eq!(prudent.get_pending_actions().iter().find(|a| a.id == id).unwrap().risk_level, 10);
    }
}
