//! Worker Servant - Tool Execution and Concrete Operations
//!
//! The Worker is the "hands" of the guild, responsible for:
//! - Executing tools and operations
//! - Running code changes
//! - Performing file operations
//! - Handling external API calls
//! - Reporting progress to Coordinator

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::{
    Servant, ServantId, ServantRole, ServantStatus, ServantTask, ServantResult, ServantError,
};
use crate::consensus::{ConsensusEngine, Vote};

/// A tool that the worker can execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Tool name/identifier
    pub name: String,
    /// Tool description
    pub description: String,
    /// Parameters schema (JSON Schema)
    pub parameters_schema: serde_json::Value,
    /// Whether this tool requires approval
    pub requires_approval: bool,
    /// Risk level (1-10, higher = more dangerous)
    pub risk_level: u8,
}

impl Tool {
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            parameters_schema: serde_json::json!({}),
            requires_approval: false,
            risk_level: 1,
        }
    }
    
    pub fn with_parameters(mut self, schema: serde_json::Value) -> Self {
        self.parameters_schema = schema;
        self
    }
    
    pub fn requires_approval(mut self) -> Self {
        self.requires_approval = true;
        self.risk_level = 7;
        self
    }
    
    pub fn with_risk_level(mut self, level: u8) -> Self {
        self.risk_level = level.min(10);
        self
    }
}

/// Result of tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Tool that was executed
    pub tool_name: String,
    /// Whether execution succeeded
    pub success: bool,
    /// Output data
    pub output: serde_json::Value,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

/// The Worker servant
pub struct Worker {
    /// Unique ID
    id: ServantId,
    /// Current status
    status: RwLock<ServantStatus>,
    /// Consensus engine reference
    consensus: Option<Arc<ConsensusEngine>>,
    /// Available tools
    tools: RwLock<HashMap<String, Tool>>,
    /// Current task (if any)
    current_task: RwLock<Option<ServantTask>>,
    /// Execution history
    execution_history: RwLock<Vec<ExecutionRecord>>,
}

/// Record of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    /// Execution ID
    pub id: String,
    /// Tool that was executed
    pub tool_name: String,
    /// Parameters used
    pub params: serde_json::Value,
    /// Result
    pub result: ToolResult,
    /// When executed
    pub executed_at: DateTime<Utc>,
    /// Whether approval was obtained
    pub approved: bool,
}

impl Worker {
    /// Create a new Worker
    pub fn new() -> Self {
        let mut worker = Self {
            id: ServantId::new(ServantRole::Worker.default_id()),
            status: RwLock::new(ServantStatus::Starting),
            consensus: None,
            tools: RwLock::new(HashMap::new()),
            current_task: RwLock::new(None),
            execution_history: RwLock::new(Vec::new()),
        };
        
        // Register default tools
        worker.register_default_tools();
        worker
    }
    
    /// Set the consensus engine
    pub fn with_consensus(mut self, consensus: Arc<ConsensusEngine>) -> Self {
        self.consensus = Some(consensus);
        self
    }
    
    /// Register default tools
    fn register_default_tools(&self) {
        let tools = vec![
            Tool::new("read_file".to_string(), "Read a file from the filesystem".to_string())
                .with_risk_level(2),
            Tool::new("write_file".to_string(), "Write content to a file".to_string())
                .with_risk_level(5)
                .requires_approval(),
            Tool::new("delete_file".to_string(), "Delete a file from the filesystem".to_string())
                .with_risk_level(8)
                .requires_approval(),
            Tool::new("run_command".to_string(), "Execute a shell command".to_string())
                .with_risk_level(7)
                .requires_approval(),
            Tool::new("http_request".to_string(), "Make an HTTP request".to_string())
                .with_risk_level(4),
            Tool::new("analyze_code".to_string(), "Analyze code for issues".to_string())
                .with_risk_level(1),
        ];
        
        let mut registered = self.tools.write();
        for tool in tools {
            registered.insert(tool.name.clone(), tool);
        }
    }
    
    /// Register a custom tool
    pub fn register_tool(&self, tool: Tool) {
        self.tools.write().insert(tool.name.clone(), tool);
    }
    
    /// Get available tools
    pub fn get_tools(&self) -> Vec<Tool> {
        self.tools.read().values().cloned().collect()
    }
    
    /// Execute a tool
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        params: serde_json::Value,
    ) -> Result<ToolResult, ServantError> {
        let start = std::time::Instant::now();
        
        // Check if tool exists
        let tool = self.tools.read()
            .get(tool_name)
            .cloned()
            .ok_or_else(|| ServantError::InvalidTask(format!("Tool '{}' not found", tool_name)))?;
        
        // Check if approval is needed
        if tool.requires_approval {
            // TODO: Check consensus for approval
            // For now, we'll proceed but log that approval was needed
        }
        
        // Mark as busy
        *self.status.write() = ServantStatus::Busy;
        
        // TODO: Implement actual tool execution
        // For now, return a mock result
        
        let result = ToolResult {
            tool_name: tool_name.to_string(),
            success: true,
            output: serde_json::json!({
                "message": format!("Tool {} executed successfully", tool_name),
                "params": params,
            }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        };
        
        // Record execution
        let record = ExecutionRecord {
            id: uuid::Uuid::new_v4().to_string(),
            tool_name: tool_name.to_string(),
            params,
            result: result.clone(),
            executed_at: Utc::now(),
            approved: tool.requires_approval,
        };
        
        self.execution_history.write().push(record);
        
        // Mark as ready
        *self.status.write() = ServantStatus::Ready;
        
        Ok(result)
    }
    
    /// Execute a task
    pub async fn execute_task(&self, task: ServantTask) -> Result<ServantResult, ServantError> {
        let start = std::time::Instant::now();
        
        // Set current task
        *self.current_task.write() = Some(task.clone());
        
        // Execute based on task type
        let result = self.execute_tool(&task.task_type, task.params.clone()).await?;
        
        // Clear current task
        *self.current_task.write() = None;
        
        let duration_ms = start.elapsed().as_millis() as u64;
        
        if result.success {
            Ok(ServantResult::success(
                task.id,
                result.output,
                duration_ms,
            ))
        } else {
            Ok(ServantResult::failure(
                task.id,
                result.error.unwrap_or_else(|| "Unknown error".to_string()),
                duration_ms,
            ))
        }
    }
    
    /// Get execution history
    pub fn get_execution_history(&self) -> Vec<ExecutionRecord> {
        self.execution_history.read().clone()
    }
    
    /// Vote on a proposal
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
}

impl Default for Worker {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Servant for Worker {
    fn id(&self) -> &ServantId {
        &self.id
    }
    
    fn role(&self) -> ServantRole {
        ServantRole::Worker
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
        // Wait for current task to complete
        // TODO: Implement graceful shutdown
        *self.status.write() = ServantStatus::Paused;
        Ok(())
    }
    
    fn capabilities(&self) -> Vec<String> {
        self.tools.read().keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_worker_creation() {
        let worker = Worker::new();
        assert_eq!(worker.role(), ServantRole::Worker);
        assert_eq!(worker.status(), ServantStatus::Starting);
        
        // Should have default tools
        let tools = worker.get_tools();
        assert!(!tools.is_empty());
    }
    
    #[tokio::test]
    async fn test_worker_start_stop() {
        let mut worker = Worker::new();
        
        worker.start().await.unwrap();
        assert_eq!(worker.status(), ServantStatus::Ready);
        
        worker.stop().await.unwrap();
        assert_eq!(worker.status(), ServantStatus::Paused);
    }
    
    #[tokio::test]
    async fn test_execute_tool() {
        let mut worker = Worker::new();
        worker.start().await.unwrap();
        
        let result = worker.execute_tool(
            "read_file",
            serde_json::json!({"path": "/test.txt"}),
        ).await;
        
        assert!(result.is_ok());
        let r = result.unwrap();
        assert!(r.success);
    }
    
    #[tokio::test]
    async fn test_execute_unknown_tool() {
        let mut worker = Worker::new();
        worker.start().await.unwrap();
        
        let result = worker.execute_tool(
            "unknown_tool",
            serde_json::json!({}),
        ).await;
        
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_register_custom_tool() {
        let worker = Worker::new();
        
        let custom_tool = Tool::new("custom_op".to_string(), "A custom operation".to_string())
            .with_risk_level(3);
        
        worker.register_tool(custom_tool);
        
        let tools = worker.get_tools();
        assert!(tools.iter().any(|t| t.name == "custom_op"));
    }
}
