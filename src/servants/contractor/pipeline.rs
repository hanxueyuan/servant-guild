//! Contractor Build Pipeline - Automated Build and Deploy
//!
//! This module implements the build pipeline for the Contractor servant,
//! providing automated building, testing, and deployment capabilities.
//!
//! Pipeline Stages:
//! 1. Prepare - Validate inputs, create workspace
//! 2. Fetch - Download dependencies
//! 3. Build - Compile the project
//! 4. Test - Run test suite
//! 5. Package - Create deployment artifact
//! 6. Deploy - Deploy to target environment

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::runtime::{BuildSandbox, SandboxConfig, SandboxManager, ErrorAnalyzer, AutoFixer, BuildContext};
use crate::consensus::{Proposal, DecisionType};

/// Build pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Project root path
    pub project_root: PathBuf,
    /// Output directory for artifacts
    pub output_dir: PathBuf,
    /// Build target (debug/release)
    pub build_target: BuildTarget,
    /// Enable auto-fix on build errors
    pub auto_fix: bool,
    /// Maximum auto-fix attempts
    pub max_auto_fix_attempts: usize,
    /// Enable incremental builds
    pub incremental: bool,
    /// Custom build commands
    pub custom_commands: HashMap<String, String>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Timeout in seconds
    pub timeout_secs: u64,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            project_root: PathBuf::from("."),
            output_dir: PathBuf::from("./target"),
            build_target: BuildTarget::Release,
            auto_fix: true,
            max_auto_fix_attempts: 3,
            incremental: true,
            custom_commands: HashMap::new(),
            env: HashMap::new(),
            timeout_secs: 600,
        }
    }
}

/// Build target
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BuildTarget {
    Debug,
    Release,
}

/// Pipeline stage
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PipelineStage {
    Prepare,
    Fetch,
    Build,
    Test,
    Package,
    Deploy,
}

impl PipelineStage {
    /// Get stage name
    pub fn name(&self) -> &str {
        match self {
            Self::Prepare => "prepare",
            Self::Fetch => "fetch",
            Self::Build => "build",
            Self::Test => "test",
            Self::Package => "package",
            Self::Deploy => "deploy",
        }
    }

    /// Get stage order
    pub fn order(&self) -> u8 {
        match self {
            Self::Prepare => 0,
            Self::Fetch => 1,
            Self::Build => 2,
            Self::Test => 3,
            Self::Package => 4,
            Self::Deploy => 5,
        }
    }

    /// Get all stages in order
    pub fn all() -> Vec<Self> {
        vec![
            Self::Prepare,
            Self::Fetch,
            Self::Build,
            Self::Test,
            Self::Package,
            Self::Deploy,
        ]
    }
}

/// Stage result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    /// Stage name
    pub stage: PipelineStage,
    /// Whether the stage succeeded
    pub success: bool,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Output from the stage
    pub output: String,
    /// Errors from the stage
    pub errors: Vec<String>,
    /// Warnings from the stage
    pub warnings: Vec<String>,
    /// Artifacts produced
    pub artifacts: Vec<String>,
}

/// Pipeline result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    /// Pipeline ID
    pub id: String,
    /// Overall success
    pub success: bool,
    /// Stage results
    pub stages: HashMap<PipelineStage, StageResult>,
    /// Total duration in milliseconds
    pub total_duration_ms: u64,
    /// Final artifact path
    pub artifact: Option<PathBuf>,
    /// Error message if failed
    pub error: Option<String>,
    /// Build context used
    pub context: BuildContext,
}

/// Build pipeline
pub struct BuildPipeline {
    /// Pipeline configuration
    config: PipelineConfig,
    /// Sandbox manager
    sandbox_manager: Arc<SandboxManager>,
    /// Error analyzer
    error_analyzer: Arc<ErrorAnalyzer>,
    /// Auto-fixer
    auto_fixer: Arc<AutoFixer>,
    /// Current pipeline runs
    runs: Arc<RwLock<HashMap<String, PipelineResult>>>,
}

impl BuildPipeline {
    /// Create a new build pipeline
    pub fn new(config: PipelineConfig) -> Result<Self> {
        let sandbox_config = SandboxConfig::new(config.project_root.join(".sandboxes"));
        let sandbox_manager = Arc::new(SandboxManager::new(sandbox_config));
        
        let error_analyzer = Arc::new(ErrorAnalyzer::new());
        let auto_fixer = Arc::new(AutoFixer::new(error_analyzer.clone(), 70, 5));
        
        Ok(Self {
            config,
            sandbox_manager,
            error_analyzer,
            auto_fixer,
            runs: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Create with LLM support
    pub fn with_llm(
        config: PipelineConfig,
        llm: Arc<dyn crate::providers::LLMProvider>,
    ) -> Result<Self> {
        let sandbox_config = SandboxConfig::new(config.project_root.join(".sandboxes"));
        let sandbox_manager = Arc::new(SandboxManager::new(sandbox_config));
        
        let error_analyzer = Arc::new(ErrorAnalyzer::with_llm(Some(llm)));
        let auto_fixer = Arc::new(AutoFixer::new(error_analyzer.clone(), 70, 5));
        
        Ok(Self {
            config,
            sandbox_manager,
            error_analyzer,
            auto_fixer,
            runs: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Run the full pipeline
    pub async fn run(&self, proposal: Option<&Proposal>) -> Result<PipelineResult> {
        let pipeline_id = format!("pipeline-{}", uuid::Uuid::new_v4());
        let start_time = std::time::Instant::now();
        
        info!("Starting build pipeline: {}", pipeline_id);
        
        let mut stages = HashMap::new();
        let mut success = true;
        let mut error = None;
        
        // Run stages in order
        for stage in PipelineStage::all() {
            let stage_result = self.run_stage(&stage, proposal).await?;
            
            success = success && stage_result.success;
            stages.insert(stage, stage_result);
            
            if !success {
                error = stages.get(&stage)
                    .and_then(|r| r.errors.first().cloned());
                break;
            }
        }
        
        let total_duration = start_time.elapsed();
        
        // Determine final artifact
        let artifact = if success {
            self.find_artifact(&stages).await?
        } else {
            None
        };
        
        // Build context
        let context = self.build_context(&stages).await;
        
        let result = PipelineResult {
            id: pipeline_id.clone(),
            success,
            stages,
            total_duration_ms: total_duration.as_millis() as u64,
            artifact,
            error,
            context,
        };
        
        // Store result
        self.runs.write().await.insert(pipeline_id, result.clone());
        
        info!(
            "Pipeline {} finished: success={}, duration={}ms",
            result.id,
            result.success,
            result.total_duration_ms
        );
        
        Ok(result)
    }

    /// Run a specific stage
    async fn run_stage(
        &self,
        stage: &PipelineStage,
        proposal: Option<&Proposal>,
    ) -> Result<StageResult> {
        let start_time = std::time::Instant::now();
        
        info!("Running stage: {:?}", stage);
        
        let result = match stage {
            PipelineStage::Prepare => self.run_prepare_stage(proposal).await,
            PipelineStage::Fetch => self.run_fetch_stage().await,
            PipelineStage::Build => self.run_build_stage().await,
            PipelineStage::Test => self.run_test_stage().await,
            PipelineStage::Package => self.run_package_stage().await,
            PipelineStage::Deploy => self.run_deploy_stage(proposal).await,
        };
        
        let duration = start_time.elapsed();
        
        match result {
            Ok(stage_result) => Ok(StageResult {
                stage: *stage,
                duration_ms: duration.as_millis() as u64,
                ..stage_result
            }),
            Err(e) => Ok(StageResult {
                stage: *stage,
                success: false,
                duration_ms: duration.as_millis() as u64,
                output: String::new(),
                errors: vec![e.to_string()],
                warnings: Vec::new(),
                artifacts: Vec::new(),
            }),
        }
    }

    /// Prepare stage - validate inputs and create workspace
    async fn run_prepare_stage(&self, proposal: Option<&Proposal>) -> Result<StageResult> {
        let mut output = String::new();
        let mut warnings = Vec::new();
        
        // Check project exists
        let cargo_toml = self.config.project_root.join("Cargo.toml");
        if !cargo_toml.exists() {
            bail!("Cargo.toml not found in project root");
        }
        
        output.push_str("✓ Found Cargo.toml\n");
        
        // Check for lock file
        let cargo_lock = self.config.project_root.join("Cargo.lock");
        if !cargo_lock.exists() {
            warnings.push("Cargo.lock not found - will be generated".to_string());
        }
        
        // Create output directory
        fs::create_dir_all(&self.config.output_dir).await?;
        output.push_str(&format!("✓ Output directory: {:?}\n", self.config.output_dir));
        
        // Parse proposal if present
        if let Some(prop) = proposal {
            output.push_str(&format!("✓ Building for proposal: {}\n", prop.title));
        }
        
        Ok(StageResult {
            stage: PipelineStage::Prepare,
            success: true,
            duration_ms: 0,
            output,
            errors: Vec::new(),
            warnings,
            artifacts: Vec::new(),
        })
    }

    /// Fetch stage - download dependencies
    async fn run_fetch_stage(&self) -> Result<StageResult> {
        let mut output = String::new();
        
        // Create sandbox for fetch
        let sandbox = self.sandbox_manager.create_sandbox(None).await?;
        
        // Copy project to sandbox
        sandbox.copy_project(&self.config.project_root).await?;
        
        // Run cargo fetch
        let result = sandbox.execute(
            "cargo",
            &["fetch", "--locked"],
            None,
        ).await?;
        
        self.sandbox_manager.remove_sandbox(&sandbox.id).await;
        sandbox.cleanup().await?;
        
        output.push_str(&result.stdout);
        
        Ok(StageResult {
            stage: PipelineStage::Fetch,
            success: result.success,
            duration_ms: result.duration_ms,
            output,
            errors: if result.success { Vec::new() } else { vec![result.stderr] },
            warnings: Vec::new(),
            artifacts: Vec::new(),
        })
    }

    /// Build stage - compile the project
    async fn run_build_stage(&self) -> Result<StageResult> {
        let mut output = String::new();
        let mut errors = Vec::new();
        let mut artifacts = Vec::new();
        
        // Create sandbox for build
        let sandbox = self.sandbox_manager.create_sandbox(None).await?;
        
        // Copy project
        sandbox.copy_project(&self.config.project_root).await?;
        
        // Build command
        let build_args = match self.config.build_target {
            BuildTarget::Debug => vec!["build"],
            BuildTarget::Release => vec!["build", "--release"],
        };
        
        // Add features if specified
        if let Some(features) = self.config.env.get("CARGO_BUILD_FEATURES") {
            // Features specified via environment
        }
        
        let mut attempts = 0;
        let max_attempts = if self.config.auto_fix {
            self.config.max_auto_fix_attempts + 1
        } else {
            1
        };
        
        loop {
            attempts += 1;
            
            let result = sandbox.execute("cargo", &build_args, None).await?;
            
            if result.success {
                output.push_str(&result.stdout);
                
                // Find artifacts
                let target_dir = sandbox.workspace().join("target");
                let artifact_dir = match self.config.build_target {
                    BuildTarget::Debug => target_dir.join("debug"),
                    BuildTarget::Release => target_dir.join("release"),
                };
                
                // Look for binary
                if let Ok(mut entries) = fs::read_dir(&artifact_dir).await {
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let path = entry.path();
                        if path.is_file() {
                            let name = path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("");
                            
                            // Skip common non-binary files
                            if !name.starts_with('.') && 
                               !name.contains('.') && 
                               !name.starts_with("build") &&
                               !name.starts_with("deps") {
                                artifacts.push(path.display().to_string());
                            }
                        }
                    }
                }
                
                break;
            }
            
            // Build failed
            if attempts >= max_attempts || !self.config.auto_fix {
                errors.push(result.stderr.clone());
                output.push_str(&result.stdout);
                break;
            }
            
            // Try auto-fix
            output.push_str(&format!(
                "\n=== Build attempt {} failed, analyzing errors ===\n",
                attempts
            ));
            
            let context = self.build_context_from_sandbox(&sandbox).await;
            let fix_result = self.auto_fixer.auto_fix(
                &result.stderr,
                sandbox.workspace(),
                &context,
            ).await?;
            
            output.push_str(&format!(
                "Auto-fix applied {}/{} fixes\n",
                fix_result.fixes_applied,
                fix_result.fixes_generated
            ));
            
            if fix_result.fixes_applied == 0 {
                errors.push(result.stderr);
                break;
            }
        }
        
        self.sandbox_manager.remove_sandbox(&sandbox.id).await;
        sandbox.cleanup().await?;
        
        Ok(StageResult {
            stage: PipelineStage::Build,
            success: errors.is_empty(),
            duration_ms: 0,
            output,
            errors,
            warnings: Vec::new(),
            artifacts,
        })
    }

    /// Test stage - run test suite
    async fn run_test_stage(&self) -> Result<StageResult> {
        let mut output = String::new();
        let mut errors = Vec::new();
        
        // Create sandbox
        let sandbox = self.sandbox_manager.create_sandbox(None).await?;
        sandbox.copy_project(&self.config.project_root).await?;
        
        // Run tests
        let result = sandbox.execute(
            "cargo",
            &["test", "--no-fail-fast"],
            None,
        ).await?;
        
        output.push_str(&result.stdout);
        
        if !result.success {
            errors.push(result.stderr);
        }
        
        // Check for test summary
        let test_summary = self.parse_test_summary(&output);
        
        self.sandbox_manager.remove_sandbox(&sandbox.id).await;
        sandbox.cleanup().await?;
        
        Ok(StageResult {
            stage: PipelineStage::Test,
            success: result.success,
            duration_ms: result.duration_ms,
            output,
            errors,
            warnings: Vec::new(),
            artifacts: Vec::new(),
        })
    }

    /// Package stage - create deployment artifact
    async fn run_package_stage(&self) -> Result<StageResult> {
        let mut output = String::new();
        let mut artifacts = Vec::new();
        
        // Create package directory
        let package_dir = self.config.output_dir.join("package");
        fs::create_dir_all(&package_dir).await?;
        
        // Copy built artifacts
        let target_dir = match self.config.build_target {
            BuildTarget::Debug => self.config.project_root.join("target/debug"),
            BuildTarget::Release => self.config.project_root.join("target/release"),
        };
        
        // Find binaries
        if target_dir.exists() {
            let mut entries = fs::read_dir(&target_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.is_file() {
                    let name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    
                    // Copy binary
                    if !name.contains('.') && !name.starts_with("build") {
                        let dest = package_dir.join(name);
                        fs::copy(&path, &dest).await?;
                        artifacts.push(dest.display().to_string());
                        output.push_str(&format!("Packaged: {}\n", name));
                    }
                }
            }
        }
        
        // Create metadata file
        let metadata = PackageMetadata {
            version: "0.1.0".to_string(), // Would read from Cargo.toml
            build_target: self.config.build_target,
            timestamp: chrono::Utc::now(),
            artifacts: artifacts.clone(),
        };
        
        let metadata_path = package_dir.join("metadata.json");
        fs::write(
            &metadata_path,
            serde_json::to_string_pretty(&metadata)?,
        ).await?;
        
        output.push_str(&format!("Created: {:?}\n", metadata_path));
        
        Ok(StageResult {
            stage: PipelineStage::Package,
            success: true,
            duration_ms: 0,
            output,
            errors: Vec::new(),
            warnings: Vec::new(),
            artifacts,
        })
    }

    /// Deploy stage - deploy to target environment
    async fn run_deploy_stage(&self, proposal: Option<&Proposal>) -> Result<StageResult> {
        let mut output = String::new();
        let mut errors = Vec::new();
        let mut artifacts = Vec::new();
        
        // Deployment depends on the proposal type and configuration
        // This is a simplified version - real deployment would involve
        // more complex logic based on the target environment
        
        let package_dir = self.config.output_dir.join("package");
        
        if !package_dir.exists() {
            errors.push("Package directory not found - run package stage first".to_string());
        } else {
            output.push_str("Deployment prepared\n");
            
            // List artifacts to deploy
            let mut entries = fs::read_dir(&package_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                artifacts.push(entry.path().display().to_string());
            }
            
            if let Some(prop) = proposal {
                output.push_str(&format!("Deployed for: {}\n", prop.title));
            }
        }
        
        Ok(StageResult {
            stage: PipelineStage::Deploy,
            success: errors.is_empty(),
            duration_ms: 0,
            output,
            errors,
            warnings: Vec::new(),
            artifacts,
        })
    }

    /// Find the final artifact from successful build
    async fn find_artifact(&self, stages: &HashMap<PipelineStage, StageResult>) -> Result<Option<PathBuf>> {
        if let Some(package_stage) = stages.get(&PipelineStage::Package) {
            if let Some(artifact) = package_stage.artifacts.first() {
                return Ok(Some(PathBuf::from(artifact)));
            }
        }
        
        // Fall back to build artifacts
        if let Some(build_stage) = stages.get(&PipelineStage::Build) {
            if let Some(artifact) = build_stage.artifacts.first() {
                return Ok(Some(PathBuf::from(artifact)));
            }
        }
        
        Ok(None)
    }

    /// Build context from stages
    async fn build_context(&self, stages: &HashMap<PipelineStage, StageResult>) -> BuildContext {
        let mut context = BuildContext::default();
        
        // Extract available modules from build output
        if let Some(build_stage) = stages.get(&PipelineStage::Build) {
            // Parse module names from output
            for line in build_stage.output.lines() {
                if line.contains("Compiling ") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        context.available_modules.push(parts[1].to_string());
                    }
                }
            }
        }
        
        context
    }

    /// Build context from sandbox
    async fn build_context_from_sandbox(&self, sandbox: &BuildSandbox) -> BuildContext {
        let mut context = BuildContext::default();
        
        // Read Cargo.toml
        let cargo_toml_path = sandbox.workspace().join("Cargo.toml");
        if cargo_toml_path.exists() {
            if let Ok(content) = fs::read_to_string(&cargo_toml_path).await {
                // Parse dependencies (simplified)
                if let Ok(doc) = content.parse::<toml::Value>() {
                    if let Some(deps) = doc.get("dependencies").and_then(|d| d.as_table()) {
                        for (name, value) in deps {
                            if let Some(version) = value.as_str() {
                                context.dependencies.insert(name.clone(), version.to_string());
                            } else if let Some(table) = value.as_table() {
                                if let Some(v) = table.get("version").and_then(|v| v.as_str()) {
                                    context.dependencies.insert(name.clone(), v.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        
        context
    }

    /// Parse test summary from output
    fn parse_test_summary(&self, output: &str) -> TestSummary {
        let mut summary = TestSummary::default();
        
        for line in output.lines() {
            if line.contains("test result:") {
                // Parse test result line
                if let Some(caps) = regex::Regex::new(r"(\d+) passed")
                    .ok()
                    .and_then(|re| re.captures(line))
                {
                    summary.passed = caps.get(1)
                        .and_then(|m| m.as_str().parse().ok())
                        .unwrap_or(0);
                }
                
                if let Some(caps) = regex::Regex::new(r"(\d+) failed")
                    .ok()
                    .and_then(|re| re.captures(line))
                {
                    summary.failed = caps.get(1)
                        .and_then(|m| m.as_str().parse().ok())
                        .unwrap_or(0);
                }
            }
        }
        
        summary
    }

    /// Get pipeline result by ID
    pub async fn get_result(&self, id: &str) -> Option<PipelineResult> {
        self.runs.read().await.get(id).cloned()
    }

    /// List all pipeline runs
    pub async fn list_runs(&self) -> Vec<(String, bool, u64)> {
        self.runs.read().await
            .iter()
            .map(|(id, result)| {
                (id.clone(), result.success, result.total_duration_ms)
            })
            .collect()
    }
}

/// Package metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    /// Version
    pub version: String,
    /// Build target
    pub build_target: BuildTarget,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Included artifacts
    pub artifacts: Vec<String>,
}

/// Test summary
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TestSummary {
    pub passed: usize,
    pub failed: usize,
    pub ignored: usize,
}

/// Pipeline builder for configuration
pub struct PipelineBuilder {
    config: PipelineConfig,
}

impl PipelineBuilder {
    /// Create a new pipeline builder
    pub fn new() -> Self {
        Self {
            config: PipelineConfig::default(),
        }
    }

    /// Set project root
    pub fn project_root(mut self, path: PathBuf) -> Self {
        self.config.project_root = path;
        self
    }

    /// Set output directory
    pub fn output_dir(mut self, path: PathBuf) -> Self {
        self.config.output_dir = path;
        self
    }

    /// Set build target
    pub fn build_target(mut self, target: BuildTarget) -> Self {
        self.config.build_target = target;
        self
    }

    /// Enable or disable auto-fix
    pub fn auto_fix(mut self, enabled: bool) -> Self {
        self.config.auto_fix = enabled;
        self
    }

    /// Set maximum auto-fix attempts
    pub fn max_auto_fix_attempts(mut self, attempts: usize) -> Self {
        self.config.max_auto_fix_attempts = attempts;
        self
    }

    /// Add environment variable
    pub fn env(mut self, key: String, value: String) -> Self {
        self.config.env.insert(key, value);
        self
    }

    /// Set timeout
    pub fn timeout(mut self, secs: u64) -> Self {
        self.config.timeout_secs = secs;
        self
    }

    /// Build the pipeline
    pub fn build(self) -> Result<BuildPipeline> {
        BuildPipeline::new(self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_stage_order() {
        let stages = PipelineStage::all();
        
        assert_eq!(stages.len(), 6);
        assert_eq!(stages[0], PipelineStage::Prepare);
        assert_eq!(stages[5], PipelineStage::Deploy);
    }

    #[test]
    fn test_pipeline_builder() {
        let config = PipelineBuilder::new()
            .project_root(PathBuf::from("/my/project"))
            .build_target(BuildTarget::Release)
            .auto_fix(true)
            .timeout(300)
            .build()
            .unwrap();
        
        assert_eq!(config.project_root, PathBuf::from("/my/project"));
    }

    #[tokio::test]
    async fn test_pipeline_create() {
        let pipeline = BuildPipeline::new(PipelineConfig::default()).unwrap();
        
        assert!(pipeline.list_runs().await.is_empty());
    }
}
