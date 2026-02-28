//! Build Tools - The Forge
//!
//! This module provides automated build capabilities for ServantGuild,
//! enabling the compilation of Rust code to WebAssembly (Wasm) modules.
//! The build tools support:
//! - Wasm compilation with Wasmtime Component Model
//! - Test execution
//! - Linting and code quality checks
//! - Release packaging
//!
//! Build Pipeline:
//! 1. Pull latest code from GitHub
//! 2. Run tests
//! 3. Compile to Wasm
//! 4. Generate release package
//! 5. Upload to GitHub Release

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;

use crate::runtime::bridges::github::GitHubBridge;

/// Build configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Target architecture (wasm32-wasi, wasm32-unknown-unknown)
    pub target: String,
    /// Release mode
    pub release: bool,
    /// Run tests before building
    pub run_tests: bool,
    /// Run clippy before building
    pub run_clippy: bool,
    /// Enable optimizations
    pub optimize: bool,
    /// Optimization level (0-3, s, z)
    pub opt_level: String,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            target: "wasm32-wasi".to_string(),
            release: true,
            run_tests: true,
            run_clippy: true,
            optimize: true,
            opt_level: "3".to_string(),
        }
    }
}

/// Build result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildResult {
    pub success: bool,
    pub wasm_path: Option<PathBuf>,
    pub wasm_size: Option<u64>,
    pub build_time: Duration,
    pub test_passed: bool,
    pub clippy_passed: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

/// Build tools for Wasm compilation
pub struct BuildTools {
    /// GitHub bridge for code access
    github: Arc<GitHubBridge>,
    /// Working directory
    work_dir: PathBuf,
    /// Output directory for Wasm files
    output_dir: PathBuf,
}

impl BuildTools {
    /// Create new Build Tools
    pub fn new(github: Arc<GitHubBridge>, work_dir: PathBuf) -> Self {
        let output_dir = work_dir.join("target/wasm32-wasi/release");

        Self {
            github,
            work_dir,
            output_dir,
        }
    }

    /// Run full build pipeline
    pub async fn build(&self, config: &BuildConfig) -> Result<BuildResult> {
        let start_time = std::time::Instant::now();

        // Ensure output directory exists
        tokio::fs::create_dir_all(&self.output_dir)
            .await
            .context("Failed to create output directory")?;

        let mut result = BuildResult {
            success: false,
            wasm_path: None,
            wasm_size: None,
            build_time: Duration::from_secs(0),
            test_passed: false,
            clippy_passed: false,
            warnings: Vec::new(),
            errors: Vec::new(),
        };

        // Step 1: Run tests if configured
        if config.run_tests {
            match self.run_tests(config.release).await {
                Ok(_) => {
                    result.test_passed = true;
                }
                Err(e) => {
                    result.errors.push(format!("Tests failed: {}", e));
                    return Ok(result);
                }
            }
        } else {
            result.test_passed = true;
        }

        // Step 2: Run clippy if configured
        if config.run_clippy {
            match self.run_clippy().await {
                Ok(warnings) => {
                    result.clippy_passed = true;
                    result.warnings.extend(warnings);
                }
                Err(e) => {
                    result.errors.push(format!("Clippy failed: {}", e));
                    result.clippy_passed = false;
                    return Ok(result);
                }
            }
        } else {
            result.clippy_passed = true;
        }

        // Step 3: Compile to Wasm
        match self.compile_wasm(config).await {
            Ok(wasm_path) => {
                result.wasm_path = Some(wasm_path.clone());

                // Get file size
                let metadata = tokio::fs::metadata(&wasm_path).await?;
                result.wasm_size = Some(metadata.len());
            }
            Err(e) => {
                result.errors.push(format!("Wasm compilation failed: {}", e));
                result.build_time = start_time.elapsed();
                return Ok(result);
            }
        }

        // Step 4: Optimize if configured
        if let Some(ref wasm_path) = result.wasm_path {
            if config.optimize {
                match self.optimize_wasm(wasm_path, &config.opt_level).await {
                    Ok(_) => {
                        // Update size after optimization
                        let metadata = tokio::fs::metadata(wasm_path).await?;
                        result.wasm_size = Some(metadata.len());
                    }
                    Err(e) => {
                        result.warnings.push(format!("Optimization failed: {}", e));
                    }
                }
            }
        }

        result.success = true;
        result.build_time = start_time.elapsed();

        Ok(result)
    }

    /// Run tests
    async fn run_tests(&self, release: bool) -> Result<()> {
        let mut cmd = Command::new("cargo");
        cmd.args(&["test"]);

        if release {
            cmd.arg("--release");
        }

        cmd.current_dir(&self.work_dir);

        let output = cmd.output().await.context("Failed to run tests")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Tests failed:\n{}", stderr);
        }

        Ok(())
    }

    /// Run clippy
    async fn run_clippy(&self) -> Result<Vec<String>> {
        let output = Command::new("cargo")
            .args(&["clippy", "--", "-W", "clippy::all"])
            .current_dir(&self.work_dir)
            .output()
            .await
            .context("Failed to run clippy")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Clippy failed:\n{}", stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let warnings: Vec<String> = stdout
            .lines()
            .filter(|line| line.contains("warning:"))
            .map(|line| line.to_string())
            .collect();

        Ok(warnings)
    }

    /// Compile to Wasm
    async fn compile_wasm(&self, config: &BuildConfig) -> Result<PathBuf> {
        let mut cmd = Command::new("cargo");
        cmd.arg("build")
            .arg("--target")
            .arg(&config.target);

        if config.release {
            cmd.arg("--release");
        }

        cmd.current_dir(&self.work_dir);

        let output = cmd.output().await.context("Failed to compile Wasm")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Wasm compilation failed:\n{}", stderr);
        }

        // Find the generated Wasm file
        let wasm_dir = self
            .work_dir
            .join("target")
            .join(&config.target);

        let target_dir = if config.release {
            wasm_dir.join("release")
        } else {
            wasm_dir.join("debug")
        };

        // Find .wasm files
        let mut wasm_files = tokio::fs::read_dir(&target_dir)
            .await?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .map(|ext| ext == "wasm")
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();

        if wasm_files.is_empty() {
            anyhow::bail!("No Wasm file found in target directory");
        }

        let wasm_path = wasm_files[0].path();

        Ok(wasm_path)
    }

    /// Optimize Wasm file
    async fn optimize_wasm(&self, wasm_path: &Path, opt_level: &str) -> Result<()> {
        // Use wasm-opt if available
        let output = Command::new("wasm-opt")
            .args(&[
                "-O",
                opt_level,
                "-o",
                wasm_path.to_str().unwrap(),
                wasm_path.to_str().unwrap(),
            ])
            .output()
            .await;

        match output {
            Ok(result) if result.status.success() => Ok(()),
            Ok(result) => {
                let stderr = String::from_utf8_lossy(&result.stderr);
                anyhow::bail!("wasm-opt failed: {}", stderr);
            }
            Err(e) => {
                // wasm-opt not available, skip optimization
                eprintln!("wasm-opt not available, skipping optimization: {}", e);
                Ok(())
            }
        }
    }

    /// Generate release package
    pub async fn generate_release(
        &self,
        build_result: &BuildResult,
        version: &str,
    ) -> Result<PathBuf> {
        let release_dir = self.output_dir.join(format!("release-{}", version));
        tokio::fs::create_dir_all(&release_dir).await?;

        // Copy Wasm file
        if let Some(ref wasm_path) = build_result.wasm_path {
            let dest = release_dir.join("servant.wasm");
            tokio::fs::copy(wasm_path, &dest).await?;
        }

        // Create manifest
        let manifest = ReleaseManifest {
            version: version.to_string(),
            build_time: chrono::Utc::now().to_rfc3339(),
            wasm_size: build_result.wasm_size.unwrap_or(0),
            test_passed: build_result.test_passed,
            clippy_passed: build_result.clippy_passed,
            warnings: build_result.warnings.clone(),
        };

        let manifest_path = release_dir.join("manifest.json");
        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        tokio::fs::write(&manifest_path, manifest_json).await?;

        // Create archive
        let archive_path = self.output_dir.join(format!("release-{}.tar.gz", version));
        let output = Command::new("tar")
            .args(&[
                "-czf",
                archive_path.to_str().unwrap(),
                "-C",
                self.output_dir.to_str().unwrap(),
                format!("release-{}", version).as_str(),
            ])
            .output()
            .await
            .context("Failed to create archive")?;

        if !output.status.success() {
            anyhow::bail!("Failed to create release archive");
        }

        Ok(archive_path)
    }
}

/// Release manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReleaseManifest {
    version: String,
    build_time: String,
    wasm_size: u64,
    test_passed: bool,
    clippy_passed: bool,
    warnings: Vec<String>,
}

/// Builder for configuring builds
pub struct BuildBuilder {
    config: BuildConfig,
}

impl BuildBuilder {
    /// Create new build builder
    pub fn new() -> Self {
        Self {
            config: BuildConfig::default(),
        }
    }

    /// Set target
    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.config.target = target.into();
        self
    }

    /// Set release mode
    pub fn release(mut self, release: bool) -> Self {
        self.config.release = release;
        self
    }

    /// Enable/disable tests
    pub fn run_tests(mut self, run: bool) -> Self {
        self.config.run_tests = run;
        self
    }

    /// Enable/disable clippy
    pub fn run_clippy(mut self, run: bool) -> Self {
        self.config.run_clippy = run;
        self
    }

    /// Enable/disable optimizations
    pub fn optimize(mut self, optimize: bool) -> Self {
        self.config.optimize = optimize;
        self
    }

    /// Set optimization level
    pub fn opt_level(mut self, level: impl Into<String>) -> Self {
        self.config.opt_level = level.into();
        self
    }

    /// Build
    pub fn build(self) -> BuildConfig {
        self.config
    }
}

impl Default for BuildBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_config_default() {
        let config = BuildConfig::default();
        assert_eq!(config.target, "wasm32-wasi");
        assert!(config.release);
        assert!(config.run_tests);
        assert!(config.run_clippy);
    }

    #[test]
    fn test_build_builder() {
        let config = BuildBuilder::new()
            .target("wasm32-unknown-unknown")
            .release(false)
            .run_tests(false)
            .run_clippy(false)
            .build();

        assert_eq!(config.target, "wasm32-unknown-unknown");
        assert!(!config.release);
        assert!(!config.run_tests);
        assert!(!config.run_clippy);
    }
}
