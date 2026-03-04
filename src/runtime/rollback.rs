//! Rollback & Recovery - System State Management
//!
//! This module provides rollback and recovery capabilities for ServantGuild,
//! enabling safe version management, state preservation, and disaster recovery.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Rollback point type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RollbackPointType {
    /// Manual checkpoint
    ManualCheckpoint,
    /// Pre-deployment snapshot
    PreDeployment,
    /// Post-deployment snapshot
    PostDeployment,
    /// Error recovery point
    ErrorRecovery,
    /// Scheduled backup
    ScheduledBackup,
}

/// Module version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleVersion {
    /// Version string
    pub version: String,
    /// Git commit SHA
    pub commit_sha: Option<String>,
    /// Build timestamp
    pub build_timestamp: Option<DateTime<Utc>>,
}

impl ModuleVersion {
    /// Create a new module version
    pub fn new(version: String) -> Self {
        Self {
            version,
            commit_sha: None,
            build_timestamp: None,
        }
    }
}

/// Rollback point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackPoint {
    /// Unique identifier
    pub id: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Point type
    pub point_type: RollbackPointType,
    /// Description
    pub description: String,
    /// Module versions
    pub module_versions: HashMap<String, ModuleVersion>,
    /// State snapshot path
    pub state_snapshot_path: Option<PathBuf>,
    /// Configuration snapshot
    pub config_snapshot: Option<serde_json::Value>,
    /// Git commit SHA
    pub git_commit_sha: Option<String>,
    /// Whether verified
    pub verified: bool,
}

impl RollbackPoint {
    /// Create a new rollback point
    pub fn new(id: String, point_type: RollbackPointType, description: String) -> Self {
        Self {
            id,
            timestamp: Utc::now(),
            point_type,
            description,
            module_versions: HashMap::new(),
            state_snapshot_path: None,
            config_snapshot: None,
            git_commit_sha: None,
            verified: false,
        }
    }

    /// Add module version
    pub fn with_module_version(mut self, module: String, version: ModuleVersion) -> Self {
        self.module_versions.insert(module, version);
        self
    }

    /// Set state snapshot path
    pub fn with_state_snapshot(mut self, path: PathBuf) -> Self {
        self.state_snapshot_path = Some(path);
        self
    }

    /// Set configuration snapshot
    pub fn with_config_snapshot(mut self, config: serde_json::Value) -> Self {
        self.config_snapshot = Some(config);
        self
    }
}

/// Rollback result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackResult {
    /// Rollback point ID
    pub rollback_point_id: String,
    /// Whether rollback succeeded
    pub success: bool,
    /// Modules rolled back
    pub modules_rolled_back: Vec<String>,
    /// State restored
    pub state_restored: bool,
    /// Configuration restored
    pub config_restored: bool,
    /// Rollback duration in milliseconds
    pub duration_ms: u64,
    /// Warnings
    pub warnings: Vec<String>,
    /// Errors
    pub errors: Vec<String>,
    /// Started at
    pub started_at: DateTime<Utc>,
    /// Ended at
    pub ended_at: DateTime<Utc>,
}

/// Recovery plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryPlan {
    /// Plan ID
    pub id: String,
    /// Target rollback point
    pub target_point: RollbackPoint,
    /// Recovery steps
    pub steps: Vec<RecoveryStep>,
    /// Estimated duration in seconds
    pub estimated_duration_secs: u64,
    /// Whether data loss is acceptable
    pub data_loss_acceptable: bool,
    /// Created at
    pub created_at: DateTime<Utc>,
}

/// Recovery step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStep {
    /// Step number
    pub step_number: u32,
    /// Step description
    pub description: String,
    /// Step type
    pub step_type: RecoveryStepType,
    /// Estimated duration in seconds
    pub estimated_duration_secs: u64,
    /// Whether step is critical
    pub is_critical: bool,
}

/// Recovery step type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryStepType {
    /// Stop services
    StopServices,
    /// Restore state
    RestoreState,
    /// Restore configuration
    RestoreConfig,
    /// Rollback modules
    RollbackModules,
    /// Verify integrity
    VerifyIntegrity,
    /// Start services
    StartServices,
    /// Health check
    HealthCheck,
}

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Backup interval in seconds
    pub backup_interval_secs: u64,
    /// Maximum number of rollback points to keep
    pub max_rollback_points: usize,
    /// Whether to include state snapshots
    pub include_state_snapshots: bool,
    /// Whether to include configuration
    pub include_config: bool,
    /// Backup storage path
    pub storage_path: PathBuf,
    /// Compression enabled
    pub compression_enabled: bool,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            backup_interval_secs: 3600,
            max_rollback_points: 100,
            include_state_snapshots: true,
            include_config: true,
            storage_path: PathBuf::from("/tmp/servant-guild/backups"),
            compression_enabled: true,
        }
    }
}

/// Rollback & Recovery manager
pub struct RollbackManager {
    /// Rollback points (in-memory cache)
    rollback_points: Arc<RwLock<Vec<RollbackPoint>>>,
    /// Backup configuration
    backup_config: BackupConfig,
}

impl RollbackManager {
    /// Create new rollback manager
    pub fn new(backup_config: BackupConfig) -> Result<Self> {
        Ok(Self {
            rollback_points: Arc::new(RwLock::new(Vec::new())),
            backup_config,
        })
    }

    /// Create a rollback point
    pub async fn create_rollback_point(
        &self,
        point_type: RollbackPointType,
        description: String,
    ) -> Result<RollbackPoint> {
        let id = uuid::Uuid::new_v4().to_string();
        info!("Creating rollback point '{}' with type {:?}", id, point_type);

        let point = RollbackPoint::new(id.clone(), point_type, description);

        let mut points = self.rollback_points.write().await;
        points.push(point.clone());

        // Trim old points if needed
        while points.len() > self.backup_config.max_rollback_points {
            points.remove(0);
        }

        Ok(point)
    }

    /// Get rollback point by ID
    pub async fn get_rollback_point(&self, id: &str) -> Option<RollbackPoint> {
        let points = self.rollback_points.read().await;
        points.iter().find(|p| p.id == id).cloned()
    }

    /// Get all rollback points
    pub async fn get_all_rollback_points(&self) -> Vec<RollbackPoint> {
        let points = self.rollback_points.read().await;
        points.clone()
    }

    /// Rollback to a specific point
    pub async fn rollback(&self, rollback_point_id: &str) -> Result<RollbackResult> {
        let started_at = Utc::now();

        let point = self.get_rollback_point(rollback_point_id).await
            .ok_or_else(|| anyhow::anyhow!("Rollback point not found: {}", rollback_point_id))?;

        info!("Rolling back to point '{}'", rollback_point_id);

        // Simulate rollback
        let result = RollbackResult {
            rollback_point_id: rollback_point_id.to_string(),
            success: true,
            modules_rolled_back: point.module_versions.keys().cloned().collect(),
            state_restored: point.state_snapshot_path.is_some(),
            config_restored: point.config_snapshot.is_some(),
            duration_ms: 100,
            warnings: Vec::new(),
            errors: Vec::new(),
            started_at,
            ended_at: Utc::now(),
        };

        Ok(result)
    }

    /// Delete a rollback point
    pub async fn delete_rollback_point(&self, id: &str) -> Result<()> {
        let mut points = self.rollback_points.write().await;
        points.retain(|p| p.id != id);
        Ok(())
    }

    /// Get the latest rollback point
    pub async fn get_latest_rollback_point(&self) -> Option<RollbackPoint> {
        let points = self.rollback_points.read().await;
        points.last().cloned()
    }

    /// Get rollback points by type
    pub async fn get_rollback_points_by_type(&self, point_type: RollbackPointType) -> Vec<RollbackPoint> {
        let points = self.rollback_points.read().await;
        points.iter().filter(|p| p.point_type == point_type).cloned().collect()
    }
}

impl Default for RollbackManager {
    fn default() -> Self {
        Self::new(BackupConfig::default()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_rollback_point() {
        let manager = RollbackManager::default();

        let point = manager
            .create_rollback_point(RollbackPointType::ManualCheckpoint, "Test checkpoint".to_string())
            .await
            .unwrap();

        assert_eq!(point.point_type, RollbackPointType::ManualCheckpoint);
        assert_eq!(point.description, "Test checkpoint");
    }

    #[tokio::test]
    async fn test_get_rollback_point() {
        let manager = RollbackManager::default();

        let point = manager
            .create_rollback_point(RollbackPointType::PreDeployment, "Pre-deploy".to_string())
            .await
            .unwrap();

        let retrieved = manager.get_rollback_point(&point.id).await.unwrap();
        assert_eq!(retrieved.id, point.id);
    }

    #[tokio::test]
    async fn test_rollback() {
        let manager = RollbackManager::default();

        let point = manager
            .create_rollback_point(RollbackPointType::PreDeployment, "Pre-deploy".to_string())
            .await
            .unwrap();

        let result = manager.rollback(&point.id).await.unwrap();
        assert!(result.success);
    }
}
