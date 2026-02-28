//! Safety module
//!
//! This module provides safety mechanisms for ServantGuild,
//! including rollback, recovery, canary testing, and disaster recovery capabilities.

pub mod canary;
pub mod rollback;
pub mod state_recovery;

pub use canary::{
    Anomaly, AnomalyThreshold, CanaryConfig, CanaryPhase, CanaryResult, CanaryRunner,
    CanaryStatus, CanaryTester, MetricSummary, ThresholdStatus,
};
pub use rollback::{
    RecoveryPlan, RecoveryStep, RecoveryStepType, RecoveryType, RollbackRecoveryManager,
    RollbackResult, SnapshotEntry, SnapshotEntryType, SnapshotMetadata, SnapshotType,
};
pub use state_recovery::{
    RecoveryConfig, RecoveryManager, RecoveryPhase, RecoveryRecord, RecoveryResult,
    RecoveryStats, RecoveryStatus, SnapshotManager, StateChange,
};
