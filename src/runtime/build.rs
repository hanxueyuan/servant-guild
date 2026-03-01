//! Build Automation - Automated Rust/Wasm Compilation
//!
//! This module provides build automation capabilities for ServantGuild,
//! enabling autonomous compilation of Rust/Wasm components, dependency
//! management, and artifact generation.

use crate::runtime::state::HostState;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::process::Command as AsyncCommand;
use tracing::{debug, info, warn};

/// Build target type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildTarget {
    /// Wasm component (for servants)
    WasmComponent,
    /// Native binary (for host)
    NativeBinary,
    /// Development build
    Dev,
    /// Production build
    Release,
}

/// Build configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Build target
    pub target: BuildTarget,
    /// Build profile (dev/release)
    pub profile: String,
    /// Features to enable
    pub features: Vec<String>,
    /// Custom build arguments
    pub args: Vec<String>,
    /// Output directory
    pub output_dir: Option<PathBuf>,
    /// Whether to run tests after build
    pub run_tests: bool,
    /// Whether to run clippy
    pub run_clippy: bool,
    /// Whether to check formatting
    pub check_fmt: bool,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            target: BuildTarget::WasmComponent,
            profile: "release".to_string(),
            features: Vec::new(),
            args: Vec::new(),
            output_dir: None,
            run_tests: true,
            run_clippy: true,
            check_fmt: false,
        }
    }
}

impl BuildConfig {
    /// Create new build config
    pub fn new(target: BuildTarget) -> Self {
        Self {
            target,
            ..Default::default()
        }
    }

    /// Add feature
    pub fn with_feature(mut self, feature: String) -> Self {
        self.features.push(feature);
        self
    }

    /// Set output directory
    pub fn with_output_dir(mut self, dir: PathBuf) -> Self {
        self.output_dir = Some(dir);
        self
    }

    /// Disable tests
    pub fn without_tests(mut self) -> Self {
        self.run_tests = false;
        self
    }

    /// Disable clippy
    pub fn without_clippy(mut self) -> Self {
        self.run_clippy = false;
        self
    }
}

/// Build result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildResult {
    /// Whether build succeeded
    pub success: bool,
    /// Build duration in milliseconds
    pub duration_ms: u64,
    /// Output artifacts
    pub artifacts: Vec<BuildArtifact>,
    /// Warnings encountered
    pub warnings: Vec<String>,
    /// Errors encountered
    pub errors: Vec<String>,
    /// Build logs
    pub logs: Vec<String>,
    /// Start time
    pub started_at: DateTime<Utc>,
    /// End time
    pub ended_at: DateTime<Utc>,
}

/// Build artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildArtifact {
    /// Artifact type
    pub artifact_type: String,
    /// File path
    pub path: PathBuf,
    /// Size in bytes
    pub size: u64,
    /// Checksum (SHA256)
    pub checksum: String,
}

/// Dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    /// Dependency name
    pub name: String,
    /// Version requirement
    pub version: String,
    /// Whether it's a dev dependency
    pub is_dev: bool,
    /// Whether it's a build dependency
    pub is_build: bool,
    /// Current installed version
    pub installed_version: Option<String>,
    /// Latest version available
    pub latest_version: Option<String>,
    /// Whether update is available
    pub has_update: bool,
}

/// Build automation trait
#[async_trait]
pub trait BuildAutomation: Send + Sync {
    /// Check if build tools are available
    async fn check_tools(&self) -> Result<bool>;

    /// Build a component
    async fn build(&self, config: BuildConfig, project_path: PathBuf) -> Result<BuildResult>;

    /// Run tests
    async fn test(&self, project_path: PathBuf, args: Vec<String>) -> Result<BuildResult>;

    /// Check code formatting
    async fn check_fmt(&self, project_path: PathBuf) -> Result<bool>;

    /// Run clippy
    async fn clippy(&self, project_path: PathBuf, args: Vec<String>) -> Result<BuildResult>;

    /// Update dependencies
    async fn update_deps(&self, project_path: PathBuf) -> Result<Vec<DependencyInfo>>;

    /// List dependencies
    async fn list_deps(&self, project_path: PathBuf) -> Result<Vec<DependencyInfo>>;

    /// Clean build artifacts
    async fn clean(&self, project_path: PathBuf) -> Result<()>;

    /// Generate documentation
    async fn docs(&self, project_path: PathBuf, open: bool) -> Result<()>;
}

/// Implementation of build automation
pub struct BuildAutomationImpl {
    /// Host state
    state: HostState,
    /// Whether to use async commands
    use_async: bool,
}

impl BuildAutomationImpl {
    /// Create new build automation instance
    pub fn new(state: HostState) -> Self {
        Self {
            state,
            use_async: true,
        }
    }

    /// Set whether to use async commands
    pub fn with_async(mut self, use_async: bool) -> Self {
        self.use_async = use_async;
        self
    }

    /// Execute cargo command
    async fn execute_cargo(&self, project_path: &Path, args: &[&str]) -> Result<BuildResult> {
        let start_time = Utc::now();
        let mut logs = Vec::new();
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        info!("Executing cargo command: cargo {}", args.join(" "));

        let output = if self.use_async {
            let mut cmd = AsyncCommand::new("cargo");
            cmd.current_dir(project_path);
            cmd.args(args);

            let output = cmd
                .output()
                .await
                .context("Failed to execute cargo command")?;

            output
        } else {
            // Fallback to sync command
            let output = Command::new("cargo")
                .current_dir(project_path)
                .args(args)
                .output()
                .context("Failed to execute cargo command")?;

            tokio::task::spawn_blocking(move || output).await?
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        logs.push(stdout.clone());
        logs.push(stderr.clone());

        // Parse warnings and errors
        for line in stdout.lines().chain(stderr.lines()) {
            if line.contains("warning:") {
                warnings.push(line.to_string());
            } else if line.contains("error:") {
                errors.push(line.to_string());
            }
        }

        let end_time = Utc::now();
        let duration_ms = (end_time - start_time).num_milliseconds() as u64;

        let success = output.status.success();

        Ok(BuildResult {
            success,
            duration_ms,
            artifacts: Vec::new(),
            warnings,
            errors,
            logs,
            started_at: start_time,
            ended_at: end_time,
        })
    }

    /// Find build artifacts
    fn find_artifacts(&self, project_path: &Path, target: BuildTarget) -> Vec<BuildArtifact> {
        let mut artifacts = Vec::new();
        let target_dir = project_path.join("target");

        let search_pattern = match target {
            BuildTarget::WasmComponent => "*.wasm",
            BuildTarget::NativeBinary => {
                if cfg!(windows) {
                    "*.exe"
                } else {
                    "servant-guild"
                }
            }
            BuildTarget::Dev | BuildTarget::Release => {
                if cfg!(windows) {
                    "*.exe"
                } else {
                    "servant-guild"
                }
            }
        };

        if let Ok(entries) = std::fs::read_dir(&target_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        let path = entry.path();
                        if let Some(name) = path.file_name() {
                            if name
                                .to_string_lossy()
                                .ends_with(search_pattern.trim_start_matches('*'))
                            {
                                if let Ok(metadata) = entry.metadata() {
                                    let checksum = if let Ok(contents) = std::fs::read(&path) {
                                        format!("{:x}", sha2::Sha256::digest(&contents))
                                    } else {
                                        "unknown".to_string()
                                    };

                                    artifacts.push(BuildArtifact {
                                        artifact_type: match target {
                                            BuildTarget::WasmComponent => "wasm",
                                            _ => "binary",
                                        }
                                        .to_string(),
                                        path,
                                        size: metadata.len(),
                                        checksum,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        artifacts
    }

    /// Get dependency info from Cargo.toml
    async fn parse_dependencies(&self, project_path: &Path) -> Result<Vec<DependencyInfo>> {
        let cargo_toml = project_path.join("Cargo.toml");
        let content = std::fs::read_to_string(&cargo_toml).context("Failed to read Cargo.toml")?;

        let manifest: cargo_toml::Manifest =
            cargo_toml::Manifest::from_str(&content).context("Failed to parse Cargo.toml")?;

        let mut deps = Vec::new();

        // Process dependencies
        if let Some(package) = manifest.package {
            for (name, dep) in manifest.dependencies {
                let version = dep.req().to_string();
                deps.push(DependencyInfo {
                    name: name.to_string(),
                    version: version.clone(),
                    is_dev: false,
                    is_build: false,
                    installed_version: Some(version),
                    latest_version: None,
                    has_update: false,
                });
            }

            for (name, dep) in manifest.dev_dependencies {
                let version = dep.req().to_string();
                deps.push(DependencyInfo {
                    name: name.to_string(),
                    version: version.clone(),
                    is_dev: true,
                    is_build: false,
                    installed_version: Some(version),
                    latest_version: None,
                    has_update: false,
                });
            }

            for (name, dep) in manifest.build_dependencies {
                let version = dep.req().to_string();
                deps.push(DependencyInfo {
                    name: name.to_string(),
                    version: version.clone(),
                    is_dev: false,
                    is_build: true,
                    installed_version: Some(version),
                    latest_version: None,
                    has_update: false,
                });
            }
        }

        Ok(deps)
    }
}

#[async_trait]
impl BuildAutomation for BuildAutomationImpl {
    async fn check_tools(&self) -> Result<bool> {
        let checks = vec![
            Command::new("cargo").arg("--version").output(),
            Command::new("rustc").arg("--version").output(),
            Command::new("wasm-opt").arg("--version").output(), // wasm-opt from binaryen
        ];

        let mut all_available = true;

        for check in checks {
            if let Ok(output) = check {
                if output.status.success() {
                    debug!(
                        "Tool available: {}",
                        String::from_utf8_lossy(&output.stdout)
                    );
                } else {
                    all_available = false;
                    warn!("Tool check failed");
                }
            } else {
                all_available = false;
                warn!("Tool not found");
            }
        }

        Ok(all_available)
    }

    async fn build(&self, config: BuildConfig, project_path: PathBuf) -> Result<BuildResult> {
        let mut args = vec!["build"];

        // Set profile
        if config.profile == "release" {
            args.push("--release");
        }

        // Add features
        for feature in &config.features {
            args.push("--features");
            args.push(feature);
        }

        // Add target for wasm
        if config.target == BuildTarget::WasmComponent {
            args.push("--target");
            args.push("wasm32-unknown-unknown");
        }

        // Add custom args
        for arg in &config.args {
            args.push(arg);
        }

        let mut result = self.execute_cargo(&project_path, &args).await?;

        // Find artifacts
        result.artifacts = self.find_artifacts(&project_path, config.target);

        // Run additional checks
        if config.run_clippy {
            let clippy_result = self.clippy(project_path.clone(), vec![]).await?;
            result.warnings.extend(clippy_result.warnings);
        }

        if config.run_tests {
            let test_result = self.test(project_path.clone(), vec![]).await?;
            if !test_result.success {
                result.success = false;
                result.errors.extend(test_result.errors);
            }
        }

        Ok(result)
    }

    async fn test(&self, project_path: PathBuf, args: Vec<String>) -> Result<BuildResult> {
        let mut cargo_args = vec!["test"];
        for arg in args.iter() {
            cargo_args.push(arg);
        }

        self.execute_cargo(&project_path, &cargo_args).await
    }

    async fn check_fmt(&self, project_path: PathBuf) -> Result<bool> {
        let output = self
            .execute_cargo(&project_path, &["fmt", "--all", "--", "--check"])
            .await?;
        Ok(output.success)
    }

    async fn clippy(&self, project_path: PathBuf, args: Vec<String>) -> Result<BuildResult> {
        let mut cargo_args = vec!["clippy", "--all-targets", "--all-features"];
        for arg in args.iter() {
            cargo_args.push(arg);
        }

        self.execute_cargo(&project_path, &cargo_args).await
    }

    async fn update_deps(&self, project_path: PathBuf) -> Result<Vec<DependencyInfo>> {
        let _result = self.execute_cargo(&project_path, &["update"]).await?;
        self.list_deps(project_path).await
    }

    async fn list_deps(&self, project_path: PathBuf) -> Result<Vec<DependencyInfo>> {
        self.parse_dependencies(&project_path).await
    }

    async fn clean(&self, project_path: PathBuf) -> Result<()> {
        let _result = self.execute_cargo(&project_path, &["clean"]).await?;
        Ok(())
    }

    async fn docs(&self, project_path: PathBuf, open: bool) -> Result<()> {
        let args = if open {
            vec!["doc", "--no-deps", "--open"]
        } else {
            vec!["doc", "--no-deps"]
        };

        let _result = self.execute_cargo(&project_path, &args).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_config_default() {
        let config = BuildConfig::default();
        assert_eq!(config.target, BuildTarget::WasmComponent);
        assert_eq!(config.profile, "release");
        assert!(config.run_tests);
        assert!(config.run_clippy);
    }

    #[test]
    fn test_build_config_chaining() {
        let config = BuildConfig::new(BuildTarget::NativeBinary)
            .with_feature("test-feature".to_string())
            .without_tests()
            .without_clippy();

        assert_eq!(config.target, BuildTarget::NativeBinary);
        assert!(config.features.contains(&"test-feature".to_string()));
        assert!(!config.run_tests);
        assert!(!config.run_clippy);
    }
}
