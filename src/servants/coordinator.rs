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
    
    /// Process a user request with full coordination workflow
    pub async fn process_request(&self, request: String) -> Result<String, ServantError> {
        // Check if ready
        if *self.status.read() != ServantStatus::Ready {
            return Err(ServantError::NotReady("Coordinator not ready".to_string()));
        }
        
        // Mark as busy
        *self.status.write() = ServantStatus::Busy;
        
        println!("[Coordinator] Processing request: {}", request);
        
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
        
        // Step 1: Decompose the task
        let sub_tasks = self.decompose_task(&task_id).await?;
        println!("[Coordinator] Created {} sub-tasks", sub_tasks.len());
        
        // Step 2: Assign sub-tasks to workers
        self.assign_subtasks(&task_id).await?;
        
        // Step 3: Simulate execution (in real implementation, workers would execute these)
        for sub_task in sub_tasks.iter() {
            let mut tasks = self.active_tasks.write();
            let coordinated = tasks.get_mut(&task_id).unwrap();
            
            // Find and update the sub-task
            for st in &mut coordinated.sub_tasks {
                if st.id == sub_task.id {
                    st.status = SubTaskStatus::Completed;
                    coordinated.results.insert(
                        st.id.clone(),
                        serde_json::json!({
                            "status": "success",
                            "result": format!("Executed: {}", st.instructions)
                        })
                    );
                    break;
                }
            }
        }
        
        // Step 4: Aggregate results
        let final_result = self.aggregate_results(&task_id).await?;
        
        // Mark as ready again
        *self.status.write() = ServantStatus::Ready;
        
        let result_str = serde_json::to_string_pretty(&final_result)
            .unwrap_or_else(|_| "Failed to serialize result".to_string());
        
        println!("[Coordinator] Task completed successfully");
        
        Ok(result_str)
    }
    
    /// Decompose a task into sub-tasks using intelligent analysis
    pub async fn decompose_task(&self, task_id: &str) -> Result<Vec<SubTask>, ServantError> {
        let mut tasks = self.active_tasks.write();
        let coordinated = tasks
            .get_mut(task_id)
            .ok_or_else(|| ServantError::InvalidTask(format!("Task {} not found", task_id)))?;
        
        coordinated.status = CoordinationStatus::Decomposing;
        
        // Analyze the request and decompose accordingly
        let request = &coordinated.request;
        let sub_tasks = self.analyze_and_decompose(request, task_id)?;
        
        coordinated.sub_tasks = sub_tasks.clone();
        coordinated.status = CoordinationStatus::Assigning;
        
        println!("[Coordinator] Decomposed task '{}' into {} sub-tasks", request, sub_tasks.len());
        
        Ok(sub_tasks)
    }
    
    /// Analyze a request and decompose it into sub-tasks
    fn analyze_and_decompose(&self, request: &str, task_id: &str) -> Result<Vec<SubTask>, ServantError> {
        let request_lower = request.to_lowercase();
        let mut sub_tasks = Vec::new();
        
        // Analyze task complexity and create appropriate sub-tasks
        if request_lower.contains("update") && request_lower.contains("readme") {
            // Task: Update README
            sub_tasks.push(SubTask {
                id: format!("{}-1", task_id),
                parent_id: task_id.to_string(),
                work_type: "read".to_string(),
                instructions: "Read the current README.md file to understand its structure".to_string(),
                assignee: Some("worker".to_string()),
                status: SubTaskStatus::Pending,
                dependencies: Vec::new(),
            });
            
            sub_tasks.push(SubTask {
                id: format!("{}-2", task_id),
                parent_id: task_id.to_string(),
                work_type: "modify".to_string(),
                instructions: "Modify README.md to include new feature documentation".to_string(),
                assignee: Some("worker".to_string()),
                status: SubTaskStatus::Pending,
                dependencies: vec![format!("{}-1", task_id)],
            });
            
            sub_tasks.push(SubTask {
                id: format!("{}-3", task_id),
                parent_id: task_id.to_string(),
                work_type: "verify".to_string(),
                instructions: "Verify the changes are correct and complete".to_string(),
                assignee: Some("worker".to_string()),
                status: SubTaskStatus::Pending,
                dependencies: vec![format!("{}-2", task_id)],
            });
        } else if request_lower.contains("bug") && request_lower.contains("fix") {
            // Task: Fix a bug
            sub_tasks.push(SubTask {
                id: format!("{}-1", task_id),
                parent_id: task_id.to_string(),
                work_type: "investigate".to_string(),
                instructions: "Investigate the bug by reading relevant code and error logs".to_string(),
                assignee: Some("worker".to_string()),
                status: SubTaskStatus::Pending,
                dependencies: Vec::new(),
            });
            
            sub_tasks.push(SubTask {
                id: format!("{}-2", task_id),
                parent_id: task_id.to_string(),
                work_type: "fix".to_string(),
                instructions: "Implement the bug fix".to_string(),
                assignee: Some("worker".to_string()),
                status: SubTaskStatus::Pending,
                dependencies: vec![format!("{}-1", task_id)],
            });
            
            sub_tasks.push(SubTask {
                id: format!("{}-3", task_id),
                parent_id: task_id.to_string(),
                work_type: "test".to_string(),
                instructions: "Test the fix to ensure it works correctly".to_string(),
                assignee: Some("worker".to_string()),
                status: SubTaskStatus::Pending,
                dependencies: vec![format!("{}-2", task_id)],
            });
        } else if request_lower.contains("test") || request_lower.contains("verify") {
            // Task: Run tests
            sub_tasks.push(SubTask {
                id: format!("{}-1", task_id),
                parent_id: task_id.to_string(),
                work_type: "test".to_string(),
                instructions: request.to_string(),
                assignee: Some("worker".to_string()),
                status: SubTaskStatus::Pending,
                dependencies: Vec::new(),
            });
        } else {
            // Generic task - single sub-task
            sub_tasks.push(SubTask {
                id: format!("{}-1", task_id),
                parent_id: task_id.to_string(),
                work_type: "execute".to_string(),
                instructions: request.to_string(),
                assignee: Some("worker".to_string()),
                status: SubTaskStatus::Pending,
                dependencies: Vec::new(),
            });
        }
        
        Ok(sub_tasks)
    }
    
    /// Assign sub-tasks to available servants
    pub async fn assign_subtasks(&self, task_id: &str) -> Result<(), ServantError> {
        let mut tasks = self.active_tasks.write();
        let coordinated = tasks
            .get_mut(task_id)
            .ok_or_else(|| ServantError::InvalidTask(format!("Task {} not found", task_id)))?;
        
        coordinated.status = CoordinationStatus::Assigning;
        
        // Assign each sub-task to an appropriate servant
        for sub_task in &mut coordinated.sub_tasks {
            if sub_task.status == SubTaskStatus::Pending {
                // Check dependencies are satisfied
                let deps_satisfied = sub_task.dependencies.iter().all(|dep_id| {
                    coordinated.sub_tasks.iter().any(|st| st.id == *dep_id && st.status == SubTaskStatus::Completed)
                });
                
                if deps_satisfied {
                    // Assign to worker for now
                    sub_task.assignee = Some("worker".to_string());
                    sub_task.status = SubTaskStatus::Assigned;
                    println!("[Coordinator] Assigned sub-task '{}' to worker", sub_task.id);
                } else {
                    sub_task.status = SubTaskStatus::Blocked;
                }
            }
        }
        
        coordinated.status = CoordinationStatus::Waiting;
        Ok(())
    }
    
    /// Aggregate results from completed sub-tasks
    pub async fn aggregate_results(&self, task_id: &str) -> Result<serde_json::Value, ServantError> {
        let mut tasks = self.active_tasks.write();
        let coordinated = tasks
            .get_mut(task_id)
            .ok_or_else(|| ServantError::InvalidTask(format!("Task {} not found", task_id)))?;
        
        coordinated.status = CoordinationStatus::Aggregating;
        
        // Combine all sub-task results
        let all_completed = coordinated.sub_tasks.iter()
            .all(|st| st.status == SubTaskStatus::Completed || st.status == SubTaskStatus::Failed);
        
        if all_completed {
            let aggregated = serde_json::json!({
                "task_id": task_id,
                "request": coordinated.request,
                "sub_tasks": coordinated.sub_tasks,
                "results": coordinated.results,
                "status": "completed"
            });
            
            coordinated.status = CoordinationStatus::Completed;
            
            // Move to history
            let completed = CompletedTask {
                id: task_id.to_string(),
                request: coordinated.request.clone(),
                result: aggregated.clone(),
                duration_ms: (Utc::now() - coordinated.started_at).num_milliseconds().max(0) as u64,
                sub_task_count: coordinated.sub_tasks.len(),
                completed_at: Utc::now(),
            };
            
            self.task_history.write().push(completed);
            
            Ok(aggregated)
        } else {
            coordinated.status = CoordinationStatus::Waiting;
            Ok(serde_json::json!({
                "status": "in_progress",
                "completed": coordinated.sub_tasks.iter().filter(|st| st.status == SubTaskStatus::Completed).count(),
                "total": coordinated.sub_tasks.len()
            }))
        }
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
