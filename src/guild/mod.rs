//! Guild - The Multi-Agent Collaboration System
//!
//! The Guild is the central hub that coordinates all Core Servants.
//! It provides a unified interface for:
//! - Starting and stopping all servants
//! - Routing requests to appropriate servants
//! - Managing consensus and voting
//! - Tracking system state
//!
//! ## Architecture
//!
//! ```text
//!                 ┌─────────────────────────────────────┐
//!                 │             Guild                    │
//!                 │  (Multi-Agent Coordination Hub)     │
//!                 └─────────────────────────────────────┘
//!                              │
//!         ┌────────────────────┼────────────────────┐
//!         │                    │                    │
//!         ▼                    ▼                    ▼
//!   ┌───────────┐       ┌───────────┐       ┌───────────┐
//!   │Coordinator│◄──────│ Speaker   │──────►│ Contractor│
//!   │ (Owner)   │       │ (Voice)   │       │ (Builder) │
//!   └─────┬─────┘       └─────┬─────┘       └───────────┘
//!         │                   │
//!         │     ┌─────────────┘
//!         │     │
//!         ▼     ▼
//!   ┌───────────┐       ┌───────────┐
//!   │  Worker   │       │  Warden   │
//!   │ (Hands)   │       │ (Guardian)│
//!   └───────────┘       └───────────┘
//!         │                   │
//!         └─────────┬─────────┘
//!                   │
//!                   ▼
//!            ┌───────────┐
//!            │ Consensus │
//!            │  Engine   │
//!            └───────────┘
//! ```

use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::consensus::{ConsensusConfig, ConsensusEngine, Constitution, DecisionType, Vote};
use crate::servants::{
    Coordinator, Worker, Warden, Speaker, Contractor,
    Servant, ServantId, ServantRole, ServantStatus, ServantError,
};
use crate::safety::{AuditLogger, TransactionManager, SnapshotManager};

// Type aliases for consistency
type AuditLog = AuditLogger;
type RollbackManager = TransactionManager;

/// The Guild - coordinates all servants
pub struct Guild {
    /// Guild ID
    id: GuildId,
    /// Consensus engine
    consensus: Arc<ConsensusEngine>,
    /// Core servants
    coordinator: RwLock<Coordinator>,
    worker: RwLock<Worker>,
    warden: RwLock<Warden>,
    speaker: RwLock<Option<Speaker>>,
    contractor: RwLock<Contractor>,
    /// Guild status
    status: RwLock<GuildStatus>,
    /// Audit log
    audit_log: Option<Arc<AuditLog>>,
    /// Rollback manager
    rollback_manager: Option<Arc<RollbackManager>>,
}

/// Unique identifier for a Guild
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct GuildId(String);

impl GuildId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for GuildId {
    fn default() -> Self {
        Self::new(uuid::Uuid::new_v4().to_string())
    }
}

/// Status of the Guild
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GuildStatus {
    /// Guild is initializing
    Initializing,
    /// All servants are ready
    Ready,
    /// Guild is processing a request
    Processing,
    /// Guild is paused
    Paused,
    /// Guild encountered an error
    Error,
    /// Guild is shutting down
    Stopping,
}

/// Configuration for the Guild
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildConfig {
    /// Consensus configuration
    pub consensus: ConsensusConfig,
    /// Whether to enable audit logging
    pub enable_audit: bool,
    /// Whether to enable rollback capability
    pub enable_rollback: bool,
    /// Maximum concurrent tasks
    pub max_concurrent_tasks: usize,
}

impl Default for GuildConfig {
    fn default() -> Self {
        Self {
            consensus: ConsensusConfig::default(),
            enable_audit: true,
            enable_rollback: true,
            max_concurrent_tasks: 10,
        }
    }
}

/// Result of a Guild operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildResult {
    /// Operation ID
    pub operation_id: String,
    /// Whether successful
    pub success: bool,
    /// Result data
    pub data: serde_json::Value,
    /// Any warnings
    pub warnings: Vec<String>,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

impl Guild {
    /// Create a new Guild with default configuration
    pub async fn new() -> Result<Self, GuildError> {
        Self::with_config(GuildConfig::default()).await
    }
    
    /// Create a Guild with custom configuration
    pub async fn with_config(config: GuildConfig) -> Result<Self, GuildError> {
        // Create consensus engine
        let consensus = Arc::new(ConsensusEngine::with_config(
            config.consensus.clone(),
            Constitution::default(),
        ));
        
        // Register all core servants
        consensus.register_servant("coordinator".to_string());
        consensus.register_servant("worker".to_string());
        consensus.register_servant("warden".to_string());
        consensus.register_servant("speaker".to_string());
        consensus.register_servant("contractor".to_string());
        consensus.set_owner("coordinator".to_string());
        
        // Create servants
        let coordinator = Coordinator::new()
            .with_consensus(consensus.clone());
        
        let worker = Worker::new()
            .with_consensus(consensus.clone());
        
        let warden = Warden::new()
            .with_consensus(consensus.clone());
        
        let speaker = Speaker::new(consensus.clone());
        
        let contractor = Contractor::new()
            .with_consensus(consensus.clone());
        
        // Setup audit log and rollback manager
        let audit_log = if config.enable_audit {
            let zeroclaw_dir = std::path::PathBuf::from("."); // TODO: Get from config
            let audit_config = crate::config::AuditConfig::default();
            Some(Arc::new(AuditLog::new(audit_config, zeroclaw_dir)?))
        } else {
            None
        };
        
        let rollback_manager = if config.enable_rollback {
            let snapshot_path = std::path::PathBuf::from("./snapshots"); // TODO: Get from config
            let snapshot_manager = Arc::new(SnapshotManager::new(snapshot_path)?);
            Some(Arc::new(TransactionManager::new(snapshot_manager)))
        } else {
            None
        };
        
        Ok(Self {
            id: GuildId::default(),
            consensus,
            coordinator: RwLock::new(coordinator),
            worker: RwLock::new(worker),
            warden: RwLock::new(warden),
            speaker: RwLock::new(Some(speaker)),
            contractor: RwLock::new(contractor),
            status: RwLock::new(GuildStatus::Initializing),
            audit_log,
            rollback_manager,
        })
    }
    
    /// Start the Guild and all servants
    pub async fn start(&self) -> Result<(), GuildError> {
        // Start all servants
        self.coordinator.write().start().await
            .map_err(|e| GuildError::ServantError(e))?;
        
        self.worker.write().start().await
            .map_err(|e| GuildError::ServantError(e))?;
        
        self.warden.write().start().await
            .map_err(|e| GuildError::ServantError(e))?;
        
        if let Some(speaker) = self.speaker.write().as_mut() {
            speaker.start().await.map_err(|e| GuildError::ServantError(e))?;
        }
        
        self.contractor.write().start().await
            .map_err(|e| GuildError::ServantError(e))?;
        
        *self.status.write() = GuildStatus::Ready;
        
        Ok(())
    }
    
    /// Stop the Guild gracefully
    pub async fn stop(&self) -> Result<(), GuildError> {
        *self.status.write() = GuildStatus::Stopping;
        
        // Stop all servants
        let _ = self.coordinator.write().stop().await;
        let _ = self.worker.write().stop().await;
        let _ = self.warden.write().stop().await;
        
        if let Some(speaker) = self.speaker.write().as_mut() {
            let _ = speaker.stop().await;
        }
        
        let _ = self.contractor.write().stop().await;
        
        *self.status.write() = GuildStatus::Paused;
        
        Ok(())
    }
    
    /// Process a user request through the Guild
    pub async fn process(&self, request: String) -> Result<GuildResult, GuildError> {
        let start = std::time::Instant::now();
        let operation_id = uuid::Uuid::new_v4().to_string();
        
        // Check if ready
        if *self.status.read() != GuildStatus::Ready {
            return Err(GuildError::NotReady);
        }
        
        *self.status.write() = GuildStatus::Processing;
        
        // 1. Security check with Warden
        let security_check = self.warden.read().check_operation(
            "process_request",
            &serde_json::json!({"request": &request}),
            "user",
        );
        
        if !security_check.allowed {
            *self.status.write() = GuildStatus::Ready;
            return Err(GuildError::SecurityDenied(security_check.reason));
        }
        
        // 2. If high risk, require consensus
        if security_check.requires_approval {
            // Create a proposal
            if let Some(speaker) = self.speaker.read().as_ref() {
                let proposal = speaker.propose(
                    "Process User Request".to_string(),
                    request.clone(),
                    "coordinator".to_string(),
                    DecisionType::CodeChange,
                    None,
                ).await.map_err(|e| GuildError::ServantError(e))?;
                
                // Wait for votes (simplified - in reality would be async)
                // For now, return pending
                *self.status.write() = GuildStatus::Ready;
                
                return Ok(GuildResult {
                    operation_id,
                    success: true,
                    data: serde_json::json!({
                        "status": "pending_approval",
                        "proposal_id": proposal.id,
                        "message": "Request requires guild approval"
                    }),
                    warnings: vec!["High-risk operation requires approval".to_string()],
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        }
        
        // 3. Process through Coordinator
        let result = self.coordinator.read().process_request(request.clone()).await
            .map_err(|e| GuildError::ServantError(e))?;

        // 4. Audit the operation
        if let Some(audit) = &self.audit_log {
            use crate::safety::audit::AuditEvent;
            let event = AuditEvent::new(crate::safety::AuditEventType::Custom("process_request".to_string()))
                .with_result(true, None, 0, Some(format!("{:?}", result)));
            audit.log(&event).ok();
        }
        
        *self.status.write() = GuildStatus::Ready;
        
        Ok(GuildResult {
            operation_id,
            success: true,
            data: serde_json::json!({
                "result": result,
            }),
            warnings: security_check.warnings,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
    
    /// Execute a tool through the Worker
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, GuildError> {
        // Security check
        let security_check = self.warden.read().check_operation(
            tool_name,
            &params,
            "coordinator",
        );
        
        if !security_check.allowed {
            return Err(GuildError::SecurityDenied(security_check.reason));
        }
        
        // Create snapshot for risky operations
        let snapshot_id = if security_check.risk_level > 5 {
            if let Some(manager) = &self.rollback_manager {
                let path = std::path::Path::new(tool_name);
                Some(manager.create_snapshot(path).ok())
            } else {
                None
            }
        } else {
            None
        };
        
        // Execute the tool
        let result = self.worker.read().execute_tool(tool_name, params.clone()).await
            .map_err(|e| GuildError::ServantError(e))?;
        
        if !result.success {
            // Rollback if we created a snapshot
            if let Some(Some(_id)) = snapshot_id {
                if let Some(manager) = &self.rollback_manager {
                    manager.rollback().ok();
                }
            }
            
            return Err(GuildError::ToolExecutionFailed(
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            ));
        }
        
        Ok(result.output)
    }
    
    /// Create a proposal for guild voting
    pub async fn propose(
        &self,
        title: String,
        description: String,
        decision_type: DecisionType,
    ) -> Result<String, GuildError> {
        if let Some(speaker) = self.speaker.read().as_ref() {
            let proposal = speaker.propose(
                title,
                description,
                "coordinator".to_string(),
                decision_type,
                None,
            ).await.map_err(|e| GuildError::ServantError(e))?;
            
            Ok(proposal.id)
        } else {
            Err(GuildError::ServantNotAvailable("speaker".to_string()))
        }
    }
    
    /// Vote on a proposal
    pub async fn vote(
        &self,
        proposal_id: &str,
        voter: ServantRole,
        vote: Vote,
        reason: String,
    ) -> Result<(), GuildError> {
        let voter_id = voter.default_id();
        
        self.consensus.cast_vote(proposal_id, voter_id.to_string(), vote, reason)
            .map_err(|e| GuildError::ConsensusError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Get the current Guild status
    pub fn status(&self) -> GuildStatus {
        self.status.read().clone()
    }
    
    /// Get all servant statuses
    pub fn servant_statuses(&self) -> HashMap<ServantRole, ServantStatus> {
        let mut statuses = HashMap::new();
        statuses.insert(ServantRole::Coordinator, self.coordinator.read().status());
        statuses.insert(ServantRole::Worker, self.worker.read().status());
        statuses.insert(ServantRole::Warden, self.warden.read().status());
        
        if let Some(speaker) = self.speaker.read().as_ref() {
            statuses.insert(ServantRole::Speaker, speaker.status());
        }
        
        statuses.insert(ServantRole::Contractor, self.contractor.read().status());
        statuses
    }
    
    /// Get active proposals
    pub fn get_active_proposals(&self) -> Vec<crate::consensus::Proposal> {
        self.consensus.get_active_proposals()
    }
    
    /// Get the consensus engine
    pub fn consensus(&self) -> Arc<ConsensusEngine> {
        self.consensus.clone()
    }
    
    /// Get the Guild ID
    pub fn id(&self) -> &GuildId {
        &self.id
    }
    
    /// Check if the guild is healthy
    pub fn is_healthy(&self) -> bool {
        let statuses = self.servant_statuses();
        statuses.values().all(|s| *s == ServantStatus::Ready || *s == ServantStatus::Busy)
    }
}

use std::collections::HashMap;

/// Errors that can occur in Guild operations
#[derive(Debug, thiserror::Error)]
pub enum GuildError {
    #[error("Guild is not ready")]
    NotReady,
    
    #[error("Security denied: {0}")]
    SecurityDenied(String),
    
    #[error("Servant error: {0}")]
    ServantError(#[from] ServantError),
    
    #[error("Servant not available: {0}")]
    ServantNotAvailable(String),
    
    #[error("Consensus error: {0}")]
    ConsensusError(String),
    
    #[error("Tool execution failed: {0}")]
    ToolExecutionFailed(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<crate::safety::SafetyError> for GuildError {
    fn from(e: crate::safety::SafetyError) -> Self {
        GuildError::Internal(e.to_string())
    }
}

impl From<anyhow::Error> for GuildError {
    fn from(e: anyhow::Error) -> Self {
        GuildError::Internal(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_guild_creation() {
        let guild = Guild::new().await.unwrap();
        assert_eq!(guild.status(), GuildStatus::Initializing);
    }
    
    #[tokio::test]
    async fn test_guild_start_stop() {
        let guild = Guild::new().await.unwrap();
        
        guild.start().await.unwrap();
        assert_eq!(guild.status(), GuildStatus::Ready);
        
        let statuses = guild.servant_statuses();
        assert_eq!(statuses.get(&ServantRole::Coordinator), Some(&ServantStatus::Ready));
        assert_eq!(statuses.get(&ServantRole::Worker), Some(&ServantStatus::Ready));
        
        guild.stop().await.unwrap();
        assert_eq!(guild.status(), GuildStatus::Paused);
    }
    
    #[tokio::test]
    async fn test_process_request() {
        let guild = Guild::new().await.unwrap();
        guild.start().await.unwrap();
        
        let result = guild.process("Hello, Guild!".to_string()).await.unwrap();
        assert!(result.success);
    }
    
    #[tokio::test]
    async fn test_propose() {
        let guild = Guild::new().await.unwrap();
        guild.start().await.unwrap();
        
        let proposal_id = guild.propose(
            "Test Proposal".to_string(),
            "This is a test".to_string(),
            DecisionType::CodeChange,
        ).await.unwrap();
        
        assert!(!proposal_id.is_empty());
        
        let proposals = guild.get_active_proposals();
        assert_eq!(proposals.len(), 1);
    }
    
    #[tokio::test]
    async fn test_vote() {
        let guild = Guild::new().await.unwrap();
        guild.start().await.unwrap();
        
        // Create a proposal
        let proposal_id = guild.propose(
            "Test".to_string(),
            "Test".to_string(),
            DecisionType::CodeChange,
        ).await.unwrap();
        
        // Vote on it
        guild.vote(&proposal_id, ServantRole::Worker, Vote::Yes, "Approve".to_string()).await.unwrap();
        guild.vote(&proposal_id, ServantRole::Warden, Vote::Yes, "Approve".to_string()).await.unwrap();
        
        // Evaluate
        let tally = guild.consensus().evaluate_proposal(&proposal_id).unwrap();
        assert_eq!(tally.yes_votes, 2);
    }
    
    #[tokio::test]
    async fn test_is_healthy() {
        let guild = Guild::new().await.unwrap();
        
        // Not healthy when initializing
        assert!(!guild.is_healthy());
        
        guild.start().await.unwrap();
        
        // Healthy when all servants are ready
        assert!(guild.is_healthy());
    }
}
