//! Safety Module - Prudent Agency and Secure Operations
//!
//! This module provides the safety infrastructure for ServantGuild:
//!
//! - **Audit Log**: Tamper-proof logging of all operations
//! - **Snapshot**: System state capture for rollback
//! - **Rollback**: Safe state restoration
//! - **Policy**: Security policy enforcement
//! - **Prudent Agency**: Safe and auditable agent actions

pub mod audit;
pub mod rollback;
pub mod snapshot;
pub mod policy;
pub mod prudent;

// Re-exports
pub use audit::{AuditEventType, AuditLogger};
pub use rollback::{TransactionManager, RecoveryPolicy, TransactionStatus};
pub use snapshot::{SnapshotManager, Snapshot, SnapshotType};
pub use policy::{SafetyPolicy as SecurityPolicy, RiskLevel as PolicyRule};
pub use prudent::{PrudentAgency, PrudentConfig, PendingAction, ActionRecord, ActionType, ActionStatus, ApprovalStatus, PrudentError};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Common safety error type
#[derive(Debug, Error)]
pub enum SafetyError {
    #[error("Audit error: {0}")]
    Audit(String),
    
    #[error("Snapshot error: {0}")]
    Snapshot(String),
    
    #[error("Rollback error: {0}")]
    Rollback(String),
    
    #[error("Policy violation: {0}")]
    Policy(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
}
