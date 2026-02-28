//! Coordinator Servant - Task Distribution and Workflow Orchestration
//!
//! The Coordinator is the "brain" of the guild, responsible for:
//! - Receiving and parsing user requests
//! - Breaking down complex tasks into sub-tasks
//! - Distributing work to appropriate servants
//! - Tracking task progress and dependencies
//! - Aggregating results

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::{
    Servant, ServantId, ServantRole, ServantStatus, ServantTask, ServantResult, ServantError,
};
use crate::consensus::{ConsensusEngine, DecisionType, Vote};

/// The Coordinator servant
pub struct Coordinator {
    /// Unique ID
    id: ServantId,
    /// Current status
    status: RwLock<ServantStatus>,
    /// Consensus engine reference
    consensus: Option<Arc<ConsensusEngine>>,
    /// Active tasks being coordinated
    active_tasks: RwLock<HashMap<String, CoordinatedTask>>,
    /// Task history
    task_history: RwLock<Vec<CompletedTask>>,
}

/// A task being coordinated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatedTask {
    /// Task ID
    pub id: String,
    /// Original user request
    pub request: String,
    /// Sub-tasks generated
    pub sub_tasks: Vec<SubTask>,
    /// Current status
    pub status: CoordinationStatus,
    /// When coordination started
    pub started_at: DateTime<Utc>,
    /// Results from completed sub-tasks
    pub results: HashMap<String, serde_json::Value>,
}

/// A sub-task assigned to a worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    /// Sub-task ID
    pub id: String,
    /// Parent task ID
    pub parent_id: String,
    /// Type of work
    pub work_type: String,
    /// Instructions for the worker
    pub instructions: String,
    /// Assigned servant (if any)
    pub assignee: Option<String>,
    /// Status
    pub status: SubTaskStatus,
    /// Dependencies on other sub-tasks
    pub dependencies: Vec<String>,
}

/// Status of a coordinated task
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CoordinationStatus {
    /// Analyzing request
    Analyzing,
    /// Decomposing into sub-tasks
    Decomposing,
    /// Assigning to workers
    Assigning,
    /// Waiting for results
    Waiting,
    /// Aggregating results
    Aggregating,
    /// Completed successfully
    Completed,
    /// Failed
    Failed,
}

/// Status of a sub-task
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SubTaskStatus {
    /// Not yet assigned
    Pending,
    /// Assigned but not started
    Assigned,
    /// In progress
    InProgress,
    /// Completed successfully
    Completed,
    /// Failed
    Failed,
    /// Blocked by dependencies
    Blocked,
}

/// A completed task record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedTask {
    /// Task ID
    pub id: String,
    /// Original request
    pub request: String,
    /// Final result
    pub result: serde_json::Value,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Number of sub-tasks
    pub sub_task_count: usize,
    /// When completed
    pub completed_at: DateTime<Utc>,
}

impl Coordinator {
    /// Create a new Coordinator
    pub fn new() -> Self {
        Self {
            id: ServantId::new(ServantRole::Coordinator.default_id()),
            status: RwLock::new(ServantStatus::Starting),
            consensus: None,
            active_tasks: RwLock::new(HashMap::new()),
            task_history: RwLock::new(Vec::new()),
        }
    }
    
    /// Set the consensus engine
    pub fn with_consensus(mut self, consensus: Arc<ConsensusEngine>) -> Self {
        self.consensus = Some(consensus);
        self
    }
    
    /// Process a user request
    pub async fn process_request(&self, request: String) -> Result<String, ServantError> {
        let start = std::time::Instant::now();
        
        // Check if ready
        if *self.status.read() != ServantStatus::Ready {
            return Err(ServantError::NotReady("Coordinator not ready".to_string()));
        }
        
        // Mark as busy
        *self.status.write() = ServantStatus::Busy;
        
        // Create coordinated task
        let task_id = uuid::Uuid::new_v4().to_string();
        let coordinated_task = CoordinatedTask {
            id: task_id.clone(),
            request: request.clone(),
            sub_tasks: Vec::new(),
            status: CoordinationStatus::Analyzing,
            started_at: Utc::now(),
            results: HashMap::new(),
        };
        
        self.active_tasks.write().insert(task_id.clone(), coordinated_task);
        
        // TODO: Implement actual LLM-based analysis and decomposition
        // For now, return a simple response
        
        // Mark as ready again
        *self.status.write() = ServantStatus::Ready;
        
        Ok(format!("Processed request: {}", request))
    }
    
    /// Decompose a task into sub-tasks
    pub async fn decompose_task(&self, task_id: &str) -> Result<Vec<SubTask>, ServantError> {
        let mut tasks = self.active_tasks.write();
        let coordinated = tasks
            .get_mut(task_id)
            .ok_or_else(|| ServantError::InvalidTask(format!("Task {} not found", task_id)))?;
        
        coordinated.status = CoordinationStatus::Decomposing;
        
        // TODO: Implement actual decomposition using LLM
        // For now, create a placeholder sub-task
        
        let sub_task = SubTask {
            id: format!("{}-1", task_id),
            parent_id: task_id.to_string(),
            work_type: "analyze".to_string(),
            instructions: coordinated.request.clone(),
            assignee: None,
            status: SubTaskStatus::Pending,
            dependencies: Vec::new(),
        };
        
        coordinated.sub_tasks.push(sub_task.clone());
        coordinated.status = CoordinationStatus::Assigning;
        
        Ok(coordinated.sub_tasks.clone())
    }
    
    /// Assign sub-tasks to workers
    pub async fn assign_sub_tasks(&self, task_id: &str) -> Result<(), ServantError> {
        let tasks = self.active_tasks.read();
        let coordinated = tasks
            .get(task_id)
            .ok_or_else(|| ServantError::InvalidTask(format!("Task {} not found", task_id)))?;
        
        // TODO: Implement smart assignment based on worker capabilities
        
        Ok(())
    }
    
    /// Record a sub-task result
    pub async fn record_result(
        &self,
        task_id: &str,
        sub_task_id: &str,
        result: serde_json::Value,
    ) -> Result<(), ServantError> {
        let mut tasks = self.active_tasks.write();
        let coordinated = tasks
            .get_mut(task_id)
            .ok_or_else(|| ServantError::InvalidTask(format!("Task {} not found", task_id)))?;
        
        // Update sub-task status
        if let Some(sub_task) = coordinated.sub_tasks.iter_mut().find(|s| s.id == sub_task_id) {
            sub_task.status = SubTaskStatus::Completed;
        }
        
        // Store result
        coordinated.results.insert(sub_task_id.to_string(), result);
        
        // Check if all sub-tasks complete
        let all_complete = coordinated.sub_tasks.iter().all(|s| s.status == SubTaskStatus::Completed);
        
        if all_complete {
            coordinated.status = CoordinationStatus::Aggregating;
        }
        
        Ok(())
    }
    
    /// Get active tasks
    pub fn get_active_tasks(&self) -> Vec<CoordinatedTask> {
        self.active_tasks.read().values().cloned().collect()
    }
    
    /// Get task history
    pub fn get_task_history(&self) -> Vec<CompletedTask> {
        self.task_history.read().clone()
    }
    
    /// Vote on a proposal (as owner, has veto power)
    pub async fn vote_on_proposal(
        &self,
        proposal_id: &str,
        vote: Vote,
        reason: String,
    ) -> Result<(), ServantError> {
        if let Some(consensus) = &self.consensus {
            consensus.cast_vote(proposal_id, self.id.as_str().to_string(), vote, reason)
                .map_err(|e| ServantError::Internal(e.to_string()))?;
        }
        Ok(())
    }
    
    /// Veto a proposal (owner privilege)
    pub async fn veto_proposal(&self, proposal_id: &str) -> Result<(), ServantError> {
        if let Some(consensus) = &self.consensus {
            consensus.veto_proposal(proposal_id, self.id.as_str())
                .map_err(|e| ServantError::Internal(e.to_string()))?;
        }
        Ok(())
    }
}

impl Default for Coordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Servant for Coordinator {
    fn id(&self) -> &ServantId {
        &self.id
    }
    
    fn role(&self) -> ServantRole {
        ServantRole::Coordinator
    }
    
    fn status(&self) -> ServantStatus {
        self.status.read().clone()
    }
    
    async fn start(&mut self) -> Result<(), ServantError> {
        *self.status.write() = ServantStatus::Ready;
        Ok(())
    }
    
    async fn stop(&mut self) -> Result<(), ServantError> {
        *self.status.write() = ServantStatus::Stopping;
        // Wait for active tasks to complete
        // TODO: Implement graceful shutdown
        *self.status.write() = ServantStatus::Paused;
        Ok(())
    }
    
    fn capabilities(&self) -> Vec<String> {
        vec![
            "task_decomposition".to_string(),
            "task_distribution".to_string(),
            "result_aggregation".to_string(),
            "workflow_orchestration".to_string(),
            "proposal_veto".to_string(), // Owner privilege
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_coordinator_creation() {
        let coordinator = Coordinator::new();
        assert_eq!(coordinator.role(), ServantRole::Coordinator);
        assert_eq!(coordinator.status(), ServantStatus::Starting);
    }
    
    #[tokio::test]
    async fn test_coordinator_start_stop() {
        let mut coordinator = Coordinator::new();
        
        coordinator.start().await.unwrap();
        assert_eq!(coordinator.status(), ServantStatus::Ready);
        
        coordinator.stop().await.unwrap();
        assert_eq!(coordinator.status(), ServantStatus::Paused);
    }
    
    #[tokio::test]
    async fn test_process_request() {
        let mut coordinator = Coordinator::new();
        coordinator.start().await.unwrap();
        
        let result = coordinator.process_request("Test request".to_string()).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_decompose_task() {
        let mut coordinator = Coordinator::new();
        coordinator.start().await.unwrap();
        
        // First create a task
        let task_id = uuid::Uuid::new_v4().to_string();
        coordinator.active_tasks.write().insert(
            task_id.clone(),
            CoordinatedTask {
                id: task_id.clone(),
                request: "Test".to_string(),
                sub_tasks: Vec::new(),
                status: CoordinationStatus::Analyzing,
                started_at: Utc::now(),
                results: HashMap::new(),
            },
        );
        
        let sub_tasks = coordinator.decompose_task(&task_id).await.unwrap();
        assert!(!sub_tasks.is_empty());
    }
}
