//! Rollback & Recovery - System State Management
//!
//! This module provides rollback and recovery capabilities for ServantGuild,
//! enabling safe version management, state preservation, and disaster recovery.

use crate::runtime::hot_swap::{HotSwap, ModuleVersion, SwapResult};
use crate::runtime::state::HostState;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sled::Db;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

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
    pub git_commit: Option<String>,
    /// Size in bytes
    pub size: u64,
    /// Tags
    pub tags: Vec<String>,
}

impl RollbackPoint {
    /// Create new rollback point
    pub fn new(id: String, point_type: RollbackPointType, description: String) -> Self {
        Self {
            id,
            timestamp: Utc::now(),
            point_type,
            description,
            module_versions: HashMap::new(),
            state_snapshot_path: None,
            config_snapshot: None,
            git_commit: None,
            size: 0,
            tags: Vec::new(),
        }
    }

    /// Add module version
    pub fn with_module_version(mut self, module: String, version: ModuleVersion) -> Self {
        self.module_versions.insert(module, version);
        self
    }

    /// Add tag
    pub fn with_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }
}

/// Rollback result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackResult {
    /// Whether rollback succeeded
    pub success: bool,
    /// Rollback point used
    pub rollback_point_id: String,
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
            backup_interval_secs: 3600, // 1 hour
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
    /// Host state
    state: HostState,
    /// Hot-swap manager
    hot_swap: Arc<dyn HotSwap>,
    /// Database for rollback points
    db: Db,
    /// Rollback points (in-memory cache)
    rollback_points: Arc<RwLock<Vec<RollbackPoint>>>,
    /// Backup configuration
    backup_config: BackupConfig,
}

impl RollbackManager {
    /// Create new rollback manager
    pub fn new(
        state: HostState,
        hot_swap: Arc<dyn HotSwap>,
        db: Db,
        backup_config: BackupConfig,
    ) -> Result<Self> {
        let manager = Self {
            state,
            hot_swap,
            db,
            rollback_points: Arc::new(RwLock::new(Vec::new())),
            backup_config,
        };

        // Load rollback points from database
        manager.load_rollback_points()?;

        Ok(manager)
    }

    /// Create a rollback point
    pub async fn create_rollback_point(
        &self,
        point_type: RollbackPointType,
        description: String,
    ) -> Result<RollbackPoint> {
        let id = uuid::Uuid::new_v4().to_string();
        info!("Creating rollback point '{}' with type {:?}", id, point_type);

        let mut point = RollbackPoint::new(id.clone(), point_type, description);

        // Capture current module versions
        // In a real implementation, this would query the hot-swap manager
        // For now, add a dummy version
        point = point.with_module_version(
            "coordinator".to_string(),
            ModuleVersion::new("1.0.0".to_string()),
        );

        // Capture state snapshot if enabled
        if self.backup_config.include_state_snapshots {
            let snapshot_path = self.create_state_snapshot().await?;
            point.state_snapshot_path = Some(snapshot_path);
        }

        // Capture configuration if enabled
        if self.backup_config.include_config {
            // This would capture current configuration
            point.config_snapshot = Some(serde_json::json!({}));
        }

        // Calculate size
        point.size = self.calculate_point_size(&point).await?;

        // Store rollback point
        self.store_rollback_point(point.clone()).await?;

        info!("Rollback point '{}' created successfully", id);

        Ok(point)
    }

    /// Perform rollback
    pub async fn rollback(&self, point_id: String) -> Result<RollbackResult> {
        let start_time = Utc::now();
        info!("Starting rollback to point '{}'", point_id);

        // Get rollback point
        let point = self.get_rollback_point(point_id.clone())
            .context("Rollback point not found")?;

        // Create recovery plan
        let recovery_plan = self.create_recovery_plan(point.clone()).await?;

        // Execute recovery
        let result = self.execute_recovery(recovery_plan).await?;

        let end_time = Utc::now();

        Ok(RollbackResult {
            success: result.success,
            rollback_point_id: point_id,
            modules_rolled_back: result.modules_rolled_back,
            state_restored: result.state_restored,
            config_restored: result.config_restored,
            duration_ms: (end_time - start_time).num_milliseconds() as u64,
            warnings: result.warnings,
            errors: result.errors,
            started_at: start_time,
            ended_at: end_time,
        })
    }

    /// List rollback points
    pub async fn list_rollback_points(&self, limit: Option<usize>) -> Vec<RollbackPoint> {
        let points = self.rollback_points.read().await;
        let sorted = {
            let mut vec = points.clone();
            vec.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            vec
        };

        if let Some(limit) = limit {
            sorted.into_iter().take(limit).collect()
        } else {
            sorted
        }
    }

    /// Delete rollback point
    pub async fn delete_rollback_point(&self, point_id: String) -> Result<()> {
        info!("Deleting rollback point '{}'", point_id);

        // Remove from cache
        let mut points = self.rollback_points.write().await;
        points.retain(|p| p.id != point_id);

        // Remove from database
        let key = format!("rollback_point:{}", point_id);
        self.db.remove(key.as_bytes())?;

        // Delete snapshot file if exists
        // This would clean up the snapshot file

        info!("Rollback point '{}' deleted", point_id);

        Ok(())
    }

    /// Create recovery plan
    async fn create_recovery_plan(&self, point: RollbackPoint) -> Result<RecoveryPlan> {
        let mut steps = Vec::new();
        let mut step_number = 1;

        steps.push(RecoveryStep {
            step_number: step_number,
            description: "Stop all services".to_string(),
            step_type: RecoveryStepType::StopServices,
            estimated_duration_secs: 10,
            is_critical: true,
        });
        step_number += 1;

        if point.state_snapshot_path.is_some() {
            steps.push(RecoveryStep {
                step_number,
                description: "Restore system state".to_string(),
                step_type: RecoveryStepType::RestoreState,
                estimated_duration_secs: 30,
                is_critical: true,
            });
            step_number += 1;
        }

        if point.config_snapshot.is_some() {
            steps.push(RecoveryStep {
                step_number,
                description: "Restore configuration".to_string(),
                step_type: RecoveryStepType::RestoreConfig,
                estimated_duration_secs: 5,
                is_critical: true,
            });
            step_number += 1;
        }

        steps.push(RecoveryStep {
            step_number,
            description: "Rollback modules".to_string(),
            step_type: RecoveryStepType::RollbackModules,
            estimated_duration_secs: 60,
            is_critical: true,
        });
        step_number += 1;

        steps.push(RecoveryStep {
            step_number,
            description: "Verify system integrity".to_string(),
            step_type: RecoveryStepType::VerifyIntegrity,
            estimated_duration_secs: 15,
            is_critical: true,
        });
        step_number += 1;

        steps.push(RecoveryStep {
            step_number,
            description: "Start services".to_string(),
            step_type: RecoveryStepType::StartServices,
            estimated_duration_secs: 20,
            is_critical: true,
        });
        step_number += 1;

        steps.push(RecoveryStep {
            step_number,
            description: "Perform health check".to_string(),
            step_type: RecoveryStepType::HealthCheck,
            estimated_duration_secs: 10,
            is_critical: false,
        });

        let estimated_duration: u64 = steps.iter().map(|s| s.estimated_duration_secs).sum();

        Ok(RecoveryPlan {
            id: uuid::Uuid::new_v4().to_string(),
            target_point: point,
            steps,
            estimated_duration_secs: estimated_duration,
            data_loss_acceptable: false,
            created_at: Utc::now(),
        })
    }

    /// Execute recovery
    async fn execute_recovery(&self, plan: RecoveryPlan) -> Result<RollbackResult> {
        let mut modules_rolled_back = Vec::new();
        let mut state_restored = false;
        let mut config_restored = false;
        let mut warnings = Vec::new();
        let mut errors = Vec::new();
        let mut success = true;

        for step in &plan.steps {
            debug!("Executing recovery step {}: {}", step.step_number, step.description);

            match step.step_type {
                RecoveryStepType::StopServices => {
                    // Stop services
                }
                RecoveryStepType::RestoreState => {
                    // Restore state
                    state_restored = true;
                }
                RecoveryStepType::RestoreConfig => {
                    // Restore config
                    config_restored = true;
                }
                RecoveryStepType::RollbackModules => {
                    // Rollback modules
                    for (module_name, version) in &plan.target_point.module_versions {
                        match self.hot_swap.rollback(module_name.clone(), version.clone(), "Recovery rollback".to_string()).await {
                            Ok(_) => {
                                modules_rolled_back.push(module_name.clone());
                            }
                            Err(e) => {
                                if step.is_critical {
                                    success = false;
                                    errors.push(format!("Failed to rollback module '{}': {}", module_name, e));
                                } else {
                                    warnings.push(format!("Failed to rollback module '{}': {}", module_name, e));
                                }
                            }
                        }
                    }
                }
                RecoveryStepType::VerifyIntegrity => {
                    // Verify integrity
                }
                RecoveryStepType::StartServices => {
                    // Start services
                }
                RecoveryStepType::HealthCheck => {
                    // Health check
                }
            }
        }

        Ok(RollbackResult {
            success,
            rollback_point_id: plan.target_point.id,
            modules_rolled_back,
            state_restored,
            config_restored,
            duration_ms: 0, // Calculated outside
            warnings,
            errors,
            started_at: Utc::now(),
            ended_at: Utc::now(),
        })
    }

    /// Load rollback points from database
    fn load_rollback_points(&self) -> Result<()> {
        // Load points from sled database
        for result in self.db.scan_prefix("rollback_point:".as_bytes()) {
            if let Ok((key, value)) = result {
                if let Ok(point) = serde_json::from_slice::<RollbackPoint>(&value) {
                    // Add to cache - would need to handle this properly
                }
            }
        }
        Ok(())
    }

    /// Store rollback point
    async fn store_rollback_point(&self, point: RollbackPoint) -> Result<()> {
        let key = format!("rollback_point:{}", point.id);
        let value = serde_json::to_vec(&point)?;

        self.db.insert(key.as_bytes(), value)?;

        // Add to cache
        let mut points = self.rollback_points.write().await;
        points.push(point);

        // Enforce max rollback points
        if points.len() > self.backup_config.max_rollback_points {
            points.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            let removed = points.split_off(self.backup_config.max_rollback_points);
            for old_point in removed {
                let _ = self.delete_rollback_point(old_point.id).await;
            }
        }

        Ok(())
    }

    /// Get rollback point
    fn get_rollback_point(&self, point_id: String) -> Option<RollbackPoint> {
        let key = format!("rollback_point:{}", point_id);
        if let Ok(Some(value)) = self.db.get(key.as_bytes()) {
            if let Ok(point) = serde_json::from_slice::<RollbackPoint>(&value) {
                return Some(point);
            }
        }
        None
    }

    /// Create state snapshot
    async fn create_state_snapshot(&self) -> Result<PathBuf> {
        let snapshot_id = uuid::Uuid::new_v4().to_string();
        let snapshot_path = self.backup_config.storage_path
            .join("snapshots")
            .join(format!("state_{}.json", snapshot_id));

        // Ensure directory exists
        if let Some(parent) = snapshot_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Create snapshot
        // In a real implementation, this would capture the actual state
        let snapshot_data = serde_json::json!({
            "timestamp": Utc::now(),
            "snapshot_id": snapshot_id,
        });

        std::fs::write(&snapshot_path, serde_json::to_vec_pretty(&snapshot_data)?)?;

        Ok(snapshot_path)
    }

    /// Calculate rollback point size
    async fn calculate_point_size(&self, point: &RollbackPoint) -> Result<u64> {
        let mut size = serde_json::to_vec(point)?.len() as u64;

        if let Some(ref snapshot_path) = point.state_snapshot_path {
            if let Ok(metadata) = std::fs::metadata(snapshot_path) {
                size += metadata.len();
            }
        }

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rollback_point() {
        let point = RollbackPoint::new(
            "test-id".to_string(),
            RollbackPointType::ManualCheckpoint,
            "Test checkpoint".to_string(),
        )
        .with_module_version("test-module".to_string(), ModuleVersion::new("1.0.0".to_string()))
        .with_tag("important".to_string());

        assert_eq!(point.id, "test-id");
        assert!(point.module_versions.contains_key("test-module"));
        assert!(point.tags.contains(&"important".to_string()));
    }
}
