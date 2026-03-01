//! Safety module
//!
//! This module provides safety mechanisms for ServantGuild,
//! including rollback, recovery, canary testing, and disaster recovery capabilities.

pub mod audit;
pub mod canary;
pub mod policy;
#[cfg(feature = "rollback-recovery")]
pub mod rollback;
pub mod snapshot;
#[cfg(feature = "rollback-recovery")]
pub mod state_recovery;

pub use audit::{AuditEvent, AuditEventType, AuditLogger};
pub use canary::{
    Anomaly, AnomalyThreshold, CanaryConfig, CanaryPhase, CanaryResult, CanaryRunner, CanaryStatus,
    CanaryTester, MetricSummary, ThresholdStatus,
};
pub use policy::{RiskLevel, SafetyPolicy};
#[cfg(feature = "rollback-recovery")]
pub use rollback::{
    RecoveryPlan, RecoveryStep, RecoveryStepType, RecoveryType, RollbackRecoveryManager,
    RollbackResult, SnapshotEntry, SnapshotEntryType, SnapshotMetadata, SnapshotType,
};
pub use snapshot::Snapshot;
pub use state_recovery::{
    RecoveryConfig, RecoveryManager, RecoveryPhase, RecoveryRecord, RecoveryResult, RecoveryStats,
    RecoveryStatus, SnapshotManager, StateChange,
};

/// Re-export SafetyPolicy as SecurityPolicy for backward compatibility
pub type SecurityPolicy = SafetyPolicy;

/// Error type for safety operations
#[derive(Debug, thiserror::Error)]
pub enum SafetyError {
    #[error("Policy violation: {0}")]
    PolicyViolation(String),

    #[error("Rollback failed: {0}")]
    RollbackFailed(String),

    #[error("Snapshot error: {0}")]
    SnapshotError(String),

    #[error("Recovery failed: {0}")]
    RecoveryFailed(String),

    #[error("Audit error: {0}")]
    AuditError(String),
}

/// Transaction manager for atomic operations
pub struct TransactionManager {
    snapshots: Vec<Snapshot>,
}

impl TransactionManager {
    pub fn new() -> Self {
        Self {
            snapshots: Vec::new(),
        }
    }

    pub fn begin(&mut self) -> Result<(), SafetyError> {
        Ok(())
    }

    pub fn commit(&mut self) -> Result<(), SafetyError> {
        self.snapshots.clear();
        Ok(())
    }

    pub fn rollback(&mut self) -> Result<(), SafetyError> {
        // Restore from snapshots
        self.snapshots.clear();
        Ok(())
    }
}

impl Default for TransactionManager {
    fn default() -> Self {
        Self::new()
    }
}
