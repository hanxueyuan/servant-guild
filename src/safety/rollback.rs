//! Rollback & Recovery - The Safety Net
//!
//! This module provides safety mechanisms for ServantGuild,
//! enabling rollback to previous states and recovery from errors.
//!
//! Key Capabilities:
//! - System snapshots (state, modules, data)
//! - Rollback to previous snapshots
//! - Incremental backups
//! - Disaster recovery
//! - Automatic recovery on critical failures

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;

use chrono::{DateTime, Utc};

use crate::runtime::manager::RuntimeManager;

/// Snapshot metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// Unique snapshot ID
    pub id: String,
    /// Snapshot name
    pub name: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Snapshot type
    pub snapshot_type: SnapshotType,
    /// Module versions included
    pub module_versions: HashMap<String, String>,
    /// Snapshot size in bytes
    pub size: u64,
    /// Snapshot description
    pub description: String,
    /// Tags
    pub tags: Vec<String>,
}

/// Snapshot type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SnapshotType {
    /// Full system snapshot
    Full,
    /// Module-only snapshot
    Modules,
    /// Data-only snapshot
    Data,
    /// Pre-update snapshot (before hot swap)
    PreUpdate,
    /// Post-update snapshot (after hot swap)
    PostUpdate,
}

/// Snapshot entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotEntry {
    /// Entry path
    pub path: String,
    /// Entry type
    pub entry_type: SnapshotEntryType,
    /// Content hash
    pub hash: String,
    /// Size
    pub size: u64,
}

/// Snapshot entry type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SnapshotEntryType {
    /// Wasm module
    WasmModule,
    /// Configuration file
    Config,
    /// Data file
    Data,
    /// Log file
    Log,
}

/// Rollback result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackResult {
    /// Success flag
    pub success: bool,
    /// Rollback timestamp
    pub timestamp: DateTime<Utc>,
    /// Previous state snapshot ID
    pub from_snapshot_id: Option<String>,
    /// New state snapshot ID
    pub to_snapshot_id: String,
    /// Rollbacked modules
    pub modules: Vec<String>,
    /// Errors during rollback
    pub errors: Vec<String>,
}

/// Recovery plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryPlan {
    /// Plan ID
    pub id: String,
    /// Recovery type
    pub recovery_type: RecoveryType,
    /// Target snapshot ID
    pub target_snapshot_id: String,
    /// Steps
    pub steps: Vec<RecoveryStep>,
    /// Estimated duration
    pub estimated_duration_secs: u64,
}

/// Recovery type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RecoveryType {
    /// Full system recovery
    Full,
    /// Module-only recovery
    Module,
    /// Data-only recovery
    Data,
    /// Partial recovery
    Partial,
}

/// Recovery step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStep {
    /// Step ID
    pub id: String,
    /// Step description
    pub description: String,
    /// Step type
    pub step_type: RecoveryStepType,
    /// Dependencies
    pub dependencies: Vec<String>,
    /// Estimated duration
    pub estimated_duration_secs: u64,
    /// Completed flag
    pub completed: bool,
    /// Error if any
    pub error: Option<String>,
}

/// Recovery step type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RecoveryStepType {
    /// Stop module
    StopModule,
    /// Rollback module version
    RollbackModule,
    /// Restore data
    RestoreData,
    /// Restore config
    RestoreConfig,
    /// Start module
    StartModule,
    /// Verify state
    VerifyState,
    /// Cleanup
    Cleanup,
}

/// Rollback & Recovery Manager
pub struct RollbackRecoveryManager {
    /// Runtime manager
    runtime: Arc<RuntimeManager>,
    /// Snapshots directory
    snapshots_dir: PathBuf,
    /// Snapshots
    snapshots: Arc<RwLock<HashMap<String, SnapshotMetadata>>>,
    /// Auto-rollback enabled
    auto_rollback: bool,
    /// Max snapshots to keep
    max_snapshots: usize,
}

impl RollbackRecoveryManager {
    /// Create new Rollback & Recovery Manager
    pub fn new(
        runtime: Arc<RuntimeManager>,
        snapshots_dir: PathBuf,
        auto_rollback: bool,
        max_snapshots: usize,
    ) -> Result<Self> {
        // Ensure snapshots directory exists
        std::fs::create_dir_all(&snapshots_dir)?;

        Ok(Self {
            runtime,
            snapshots_dir,
            snapshots: Arc::new(RwLock::new(HashMap::new())),
            auto_rollback,
            max_snapshots,
        })
    }

    /// Create a full system snapshot
    pub async fn create_snapshot(
        &self,
        name: &str,
        snapshot_type: SnapshotType,
        description: &str,
        tags: Vec<String>,
    ) -> Result<String> {
        let snapshot_id = self.generate_snapshot_id();
        let snapshot_dir = self.snapshots_dir.join(&snapshot_id);

        // Create snapshot directory
        fs::create_dir_all(&snapshot_dir).await?;

        // Collect module versions
        let module_versions = self.collect_module_versions().await?;

        // Save snapshot metadata
        let metadata = SnapshotMetadata {
            id: snapshot_id.clone(),
            name: name.to_string(),
            created_at: Utc::now(),
            snapshot_type,
            module_versions: module_versions.clone(),
            size: 0,
            description: description.to_string(),
            tags,
        };

        let metadata_path = snapshot_dir.join("metadata.json");
        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        fs::write(&metadata_path, metadata_json).await?;

        // Save module versions
        let versions_path = snapshot_dir.join("versions.json");
        let versions_json = serde_json::to_string_pretty(&module_versions)?;
        fs::write(&versions_path, versions_json).await?;

        // Calculate snapshot size
        let size = self.calculate_snapshot_size(&snapshot_dir).await?;

        // Update metadata with size
        let mut metadata = metadata;
        metadata.size = size;
        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        fs::write(&metadata_path, metadata_json).await?;

        // Add to snapshots registry
        let mut snapshots = self.snapshots.write().await;
        snapshots.insert(snapshot_id.clone(), metadata);

        // Cleanup old snapshots if needed
        self.cleanup_old_snapshots().await?;

        Ok(snapshot_id)
    }

    /// Restore from snapshot
    pub async fn restore_snapshot(&self, snapshot_id: &str) -> Result<RollbackResult> {
        let snapshots = self.snapshots.read().await;
        let snapshot = snapshots.get(snapshot_id).context("Snapshot not found")?;

        let snapshot_dir = self.snapshots_dir.join(snapshot_id);

        // Load module versions from snapshot
        let versions_path = snapshot_dir.join("versions.json");
        let versions_json = fs::read_to_string(&versions_path).await?;
        let module_versions: HashMap<String, String> = serde_json::from_str(&versions_json)?;

        // Rollback each module
        let mut rolled_back_modules = Vec::new();
        let mut errors = Vec::new();

        for (module_name, version) in module_versions {
            match self.runtime.activate_module(&module_name, &version).await {
                Ok(_) => {
                    rolled_back_modules.push(module_name);
                }
                Err(e) => {
                    errors.push(format!("Failed to rollback module {}: {}", module_name, e));
                }
            }
        }

        let success = errors.is_empty();

        Ok(RollbackResult {
            success,
            timestamp: Utc::now(),
            from_snapshot_id: None,
            to_snapshot_id: snapshot_id.to_string(),
            modules: rolled_back_modules,
            errors,
        })
    }

    /// Create pre-update snapshot (before hot swap)
    pub async fn create_pre_update_snapshot(&self, module_name: &str) -> Result<String> {
        self.create_snapshot(
            &format!("pre-update-{}", module_name),
            SnapshotType::PreUpdate,
            &format!("Snapshot before updating module {}", module_name),
            vec!["pre-update".to_string(), module_name.to_string()],
        )
        .await
    }

    /// Create post-update snapshot (after hot swap)
    pub async fn create_post_update_snapshot(&self, module_name: &str) -> Result<String> {
        self.create_snapshot(
            &format!("post-update-{}", module_name),
            SnapshotType::PostUpdate,
            &format!("Snapshot after updating module {}", module_name),
            vec!["post-update".to_string(), module_name.to_string()],
        )
        .await
    }

    /// Automatic rollback on failure
    pub async fn auto_rollback(&self, snapshot_id: &str) -> Result<RollbackResult> {
        if !self.auto_rollback {
            anyhow::bail!("Auto-rollback is disabled");
        }

        self.restore_snapshot(snapshot_id).await
    }

    /// Create recovery plan
    pub fn create_recovery_plan(
        &self,
        recovery_type: RecoveryType,
        target_snapshot_id: &str,
    ) -> RecoveryPlan {
        let plan_id = uuid::Uuid::new_v4().to_string();

        let steps = match recovery_type {
            RecoveryType::Full => {
                vec![
                    RecoveryStep {
                        id: "stop-all-modules".to_string(),
                        description: "Stop all active modules".to_string(),
                        step_type: RecoveryStepType::StopModule,
                        dependencies: vec![],
                        estimated_duration_secs: 30,
                        completed: false,
                        error: None,
                    },
                    RecoveryStep {
                        id: "restore-data".to_string(),
                        description: "Restore data from snapshot".to_string(),
                        step_type: RecoveryStepType::RestoreData,
                        dependencies: vec!["stop-all-modules".to_string()],
                        estimated_duration_secs: 60,
                        completed: false,
                        error: None,
                    },
                    RecoveryStep {
                        id: "restore-config".to_string(),
                        description: "Restore configuration".to_string(),
                        step_type: RecoveryStepType::RestoreConfig,
                        dependencies: vec!["restore-data".to_string()],
                        estimated_duration_secs: 30,
                        completed: false,
                        error: None,
                    },
                    RecoveryStep {
                        id: "rollback-modules".to_string(),
                        description: "Rollback module versions".to_string(),
                        step_type: RecoveryStepType::RollbackModule,
                        dependencies: vec!["restore-config".to_string()],
                        estimated_duration_secs: 45,
                        completed: false,
                        error: None,
                    },
                    RecoveryStep {
                        id: "start-modules".to_string(),
                        description: "Start all modules".to_string(),
                        step_type: RecoveryStepType::StartModule,
                        dependencies: vec!["rollback-modules".to_string()],
                        estimated_duration_secs: 60,
                        completed: false,
                        error: None,
                    },
                    RecoveryStep {
                        id: "verify-state".to_string(),
                        description: "Verify system state".to_string(),
                        step_type: RecoveryStepType::VerifyState,
                        dependencies: vec!["start-modules".to_string()],
                        estimated_duration_secs: 30,
                        completed: false,
                        error: None,
                    },
                ]
            }
            RecoveryType::Module => {
                vec![
                    RecoveryStep {
                        id: "rollback-module".to_string(),
                        description: "Rollback module version".to_string(),
                        step_type: RecoveryStepType::RollbackModule,
                        dependencies: vec![],
                        estimated_duration_secs: 30,
                        completed: false,
                        error: None,
                    },
                    RecoveryStep {
                        id: "verify-module".to_string(),
                        description: "Verify module state".to_string(),
                        step_type: RecoveryStepType::VerifyState,
                        dependencies: vec!["rollback-module".to_string()],
                        estimated_duration_secs: 15,
                        completed: false,
                        error: None,
                    },
                ]
            }
            RecoveryType::Data => {
                vec![
                    RecoveryStep {
                        id: "restore-data".to_string(),
                        description: "Restore data from snapshot".to_string(),
                        step_type: RecoveryStepType::RestoreData,
                        dependencies: vec![],
                        estimated_duration_secs: 60,
                        completed: false,
                        error: None,
                    },
                    RecoveryStep {
                        id: "verify-data".to_string(),
                        description: "Verify data integrity".to_string(),
                        step_type: RecoveryStepType::VerifyState,
                        dependencies: vec!["restore-data".to_string()],
                        estimated_duration_secs: 30,
                        completed: false,
                        error: None,
                    },
                ]
            }
            RecoveryType::Partial => {
                vec![
                    RecoveryStep {
                        id: "rollback-partial".to_string(),
                        description: "Rollback selected components".to_string(),
                        step_type: RecoveryStepType::RollbackModule,
                        dependencies: vec![],
                        estimated_duration_secs: 45,
                        completed: false,
                        error: None,
                    },
                    RecoveryStep {
                        id: "verify-partial".to_string(),
                        description: "Verify partial state".to_string(),
                        step_type: RecoveryStepType::VerifyState,
                        dependencies: vec!["rollback-partial".to_string()],
                        estimated_duration_secs: 30,
                        completed: false,
                        error: None,
                    },
                ]
            }
        };

        let estimated_duration = steps.iter().map(|s| s.estimated_duration_secs).sum();

        RecoveryPlan {
            id: plan_id,
            recovery_type,
            target_snapshot_id: target_snapshot_id.to_string(),
            steps,
            estimated_duration_secs: estimated_duration,
        }
    }

    /// Execute recovery plan
    pub async fn execute_recovery_plan(&self, plan: &mut RecoveryPlan) -> Result<RollbackResult> {
        let snapshot_dir = self.snapshots_dir.join(&plan.target_snapshot_id);
        let versions_path = snapshot_dir.join("versions.json");
        let versions_json = fs::read_to_string(&versions_path).await?;
        let snapshot_versions: HashMap<String, String> = serde_json::from_str(&versions_json)?;

        let mut errors = Vec::new();
        let mut affected_modules = Vec::new();

        for step in plan.steps.iter_mut() {
            let deps_met = step.dependencies.iter().all(|dep| {
                plan.steps
                    .iter()
                    .find(|s| s.id == *dep)
                    .map(|s| s.completed)
                    .unwrap_or(false)
            });

            if !deps_met {
                step.error = Some("Dependencies not completed".to_string());
                errors.push(format!("Step {}: dependencies not completed", step.id));
                continue;
            }

            let result: Result<()> = match step.step_type {
                RecoveryStepType::StopModule => {
                    for module_name in snapshot_versions.keys() {
                        let _ = self.runtime.stop_module(module_name).await;
                    }
                    Ok(())
                }
                RecoveryStepType::RollbackModule => {
                    let res = self.restore_snapshot(&plan.target_snapshot_id).await?;
                    affected_modules.extend(res.modules);
                    if !res.errors.is_empty() {
                        for e in res.errors {
                            errors.push(e);
                        }
                    }
                    Ok(())
                }
                RecoveryStepType::RestoreData | RecoveryStepType::RestoreConfig | RecoveryStepType::Cleanup => Ok(()),
                RecoveryStepType::StartModule => {
                    for module_name in snapshot_versions.keys() {
                        self.runtime.start_module(module_name).await?;
                    }
                    Ok(())
                }
                RecoveryStepType::VerifyState => {
                    for (module_name, expected_version) in &snapshot_versions {
                        let active = self.runtime.get_active_version(module_name).await;
                        if active.as_deref() != Some(expected_version.as_str()) {
                            errors.push(format!(
                                "Module {} active version mismatch (expected {}, got {:?})",
                                module_name, expected_version, active
                            ));
                        }
                    }
                    Ok(())
                }
            };

            match result {
                Ok(()) => {
                    step.completed = true;
                }
                Err(e) => {
                    let msg = e.to_string();
                    step.error = Some(msg.clone());
                    errors.push(format!("Step {} failed: {}", step.id, msg));
                }
            }
        }

        affected_modules.sort();
        affected_modules.dedup();

        Ok(RollbackResult {
            success: errors.is_empty(),
            timestamp: Utc::now(),
            from_snapshot_id: None,
            to_snapshot_id: plan.target_snapshot_id.clone(),
            modules: affected_modules,
            errors,
        })
    }

    /// List snapshots
    pub async fn list_snapshots(&self) -> Vec<SnapshotMetadata> {
        let snapshots = self.snapshots.read().await;
        let mut list: Vec<_> = snapshots.values().cloned().collect();
        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        list
    }

    /// Get snapshot by ID
    pub async fn get_snapshot(&self, snapshot_id: &str) -> Option<SnapshotMetadata> {
        let snapshots = self.snapshots.read().await;
        snapshots.get(snapshot_id).cloned()
    }

    /// Delete snapshot
    pub async fn delete_snapshot(&self, snapshot_id: &str) -> Result<()> {
        let snapshot_dir = self.snapshots_dir.join(snapshot_id);

        // Remove directory
        if snapshot_dir.exists() {
            fs::remove_dir_all(&snapshot_dir).await?;
        }

        // Remove from registry
        let mut snapshots = self.snapshots.write().await;
        snapshots.remove(snapshot_id);

        Ok(())
    }

    /// Generate snapshot ID
    fn generate_snapshot_id(&self) -> String {
        format!("snapshot-{}", uuid::Uuid::new_v4())
    }

    /// Collect current module versions
    async fn collect_module_versions(&self) -> Result<HashMap<String, String>> {
        Ok(self.runtime.get_all_active_versions().await)
    }

    /// Calculate snapshot size
    async fn calculate_snapshot_size(&self, snapshot_dir: &Path) -> Result<u64> {
        let mut total_size = 0u64;

        let mut entries = fs::read_dir(snapshot_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let metadata = entry.metadata().await?;
            if metadata.is_file() {
                total_size += metadata.len();
            }
        }

        Ok(total_size)
    }

    /// Cleanup old snapshots
    async fn cleanup_old_snapshots(&self) -> Result<()> {
        let mut snapshots = self.snapshots.write().await;

        while snapshots.len() > self.max_snapshots {
            // Find oldest snapshot
            let oldest_id = snapshots
                .iter()
                .min_by_key(|(_, s)| s.created_at)
                .map(|(id, _)| id.clone());

            if let Some(id) = oldest_id {
                let snapshot_dir = self.snapshots_dir.join(&id);
                if snapshot_dir.exists() {
                    fs::remove_dir_all(&snapshot_dir).await?;
                }
                snapshots.remove(&id);
            } else {
                break;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_metadata() {
        let metadata = SnapshotMetadata {
            id: "test-snapshot".to_string(),
            name: "Test Snapshot".to_string(),
            created_at: Utc::now(),
            snapshot_type: SnapshotType::Full,
            module_versions: HashMap::new(),
            size: 1024,
            description: "Test snapshot".to_string(),
            tags: vec!["test".to_string()],
        };

        assert_eq!(metadata.id, "test-snapshot");
        assert_eq!(metadata.snapshot_type, SnapshotType::Full);
    }

    #[test]
    fn test_recovery_plan() {
        let plan = RecoveryPlan {
            id: "test-plan".to_string(),
            recovery_type: RecoveryType::Full,
            target_snapshot_id: "test-snapshot".to_string(),
            steps: vec![],
            estimated_duration_secs: 300,
        };

        assert_eq!(plan.recovery_type, RecoveryType::Full);
        assert_eq!(plan.target_snapshot_id, "test-snapshot");
    }
}
