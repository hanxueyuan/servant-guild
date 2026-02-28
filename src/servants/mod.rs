//! Core Servants - The Five Pillars of the Guild
//!
//! The Core Servants are permanent members of the guild, each with specific
//! responsibilities. They are always running and form the backbone of the
//! multi-agent collaboration system.
//!
//! ## The Five Core Servants
//!
//! 1. **Coordinator** - Task distribution and workflow orchestration
//! 2. **Worker** - Tool execution and concrete operations
//! 3. **Warden** - Security auditing and safety enforcement
//! 4. **Speaker** - Communication and consensus building
//! 5. **Contractor** - Resource management and configuration
//!
//! ## Architecture
//!
//! Each servant:
//! - Has a unique ID and role
//! - Can vote on proposals (consensus)
//! - Can be invoked via Host interface
//! - Maintains its own state and memory

pub mod coordinator;
pub mod worker;
pub mod warden;
pub mod speaker;
pub mod contractor;

// Re-exports
pub use coordinator::Coordinator;
pub use worker::Worker;
pub use warden::Warden;
pub use speaker::Speaker;
pub use contractor::Contractor;

use serde::{Deserialize, Serialize};
use async_trait::async_trait;

/// Unique identifier for a servant
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ServantId(String);

impl ServantId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ServantId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Role of a servant in the guild
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServantRole {
    /// Task distribution and workflow orchestration
    Coordinator,
    /// Tool execution and concrete operations
    Worker,
    /// Security auditing and safety enforcement
    Warden,
    /// Communication and consensus building
    Speaker,
    /// Resource management and configuration
    Contractor,
}

impl ServantRole {
    /// Get all core roles
    pub fn all() -> Vec<Self> {
        vec![
            Self::Coordinator,
            Self::Worker,
            Self::Warden,
            Self::Speaker,
            Self::Contractor,
        ]
    }
    
    /// Get the default ID for this role
    pub fn default_id(&self) -> &'static str {
        match self {
            Self::Coordinator => "coordinator",
            Self::Worker => "worker",
            Self::Warden => "warden",
            Self::Speaker => "speaker",
            Self::Contractor => "contractor",
        }
    }
    
    /// Get a human-readable description of this role
    pub fn description(&self) -> &'static str {
        match self {
            Self::Coordinator => "Orchestrates task distribution and manages workflow",
            Self::Worker => "Executes tools and performs concrete operations",
            Self::Warden => "Enforces security policies and audits operations",
            Self::Speaker => "Manages communication and facilitates consensus",
            Self::Contractor => "Manages resources and configuration",
        }
    }
}

/// Status of a servant
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServantStatus {
    /// Servant is starting up
    Starting,
    /// Servant is ready to accept tasks
    Ready,
    /// Servant is processing a task
    Busy,
    /// Servant is paused (maintenance mode)
    Paused,
    /// Servant encountered an error
    Error,
    /// Servant is shutting down
    Stopping,
}

/// Trait that all core servants must implement
#[async_trait]
pub trait Servant: Send + Sync {
    /// Get the servant's ID
    fn id(&self) -> &ServantId;
    
    /// Get the servant's role
    fn role(&self) -> ServantRole;
    
    /// Get the current status
    fn status(&self) -> ServantStatus;
    
    /// Start the servant
    async fn start(&mut self) -> Result<(), ServantError>;
    
    /// Stop the servant gracefully
    async fn stop(&mut self) -> Result<(), ServantError>;
    
    /// Get the servant's capabilities
    fn capabilities(&self) -> Vec<String>;
}

/// Errors that can occur in servant operations
#[derive(Debug, thiserror::Error)]
pub enum ServantError {
    #[error("Servant is not ready: {0}")]
    NotReady(String),
    
    #[error("Servant is already running")]
    AlreadyRunning,
    
    #[error("Servant failed to start: {0}")]
    StartupFailed(String),
    
    #[error("Task execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Invalid task: {0}")]
    InvalidTask(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

/// A task to be executed by a servant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServantTask {
    /// Unique task ID
    pub id: String,
    /// Type of task
    pub task_type: String,
    /// Task parameters
    pub params: serde_json::Value,
    /// Priority (higher = more urgent)
    pub priority: u8,
    /// Who requested this task
    pub requester: String,
    /// When the task was created
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl ServantTask {
    pub fn new(task_type: String, params: serde_json::Value, requester: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            task_type,
            params,
            priority: 5,
            requester,
            created_at: chrono::Utc::now(),
        }
    }
    
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority.min(10);
        self
    }
}

/// Result of a task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServantResult {
    /// ID of the task that was executed
    pub task_id: String,
    /// Whether the task succeeded
    pub success: bool,
    /// Result data (if successful)
    pub data: Option<serde_json::Value>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

impl ServantResult {
    pub fn success(task_id: String, data: serde_json::Value, duration_ms: u64) -> Self {
        Self {
            task_id,
            success: true,
            data: Some(data),
            error: None,
            duration_ms,
        }
    }
    
    pub fn failure(task_id: String, error: String, duration_ms: u64) -> Self {
        Self {
            task_id,
            success: false,
            data: None,
            error: Some(error),
            duration_ms,
        }
    }
}
