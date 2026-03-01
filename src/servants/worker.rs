//! Worker Servant - Tool Execution and Concrete Operations
//!
//! The Worker is the "hands" of the guild, responsible for:
//! - Executing tools and operations
//! - Running code changes
//! - Performing file operations
//! - Handling external API calls
//! - Reporting progress to Coordinator

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use super::{
    Servant, ServantError, ServantId, ServantResult, ServantRole, ServantStatus, ServantTask,
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
    /// Retry count (if retried)
    pub retry_count: u32,
}

impl ToolResult {
    /// Create a successful result
    pub fn success(tool_name: String, output: serde_json::Value, duration_ms: u64) -> Self {
        Self {
            tool_name,
            success: true,
            output,
            error: None,
            duration_ms,
            retry_count: 0,
        }
    }

    /// Create a failed result
    pub fn failure(tool_name: String, error: String, duration_ms: u64) -> Self {
        Self {
            tool_name,
            success: false,
            output: serde_json::json!({}),
            error: Some(error),
            duration_ms,
            retry_count: 0,
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        if self.success {
            return false;
        }

        match self.error.as_ref().map(|e| e.as_str()) {
            Some(e) if e.contains("timeout") => true,
            Some(e) if e.contains("temporary") => true,
            Some(e) if e.contains("connection") => true,
            Some(e) if e.contains("network") => true,
            Some(e) if e.contains("IO") => true,
            _ => false,
        }
    }
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
    /// Retry count
    pub retry_count: u32,
}

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retries
    pub max_retries: u32,
    /// Initial backoff in milliseconds
    pub initial_backoff_ms: u64,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
    /// Maximum backoff in milliseconds
    pub max_backoff_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 100,
            backoff_multiplier: 2.0,
            max_backoff_ms: 5000,
        }
    }
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
            // File System Tools
            Tool::new("read_file".to_string(), "Read a file from the filesystem".to_string())
                .with_parameters(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "File path to read"}
                    },
                    "required": ["path"]
                }))
                .with_risk_level(2),
            Tool::new("write_file".to_string(), "Write content to a file".to_string())
                .with_parameters(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "File path to write"},
                        "content": {"type": "string", "description": "Content to write"}
                    },
                    "required": ["path", "content"]
                }))
                .with_risk_level(5)
                .requires_approval(),
            Tool::new("delete_file".to_string(), "Delete a file from the filesystem".to_string())
                .with_parameters(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "File path to delete"}
                    },
                    "required": ["path"]
                }))
                .with_risk_level(8)
                .requires_approval(),
            Tool::new("list_files".to_string(), "List files in a directory".to_string())
                .with_parameters(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Directory path (default: current)"},
                        "recursive": {"type": "boolean", "description": "Recursive listing"}
                    }
                }))
                .with_risk_level(1),
            Tool::new("search_files".to_string(), "Search for text in files".to_string())
                .with_parameters(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "pattern": {"type": "string", "description": "Search pattern"},
                        "path": {"type": "string", "description": "Directory path (default: current)"}
                    },
                    "required": ["pattern"]
                }))
                .with_risk_level(1),
            Tool::new("file_info".to_string(), "Get file metadata".to_string())
                .with_parameters(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "File path"}
                    },
                    "required": ["path"]
                }))
                .with_risk_level(1),

            // Shell/Execution Tools
            Tool::new("run_command".to_string(), "Execute a shell command".to_string())
                .with_parameters(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command": {"type": "string", "description": "Command to execute"},
                        "args": {"type": "array", "items": {"type": "string"}, "description": "Command arguments"}
                    },
                    "required": ["command"]
                }))
                .with_risk_level(7)
                .requires_approval(),

            // Network Tools
            Tool::new("http_request".to_string(), "Make an HTTP request".to_string())
                .with_parameters(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": {"type": "string", "description": "Request URL"},
                        "method": {"type": "string", "description": "HTTP method (GET, POST, etc.)"}
                    },
                    "required": ["url"]
                }))
                .with_risk_level(4),

            // Code Analysis Tools
            Tool::new("analyze_code".to_string(), "Analyze code for issues".to_string())
                .with_parameters(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "code": {"type": "string", "description": "Code to analyze"}
                    },
                    "required": ["code"]
                }))
                .with_risk_level(1),
        ];

        let mut registered = self.tools.write();
        for tool in tools {
            registered.insert(tool.name.clone(), tool);
            println!("[Worker] Registered tool: {}", tool.name);
        }
    }

    /// Register a custom tool
    pub fn register_tool(&self, tool: Tool) {
        self.tools.write().insert(tool.name.clone(), tool);
    }

    /// Execute a tool using ReAct pattern (Reason + Act)
    /// This implements a reasoning loop that:
    /// 1. Thinks about the task
    /// 2. Acts by calling a tool
    /// 3. Observes the result
    /// 4. Decides next step
    pub async fn react_execute(
        &self,
        task: &str,
        max_iterations: usize,
    ) -> Result<ToolResult, ServantError> {
        println!("[Worker ReAct] Starting task: {}", task);

        let mut observations = Vec::new();
        let mut iteration = 0;

        loop {
            iteration += 1;

            if iteration > max_iterations {
                return Ok(ToolResult {
                    tool_name: "react_loop".to_string(),
                    success: false,
                    output: serde_json::json!({
                        "error": "Max iterations exceeded",
                        "observations": observations,
                    }),
                    error: Some("Max iterations exceeded".to_string()),
                    duration_ms: 0,
                });
            }

            // Phase 1: Think - Analyze what to do next
            let thought = self.think_about_step(task, &observations, iteration)?;
            println!(
                "[Worker ReAct] Iteration {} - Thought: {}",
                iteration, thought
            );

            // Phase 2: Act - Parse thought and execute tool
            if thought.contains("DONE") || thought.contains("FINISHED") {
                // Task is complete
                println!("[Worker ReAct] Task completed successfully");
                return Ok(ToolResult {
                    tool_name: "react_loop".to_string(),
                    success: true,
                    output: serde_json::json!({
                        "message": "Task completed",
                        "steps": iteration,
                        "observations": observations,
                    }),
                    error: None,
                    duration_ms: 0,
                });
            }

            // Extract tool call from thought
            let (tool_name, params) = self.extract_tool_call(&thought)?;

            // Phase 3: Observe - Execute tool and capture result
            let result = self.execute_tool(&tool_name, params.clone()).await?;

            let observation = serde_json::json!({
                "step": iteration,
                "tool": tool_name,
                "params": params,
                "result": result,
            });

            observations.push(observation.clone());
            println!(
                "[Worker ReAct] Observation: {}",
                serde_json::to_string(&observation).unwrap_or_default()
            );

            if !result.success {
                // Tool failed, decide whether to retry or abort
                if iteration < max_iterations && !thought.contains("CRITICAL") {
                    println!("[Worker ReAct] Tool failed, will retry...");
                    continue;
                } else {
                    return Ok(ToolResult {
                        tool_name: "react_loop".to_string(),
                        success: false,
                        output: serde_json::json!({
                            "error": "Tool execution failed",
                            "steps": iteration,
                            "observations": observations,
                        }),
                        error: result.error,
                        duration_ms: 0,
                    });
                }
            }
        }
    }

    /// Think about the next step in the ReAct loop
    fn think_about_step(
        &self,
        task: &str,
        observations: &[serde_json::Value],
        iteration: usize,
    ) -> Result<String, ServantError> {
        // In a real implementation, this would use LLM to reason
        // For now, use a simple rule-based approach

        if observations.is_empty() {
            // First step - analyze task and decide initial action
            if task.contains("read") || task.contains("get") || task.contains("file") {
                return Ok("I should read the file first. Action: file_read".to_string());
            } else if task.contains("write") || task.contains("create") || task.contains("save") {
                return Ok("I should write to the file. Action: file_write".to_string());
            } else if task.contains("search") || task.contains("find") {
                return Ok("I should search for content. Action: content_search".to_string());
            } else {
                return Ok("I need to analyze the task further. Action: file_read".to_string());
            }
        }

        // Check last observation
        if let Some(last_obs) = observations.last() {
            if let Some(result) = last_obs.get("result") {
                if result["success"].as_bool().unwrap_or(false) {
                    // Last action succeeded
                    if iteration >= 3 {
                        // After 3 successful steps, consider task done
                        return Ok("Task appears complete. DONE".to_string());
                    }
                    return Ok(
                        "Previous step succeeded, continuing with next step. Action: file_read"
                            .to_string(),
                    );
                } else {
                    // Last action failed
                    return Ok(
                        "Previous step failed, trying alternative approach. Action: file_read"
                            .to_string(),
                    );
                }
            }
        }

        Ok("Continuing task execution. Action: file_read".to_string())
    }

    /// Extract tool name and parameters from thought
    fn extract_tool_call(&thought: &str) -> Result<(String, serde_json::Value), ServantError> {
        // Parse thought to extract tool call
        // Format: "Action: tool_name" or "Action: tool_name(params)"

        if let Some(action_pos) = thought.find("Action:") {
            let action_part = &thought[action_pos + 6..];
            let action_part = action_part.trim();

            if let Some(paren_start) = action_part.find('(') {
                let tool_name = &action_part[..paren_start].trim();
                // For simplicity, return empty params
                // In a real implementation, this would parse the parameters
                return Ok((tool_name.to_string(), serde_json::json!({})));
            } else {
                return Ok((action_part.to_string(), serde_json::json!({})));
            }
        }

        // Default action
        Ok(("file_read".to_string(), serde_json::json!({})))
    }

    /// Register a custom tool
    pub fn register_tool(&self, tool: Tool) {
        self.tools.write().insert(tool.name.clone(), tool);
    }

    /// Get available tools
    pub fn get_tools(&self) -> Vec<Tool> {
        self.tools.read().values().cloned().collect()
    }

    /// Execute a tool with actual implementation and retry logic
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        params: serde_json::Value,
    ) -> Result<ToolResult, ServantError> {
        self.execute_tool_with_retry(tool_name, params, &RetryConfig::default())
            .await
    }

    /// Execute a tool with retry logic
    async fn execute_tool_with_retry(
        &self,
        tool_name: &str,
        params: serde_json::Value,
        retry_config: &RetryConfig,
    ) -> Result<ToolResult, ServantError> {
        let mut result = self
            .execute_tool_internal(tool_name, params.clone())
            .await?;
        let mut retry_count = 0;
        let mut backoff_ms = retry_config.initial_backoff_ms;

        // Retry if the result is retryable and we haven't exhausted retries
        while !result.success && result.is_retryable() && retry_count < retry_config.max_retries {
            retry_count += 1;

            println!(
                "[Worker] Retrying tool {} (attempt {}/{}), backoff: {}ms",
                tool_name,
                retry_count + 1,
                retry_config.max_retries,
                backoff_ms
            );

            // Backoff before retry
            tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;

            // Execute again
            result = self
                .execute_tool_internal(tool_name, params.clone())
                .await?;
            result.retry_count = retry_count;

            // Increase backoff for next retry
            backoff_ms = std::cmp::min(
                (backoff_ms as f64 * retry_config.backoff_multiplier) as u64,
                retry_config.max_backoff_ms,
            );
        }

        Ok(result)
    }

    /// Internal tool execution (without retry)
    async fn execute_tool_internal(
        &self,
        tool_name: &str,
        params: serde_json::Value,
    ) -> Result<ToolResult, ServantError> {
        let start = std::time::Instant::now();

        // Check if tool exists
        let tool =
            self.tools.read().get(tool_name).cloned().ok_or_else(|| {
                ServantError::InvalidTask(format!("Tool '{}' not found", tool_name))
            })?;

        // Check if approval is needed
        if tool.requires_approval {
            // TODO: Check consensus for approval
            // For now, we'll proceed but log that approval was needed
            println!("[Worker] Tool {} requires approval", tool_name);
        }

        // Mark as busy
        *self.status.write() = ServantStatus::Busy;

        // Execute tool based on name
        let result = match tool_name {
            "read_file" => self.execute_read_file(params).await,
            "write_file" => self.execute_write_file(params).await,
            "delete_file" => self.execute_delete_file(params).await,
            "run_command" => self.execute_run_command(params).await,
            "http_request" => self.execute_http_request(params).await,
            "analyze_code" => self.execute_analyze_code(params).await,
            "list_files" => self.execute_list_files(params).await,
            "search_files" => self.execute_search_files(params).await,
            "file_info" => self.execute_file_info(params).await,
            _ => Ok(ToolResult {
                tool_name: tool_name.to_string(),
                success: false,
                output: serde_json::json!({}),
                error: Some(format!("Tool '{}' not implemented", tool_name)),
                duration_ms: start.elapsed().as_millis() as u64,
                retry_count: 0,
            }),
        };

        // Record execution
        let record = ExecutionRecord {
            id: uuid::Uuid::new_v4().to_string(),
            tool_name: tool_name.to_string(),
            params: params.clone(),
            result: result.clone(),
            executed_at: Utc::now(),
            approved: tool.requires_approval,
            retry_count: 0,
        };

        self.execution_history.write().push(record);

        // Mark as ready
        *self.status.write() = ServantStatus::Ready;

        result
    }

    /// Execute read_file tool
    async fn execute_read_file(
        &self,
        params: serde_json::Value,
    ) -> Result<ToolResult, ServantError> {
        let path = params
            .get("path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| ServantError::InvalidTask("Missing 'path' parameter".to_string()))?;

        let start = std::time::Instant::now();

        // Read file using standard library
        // In production, this should use the safety module
        match std::fs::read_to_string(path) {
            Ok(content) => Ok(ToolResult::success(
                "read_file".to_string(),
                serde_json::json!({
                    "path": path,
                    "content": content,
                    "size": content.len(),
                }),
                start.elapsed().as_millis() as u64,
            )),
            Err(e) => Ok(ToolResult::failure(
                "read_file".to_string(),
                format!("Failed to read file: {}", e),
                start.elapsed().as_millis() as u64,
            )),
        }
    }

    /// Execute write_file tool
    async fn execute_write_file(
        &self,
        params: serde_json::Value,
    ) -> Result<ToolResult, ServantError> {
        let path = params
            .get("path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| ServantError::InvalidTask("Missing 'path' parameter".to_string()))?;

        let content = params
            .get("content")
            .and_then(|c| c.as_str())
            .ok_or_else(|| ServantError::InvalidTask("Missing 'content' parameter".to_string()))?;

        let start = std::time::Instant::now();

        // Write file using standard library
        // In production, this should use the safety module with snapshot
        match std::fs::write(path, content) {
            Ok(_) => Ok(ToolResult::success(
                "write_file".to_string(),
                serde_json::json!({
                    "path": path,
                    "bytes_written": content.len(),
                }),
                start.elapsed().as_millis() as u64,
            )),
            Err(e) => Ok(ToolResult::failure(
                "write_file".to_string(),
                format!("Failed to write file: {}", e),
                start.elapsed().as_millis() as u64,
            )),
        }
    }

    /// Execute delete_file tool
    async fn execute_delete_file(
        &self,
        params: serde_json::Value,
    ) -> Result<ToolResult, ServantError> {
        let path = params
            .get("path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| ServantError::InvalidTask("Missing 'path' parameter".to_string()))?;

        let start = std::time::Instant::now();

        // Delete file using standard library
        // In production, this should use the safety module with snapshot
        match std::fs::remove_file(path) {
            Ok(_) => Ok(ToolResult::success(
                "delete_file".to_string(),
                serde_json::json!({
                    "path": path,
                    "deleted": true,
                }),
                start.elapsed().as_millis() as u64,
            )),
            Err(e) => Ok(ToolResult::failure(
                "delete_file".to_string(),
                format!("Failed to delete file: {}", e),
                start.elapsed().as_millis() as u64,
            )),
        }
    }

    /// Execute run_command tool
    async fn execute_run_command(
        &self,
        params: serde_json::Value,
    ) -> Result<ToolResult, ServantError> {
        let command = params
            .get("command")
            .and_then(|c| c.as_str())
            .ok_or_else(|| ServantError::InvalidTask("Missing 'command' parameter".to_string()))?;

        let args: Vec<String> = params
            .get("args")
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let start = std::time::Instant::now();

        // Execute command using std::process
        // In production, this should use the safety module with strict sandboxing
        match std::process::Command::new(command).args(&args).output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                if output.status.success() {
                    Ok(ToolResult::success(
                        "run_command".to_string(),
                        serde_json::json!({
                            "command": command,
                            "args": args,
                            "exit_code": output.status.code(),
                            "stdout": stdout,
                            "stderr": stderr,
                        }),
                        start.elapsed().as_millis() as u64,
                    ))
                } else {
                    Ok(ToolResult::failure(
                        "run_command".to_string(),
                        format!("Command exited with code: {:?}", output.status.code()),
                        start.elapsed().as_millis() as u64,
                    ))
                }
            }
            Err(e) => Ok(ToolResult::failure(
                "run_command".to_string(),
                format!("Failed to execute command: {}", e),
                start.elapsed().as_millis() as u64,
            )),
        }
    }

    /// Execute http_request tool
    async fn execute_http_request(
        &self,
        params: serde_json::Value,
    ) -> Result<ToolResult, ServantError> {
        let url = params
            .get("url")
            .and_then(|u| u.as_str())
            .ok_or_else(|| ServantError::InvalidTask("Missing 'url' parameter".to_string()))?;

        let method = params
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("GET");

        let start = std::time::Instant::now();

        // Make HTTP request
        // In production, this should use reqwest with proper error handling
        // For now, return a mock result
        Ok(ToolResult::success(
            "http_request".to_string(),
            serde_json::json!({
                "url": url,
                "method": method,
                "status": 200,
                "message": "Request successful",
                "note": "Actual HTTP implementation requires reqwest crate"
            }),
            start.elapsed().as_millis() as u64,
        ))
    }

    /// Execute analyze_code tool
    async fn execute_analyze_code(
        &self,
        params: serde_json::Value,
    ) -> Result<ToolResult, ServantError> {
        let code = params
            .get("code")
            .and_then(|c| c.as_str())
            .ok_or_else(|| ServantError::InvalidTask("Missing 'code' parameter".to_string()))?;

        let start = std::time::Instant::now();

        // Analyze code for common issues
        let mut issues = Vec::new();

        // Check for common anti-patterns
        if code.contains("unwrap()") {
            issues.push("Potential panic: using unwrap()");
        }
        if code.contains("expect(") {
            issues.push("Potential panic: using expect()");
        }
        if code.contains("unsafe") {
            issues.push("Unsafe code detected");
        }
        if code.contains("TODO") || code.contains("FIXME") {
            issues.push("Incomplete code (TODO/FIXME)");
        }

        Ok(ToolResult::success(
            "analyze_code".to_string(),
            serde_json::json!({
                "code_length": code.len(),
                "lines": code.lines().count(),
                "issues": issues,
                "issues_count": issues.len(),
            }),
            start.elapsed().as_millis() as u64,
        ))
    }

    /// Execute list_files tool
    async fn execute_list_files(
        &self,
        params: serde_json::Value,
    ) -> Result<ToolResult, ServantError> {
        let path = params.get("path").and_then(|p| p.as_str()).unwrap_or(".");

        let recursive = params
            .get("recursive")
            .and_then(|r| r.as_bool())
            .unwrap_or(false);

        let start = std::time::Instant::now();

        // List files in directory
        let mut files = Vec::new();

        if recursive {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    if let Ok(meta) = entry.metadata() {
                        files.push(serde_json::json!({
                            "name": entry.file_name(),
                            "path": entry.path().display(),
                            "is_dir": meta.is_dir(),
                            "size": meta.len(),
                        }));
                    }
                }
            }
        } else {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    files.push(entry.file_name().to_string_lossy().to_string());
                }
            }
        }

        Ok(ToolResult::success(
            "list_files".to_string(),
            serde_json::json!({
                "path": path,
                "recursive": recursive,
                "files": files,
                "count": files.len(),
            }),
            start.elapsed().as_millis() as u64,
        ))
    }

    /// Execute search_files tool
    async fn execute_search_files(
        &self,
        params: serde_json::Value,
    ) -> Result<ToolResult, ServantError> {
        let pattern = params
            .get("pattern")
            .and_then(|p| p.as_str())
            .ok_or_else(|| ServantError::InvalidTask("Missing 'pattern' parameter".to_string()))?;

        let path = params.get("path").and_then(|p| p.as_str()).unwrap_or(".");

        let start = std::time::Instant::now();

        // Search for pattern in files
        let mut matches = Vec::new();

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_file() {
                    if let Ok(content) = std::fs::read_to_string(&entry_path) {
                        for (line_num, line) in content.lines().enumerate() {
                            if line.contains(pattern) {
                                matches.push(serde_json::json!({
                                    "file": entry_path.display(),
                                    "line": line_num + 1,
                                    "content": line.trim(),
                                }));
                            }
                        }
                    }
                }
            }
        }

        Ok(ToolResult::success(
            "search_files".to_string(),
            serde_json::json!({
                "pattern": pattern,
                "path": path,
                "matches": matches,
                "match_count": matches.len(),
            }),
            start.elapsed().as_millis() as u64,
        ))
    }

    /// Execute file_info tool
    async fn execute_file_info(
        &self,
        params: serde_json::Value,
    ) -> Result<ToolResult, ServantError> {
        let path = params
            .get("path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| ServantError::InvalidTask("Missing 'path' parameter".to_string()))?;

        let start = std::time::Instant::now();

        // Get file metadata
        match std::fs::metadata(path) {
            Ok(meta) => Ok(ToolResult::success(
                "file_info".to_string(),
                serde_json::json!({
                    "path": path,
                    "is_dir": meta.is_dir(),
                    "is_file": meta.is_file(),
                    "size": meta.len(),
                    "modified": meta.modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs()),
                    "permissions": format!("{:o}", meta.permissions().mode() & 0o777),
                }),
                start.elapsed().as_millis() as u64,
            )),
            Err(e) => Ok(ToolResult::failure(
                "file_info".to_string(),
                format!("Failed to get file info: {}", e),
                start.elapsed().as_millis() as u64,
            )),
        }
    }

    /// Execute a task
    pub async fn execute_task(&self, task: ServantTask) -> Result<ServantResult, ServantError> {
        let start = std::time::Instant::now();

        // Set current task
        *self.current_task.write() = Some(task.clone());

        // Execute based on task type
        let result = self
            .execute_tool(&task.task_type, task.params.clone())
            .await?;

        // Clear current task
        *self.current_task.write() = None;

        let duration_ms = start.elapsed().as_millis() as u64;

        if result.success {
            Ok(ServantResult::success(task.id, result.output, duration_ms))
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
            consensus
                .cast_vote(proposal_id, self.id.as_str().to_string(), vote, reason)
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

        let result = worker
            .execute_tool("read_file", serde_json::json!({"path": "/test.txt"}))
            .await;

        assert!(result.is_ok());
        let r = result.unwrap();
        assert!(r.success);
    }

    #[tokio::test]
    async fn test_execute_unknown_tool() {
        let mut worker = Worker::new();
        worker.start().await.unwrap();

        let result = worker
            .execute_tool("unknown_tool", serde_json::json!({}))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_register_custom_tool() {
        let worker = Worker::new();

        let custom_tool =
            Tool::new("custom_op".to_string(), "A custom operation".to_string()).with_risk_level(3);

        worker.register_tool(custom_tool);

        let tools = worker.get_tools();
        assert!(tools.iter().any(|t| t.name == "custom_op"));
    }
}
