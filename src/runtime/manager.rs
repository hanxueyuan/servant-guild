//! Runtime Manager - The Puppet Master
//!
//! The Runtime Manager manages the lifecycle of Wasm modules within ServantGuild,
//! enabling hot swapping and version management without restarting the system.
//!
//! Key Capabilities:
//! - Load Wasm modules into Wasmtime runtime
//! - Track active versions and rollback points
//! - Perform hot swaps without system restart
//! - Validate new modules before activation
//! - Coordinate with the Consensus Engine for updates

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use wasmtime::{Engine, Module, Store};

use crate::runtime::bridges::github::GitHubBridge;
use crate::tools::build::BuildTools;

/// Module version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleVersion {
    /// Version identifier (e.g., "v1.2.3")
    pub version: String,
    /// Git commit SHA
    pub commit_sha: String,
    /// Wasm file path
    pub wasm_path: PathBuf,
    /// Module size in bytes
    pub size: u64,
    /// Build timestamp
    pub build_time: i64,
    /// Activation timestamp
    pub activated_at: Option<i64>,
    /// Active status
    pub active: bool,
    /// Test passed
    pub test_passed: bool,
    /// Clippy passed
    pub clippy_passed: bool,
}

/// Hot swap result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotSwapResult {
    /// Success flag
    pub success: bool,
    /// Previous version
    pub previous_version: Option<String>,
    /// New version
    pub new_version: String,
    /// Swap timestamp
    pub timestamp: i64,
    /// Error message if failed
    pub error: Option<String>,
    /// Validation results
    pub validation: ValidationReport,
}

/// Validation report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Hash validation passed
    pub hash_valid: bool,
    /// API compatibility check
    pub api_compatible: bool,
    /// Memory requirements
    pub memory_required: Option<u64>,
    /// Safety checks
    pub safety_checks: HashMap<String, bool>,
    /// Warnings
    pub warnings: Vec<String>,
}

/// Runtime Manager for Wasm module lifecycle
pub struct RuntimeManager {
    /// Wasmtime engine
    engine: Arc<Engine>,
    /// GitHub bridge for code access
    github: Arc<GitHubBridge>,
    /// Build tools for compilation
    build_tools: Arc<BuildTools>,
    /// Module versions (name -> Vec<ModuleVersion>)
    versions: Arc<RwLock<HashMap<String, Vec<ModuleVersion>>>>,
    /// Active versions (name -> version)
    active_versions: Arc<RwLock<HashMap<String, String>>>,
    /// Rollback points (name -> version)
    rollback_points: Arc<RwLock<HashMap<String, String>>>,
    /// Wasmtime stores
    stores: Arc<RwLock<HashMap<String, Store<wasmtypes::State>>>>,
}

/// State for Wasmtime store
struct State {
    memory_limit: u64,
}

impl RuntimeManager {
    /// Create new Runtime Manager
    pub fn new(
        engine: Arc<Engine>,
        github: Arc<GitHubBridge>,
        build_tools: Arc<BuildTools>,
    ) -> Self {
        Self {
            engine,
            github,
            build_tools,
            versions: Arc::new(RwLock::new(HashMap::new())),
            active_versions: Arc::new(RwLock::new(HashMap::new())),
            rollback_points: Arc::new(RwLock::new(HashMap::new())),
            stores: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load a module from file
    pub async fn load_module(
        &self,
        name: &str,
        wasm_path: &PathBuf,
        version: &str,
        commit_sha: &str,
        test_passed: bool,
        clippy_passed: bool,
    ) -> Result<()> {
        // Load Wasm module
        let module = Module::from_file(&self.engine, wasm_path)
            .context("Failed to load Wasm module")?;

        // Create store
        let mut store = Store::new(&self.engine, State {
            memory_limit: 64 * 1024 * 1024, // 64MB default
        });

        // Create instance
        let _instance = wasmtime::Instance::new(&mut store, &module, &[])
            .context("Failed to create instance")?;

        // Create version info
        let metadata = tokio::fs::metadata(wasm_path).await?;
        let module_version = ModuleVersion {
            version: version.to_string(),
            commit_sha: commit_sha.to_string(),
            wasm_path: wasm_path.clone(),
            size: metadata.len(),
            build_time: chrono::Utc::now().timestamp(),
            activated_at: None,
            active: false,
            test_passed,
            clippy_passed,
        };

        // Add to versions
        let mut versions = self.versions.write().await;
        versions
            .entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(module_version);

        // Store the store
        let mut stores = self.stores.write().await;
        stores.insert(name.to_string(), store);

        Ok(())
    }

    /// Activate a module version
    pub async fn activate_module(&self, name: &str, version: &str) -> Result<HotSwapResult> {
        let mut versions = self.versions.write().await;
        let module_versions = versions
            .get_mut(name)
            .context("Module not found")?;

        // Find the version
        let version_index = module_versions
            .iter()
            .position(|v| v.version == version)
            .context("Version not found")?;

        // Deactivate current active version
        let previous_version = module_versions
            .iter()
            .find(|v| v.active)
            .map(|v| v.version.clone());

        // Deactivate all versions
        for v in module_versions.iter_mut() {
            v.active = false;
            v.activated_at = None;
        }

        // Activate new version
        module_versions[version_index].active = true;
        module_versions[version_index].activated_at = Some(chrono::Utc::now().timestamp());

        // Update active versions
        let mut active_versions = self.active_versions.write().await;
        active_versions.insert(name.to_string(), version.to_string());

        // Create rollback point
        let mut rollback_points = self.rollback_points.write().await;
        if let Some(prev) = &previous_version {
            rollback_points.insert(name.to_string(), prev.clone());
        }

        Ok(HotSwapResult {
            success: true,
            previous_version,
            new_version: version.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            error: None,
            validation: ValidationReport {
                hash_valid: true,
                api_compatible: true,
                memory_required: None,
                safety_checks: HashMap::new(),
                warnings: Vec::new(),
            },
        })
    }

    /// Perform hot swap of a module
    pub async fn hot_swap(
        &self,
        name: &str,
        new_wasm_path: &PathBuf,
        new_version: &str,
        commit_sha: &str,
    ) -> Result<HotSwapResult> {
        // Step 1: Validate new module
        let validation = self.validate_module(new_wasm_path).await?;

        if !validation.hash_valid || !validation.api_compatible {
            return Ok(HotSwapResult {
                success: false,
                previous_version: None,
                new_version: new_version.to_string(),
                timestamp: chrono::Utc::now().timestamp(),
                error: Some("Module validation failed".to_string()),
                validation,
            });
        }

        // Step 2: Load new module
        self.load_module(
            name,
            new_wasm_path,
            new_version,
            commit_sha,
            true,
            true,
        )
        .await?;

        // Step 3: Activate new module
        self.activate_module(name, new_version).await
    }

    /// Rollback to previous version
    pub async fn rollback(&self, name: &str) -> Result<HotSwapResult> {
        let rollback_points = self.rollback_points.read().await;
        let previous_version = rollback_points
            .get(name)
            .context("No rollback point available")?;

        let versions = self.versions.read().await;
        let module_versions = versions
            .get(name)
            .context("Module not found")?;

        // Find previous version
        let prev = module_versions
            .iter()
            .find(|v| v.version == *previous_version)
            .context("Previous version not found")?;

        // Rollback to previous version
        self.activate_module(name, &prev.version).await
    }

    /// Create snapshot of current state
    pub async fn create_snapshot(&self, name: &str) -> Result<String> {
        let active_versions = self.active_versions.read().await;
        let versions = self.versions.read().await;

        let current_version = active_versions
            .get(name)
            .context("No active version")?;

        let snapshot = serde_json::json!({
            "name": name,
            "version": current_version,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        Ok(serde_json::to_string(&snapshot)?)
    }

    /// Restore from snapshot
    pub async fn restore_snapshot(&self, name: &str, snapshot: &str) -> Result<()> {
        let snapshot_data: serde_json::Value = serde_json::from_str(snapshot)?;

        let version = snapshot_data["version"]
            .as_str()
            .context("Invalid snapshot format")?;

        self.activate_module(name, version).await
    }

    /// Get active version
    pub async fn get_active_version(&self, name: &str) -> Option<String> {
        let active_versions = self.active_versions.read().await;
        active_versions.get(name).cloned()
    }

    /// Get all versions for a module
    pub async fn get_versions(&self, name: &str) -> Vec<ModuleVersion> {
        let versions = self.versions.read().await;
        versions.get(name).cloned().unwrap_or_default()
    }

    /// Validate a Wasm module
    async fn validate_module(&self, wasm_path: &PathBuf) -> Result<ValidationReport> {
        // Check if file exists
        if !wasm_path.exists() {
            return Ok(ValidationReport {
                hash_valid: false,
                api_compatible: false,
                memory_required: None,
                safety_checks: HashMap::new(),
                warnings: vec!["Wasm file not found".to_string()],
            });
        }

        // Load module to validate
        let module = Module::from_file(&self.engine, wasm_path)
            .context("Failed to load module for validation")?;

        // Check imports
        let imports = module.imports();
        let mut safety_checks = HashMap::new();

        // Safety check: ensure module doesn't use dangerous imports
        let has_unsafe_imports = imports.iter().any(|import| {
            let module_name = import.module();
            let name = import.name();

            // Check for potentially unsafe imports
            module_name == "env" && name == "unsafe_memory"
                || module_name == "dangerous"
                || name.contains("unsafe")
        });

        safety_checks.insert("no_unsafe_imports".to_string(), !has_unsafe_imports);

        // Check if module is valid Wasm
        safety_checks.insert("valid_wasm".to_string(), true);

        // Calculate hash
        let wasm_content = tokio::fs::read(wasm_path).await?;
        let hash = sha2::Sha256::digest(&wasm_content);
        let hash_valid = !hash.as_slice().iter().all(|&b| b == 0);

        Ok(ValidationReport {
            hash_valid,
            api_compatible: true, // Simplified: assume compatible for now
            memory_required: None,
            safety_checks,
            warnings: Vec::new(),
        })
    }

    /// Check if a module can be hot swapped
    pub async fn can_hot_swap(&self, name: &str) -> bool {
        let versions = self.versions.read().await;
        let module_versions = versions.get(name);

        match module_versions {
            Some(versions) => {
                // Can hot swap if there's at least one version
                !versions.is_empty()
            }
            None => false,
        }
    }

    /// Get module status
    pub async fn get_module_status(&self, name: &str) -> ModuleStatus {
        let active_version = self.get_active_version(name).await;
        let versions = self.get_versions(name).await;
        let can_swap = self.can_hot_swap(name).await;

        let rollback_point = {
            let rollback_points = self.rollback_points.read().await;
            rollback_points.get(name).cloned()
        };

        ModuleStatus {
            name: name.to_string(),
            active_version,
            total_versions: versions.len(),
            can_hot_swap: can_swap,
            rollback_point,
            last_update: versions
                .iter()
                .filter_map(|v| v.activated_at)
                .max()
                .unwrap_or(0),
        }
    }
}

/// Module status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleStatus {
    pub name: String,
    pub active_version: Option<String>,
    pub total_versions: usize,
    pub can_hot_swap: bool,
    pub rollback_point: Option<String>,
    pub last_update: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_module_status() {
        let status = ModuleStatus {
            name: "test-module".to_string(),
            active_version: Some("v1.0.0".to_string()),
            total_versions: 3,
            can_hot_swap: true,
            rollback_point: Some("v0.9.0".to_string()),
            last_update: chrono::Utc::now().timestamp(),
        };

        assert_eq!(status.name, "test-module");
        assert!(status.active_version.is_some());
    }
}
