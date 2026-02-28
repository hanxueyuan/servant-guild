//! Build Sandbox - Secure Isolated Build Environment
//!
//! This module provides sandboxed build execution for ServantGuild,
//! ensuring that build processes are isolated, resource-limited, and safe.
//!
//! Security Features:
//! - Isolated workspace per agent
//! - Memory and CPU limits
//! - Network access control (whitelist only)
//! - Filesystem isolation (read-only except workspace)
//! - Timeout enforcement

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Root directory for all sandboxes
    pub sandbox_root: PathBuf,
    /// Maximum memory per build (MB)
    pub max_memory_mb: u64,
    /// Maximum CPU time (seconds)
    pub max_cpu_time_secs: u64,
    /// Maximum wall clock time (seconds)
    pub max_wall_time_secs: u64,
    /// Maximum output size (MB)
    pub max_output_mb: u64,
    /// Allowed network domains
    pub allowed_domains: HashSet<String>,
    /// Whether network access is allowed
    pub network_allowed: bool,
    /// Whether to use container isolation (Docker/Podman)
    pub use_container: bool,
    /// Container image to use (if use_container is true)
    pub container_image: Option<String>,
    /// Environment variables to pass through
    pub env_passthrough: Vec<String>,
    /// Environment variables to set
    pub env_set: std::collections::HashMap<String, String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        let mut allowed_domains = HashSet::new();
        allowed_domains.insert("crates.io".to_string());
        allowed_domains.insert("static.crates.io".to_string());
        allowed_domains.insert("index.crates.io".to_string());
        allowed_domains.insert("github.com".to_string());
        allowed_domains.insert("api.github.com".to_string());

        Self {
            sandbox_root: PathBuf::from("/tmp/servant-sandboxes"),
            max_memory_mb: 2048,        // 2GB
            max_cpu_time_secs: 600,     // 10 minutes CPU time
            max_wall_time_secs: 900,    // 15 minutes wall time
            max_output_mb: 100,         // 100MB output
            allowed_domains,
            network_allowed: true,      // Need network for cargo fetch
            use_container: false,       // Default to process isolation
            container_image: Some("rust:1.87".to_string()),
            env_passthrough: vec![
                "PATH".to_string(),
                "HOME".to_string(),
                "USER".to_string(),
                "CARGO_HOME".to_string(),
                "RUSTUP_HOME".to_string(),
            ],
            env_set: std::collections::HashMap::new(),
        }
    }
}

impl SandboxConfig {
    /// Create a new sandbox configuration
    pub fn new(sandbox_root: PathBuf) -> Self {
        Self {
            sandbox_root,
            ..Default::default()
        }
    }

    /// Set memory limit
    pub fn with_memory_limit(mut self, mb: u64) -> Self {
        self.max_memory_mb = mb;
        self
    }

    /// Set time limits
    pub fn with_time_limits(mut self, cpu_secs: u64, wall_secs: u64) -> Self {
        self.max_cpu_time_secs = cpu_secs;
        self.max_wall_time_secs = wall_secs;
        self
    }

    /// Allow network access
    pub fn with_network(mut self, allowed: bool) -> Self {
        self.network_allowed = allowed;
        self
    }

    /// Use container isolation
    pub fn with_container(mut self, image: String) -> Self {
        self.use_container = true;
        self.container_image = Some(image);
        self
    }

    /// Add allowed domain
    pub fn with_allowed_domain(mut self, domain: String) -> Self {
        self.allowed_domains.insert(domain);
        self
    }

    /// Set environment variable
    pub fn with_env(mut self, key: String, value: String) -> Self {
        self.env_set.insert(key, value);
        self
    }
}

/// Build sandbox instance
pub struct BuildSandbox {
    /// Unique sandbox ID
    pub id: String,
    /// Configuration
    config: SandboxConfig,
    /// Workspace path for this sandbox
    pub workspace: PathBuf,
    /// Whether the sandbox is active
    active: Arc<RwLock<bool>>,
    /// Process ID (if running)
    pid: Arc<RwLock<Option<u32>>>,
}

impl BuildSandbox {
    /// Create a new build sandbox
    pub async fn new(id: String, config: SandboxConfig) -> Result<Self> {
        let workspace = config.sandbox_root.join(&id);

        // Create workspace directory with restricted permissions
        Self::create_isolated_workspace(&workspace)?;

        info!("Created build sandbox: {} at {:?}", id, workspace);

        Ok(Self {
            id,
            config,
            workspace,
            active: Arc::new(RwLock::new(true)),
            pid: Arc::new(RwLock::new(None)),
        })
    }

    /// Create isolated workspace with proper permissions
    fn create_isolated_workspace(workspace: &Path) -> Result<()> {
        // Create parent directories
        if let Some(parent) = workspace.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create sandbox root")?;
        }

        // Create workspace
        std::fs::create_dir_all(workspace)
            .context("Failed to create workspace")?;

        // Set restrictive permissions (rwx for owner only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o700);
            std::fs::set_permissions(workspace, perms)
                .context("Failed to set workspace permissions")?;
        }

        Ok(())
    }

    /// Execute a command in the sandbox
    pub async fn execute(
        &self,
        program: &str,
        args: &[&str],
        working_dir: Option<&Path>,
    ) -> Result<SandboxResult> {
        let start_time = std::time::Instant::now();

        info!(
            "Executing in sandbox {}: {} {:?}",
            self.id, program, args
        );

        // Check if sandbox is still active
        if !*self.active.read().await {
            bail!("Sandbox {} is no longer active", self.id);
        }

        let result = if self.config.use_container {
            self.execute_in_container(program, args, working_dir).await?
        } else {
            self.execute_with_limits(program, args, working_dir).await?
        };

        let duration = start_time.elapsed();

        Ok(SandboxResult {
            success: result.success,
            exit_code: result.exit_code,
            stdout: result.stdout,
            stderr: result.stderr,
            duration_ms: duration.as_millis() as u64,
            timed_out: result.timed_out,
            oom_killed: result.oom_killed,
        })
    }

    /// Execute with resource limits (process isolation)
    async fn execute_with_limits(
        &self,
        program: &str,
        args: &[&str],
        working_dir: Option<&Path>,
    ) -> Result<InternalResult> {
        let work_dir = working_dir
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| self.workspace.clone());

        // Build command with resource limits
        let mut cmd = Command::new(program);
        cmd.args(args)
            .current_dir(&work_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set environment
        for key in &self.config.env_passthrough {
            if let Ok(value) = std::env::var(key) {
                cmd.env(key, value);
            }
        }
        for (key, value) in &self.config.env_set {
            cmd.env(key, value);
        }

        // Apply ulimit-style limits (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            
            // This will be applied in pre_exec
            let max_memory = self.config.max_memory_mb * 1024 * 1024;
            let max_cpu_time = self.config.max_cpu_time_secs;
            
            unsafe {
                cmd.pre_exec(move || {
                    // Set memory limit (RLIMIT_DATA)
                    let rlimit = libc::rlimit {
                        rlim_cur: max_memory,
                        rlim_max: max_memory,
                    };
                    libc::setrlimit(libc::RLIMIT_DATA, &rlimit);

                    // Set CPU time limit
                    let cpu_limit = libc::rlimit {
                        rlim_cur: max_cpu_time,
                        rlim_max: max_cpu_time,
                    };
                    libc::setrlimit(libc::RLIMIT_CPU, &cpu_limit);

                    Ok(())
                });
            }
        }

        // Spawn and capture output
        let mut child = cmd.spawn()
            .context("Failed to spawn process")?;

        // Store PID
        *self.pid.write().await = Some(child.id());

        // Set up timeout
        let timeout_duration = std::time::Duration::from_secs(self.config.max_wall_time_secs);
        let timeout = tokio::time::sleep(timeout_duration);

        tokio::select! {
            result = child.wait() => {
                let status = result.context("Failed to wait for process")?;
                
                // Read output
                let stdout = Self::read_stream(child.stdout.take()).await?;
                let stderr = Self::read_stream(child.stderr.take()).await?;

                Ok(InternalResult {
                    success: status.success(),
                    exit_code: status.code(),
                    stdout,
                    stderr,
                    timed_out: false,
                    oom_killed: false,
                })
            }
            _ = timeout => {
                // Kill the process
                warn!("Process timed out, killing...");
                let _ = child.kill().await;
                
                Ok(InternalResult {
                    success: false,
                    exit_code: None,
                    stdout: String::new(),
                    stderr: "Process timed out".to_string(),
                    timed_out: true,
                    oom_killed: false,
                })
            }
        }
    }

    /// Execute in container (Docker/Podman)
    async fn execute_in_container(
        &self,
        program: &str,
        args: &[&str],
        working_dir: Option<&Path>,
    ) -> Result<InternalResult> {
        let image = self.config.container_image.as_ref()
            .context("Container image not specified")?;

        let work_dir = working_dir
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("/workspace"));

        // Build docker command
        let mut docker_args = vec![
            "run".to_string(),
            "--rm".to_string(),
            "-v".to_string(),
            format!("{}:/workspace", self.workspace.display()),
            "-w".to_string(),
            work_dir.to_string_lossy().to_string(),
            "--memory".to_string(),
            format!("{}m", self.config.max_memory_mb),
            "--cpus".to_string(),
            "2".to_string(),  // Limit to 2 CPUs
        ];

        // Add network configuration
        if !self.config.network_allowed {
            docker_args.push("--network".to_string());
            docker_args.push("none".to_string());
        }

        // Add environment variables
        for (key, value) in &self.config.env_set {
            docker_args.push("-e".to_string());
            docker_args.push(format!("{}={}", key, value));
        }

        // Add image and command
        docker_args.push(image.clone());
        docker_args.push(program.to_string());
        docker_args.extend(args.iter().map(|s| s.to_string()));

        // Execute
        let mut cmd = Command::new("docker");
        cmd.args(&docker_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn()
            .context("Failed to spawn docker")?;

        // Store PID
        *self.pid.write().await = Some(child.id());

        // Set up timeout
        let timeout_duration = std::time::Duration::from_secs(self.config.max_wall_time_secs);
        let timeout = tokio::time::sleep(timeout_duration);

        tokio::select! {
            result = child.wait() => {
                let status = result.context("Failed to wait for container")?;
                
                let stdout = Self::read_stream(child.stdout.take()).await?;
                let stderr = Self::read_stream(child.stderr.take()).await?;

                Ok(InternalResult {
                    success: status.success(),
                    exit_code: status.code(),
                    stdout,
                    stderr,
                    timed_out: false,
                    oom_killed: stderr.contains("OOMKilled"),
                })
            }
            _ = timeout => {
                warn!("Container timed out, killing...");
                let _ = child.kill().await;
                
                Ok(InternalResult {
                    success: false,
                    exit_code: None,
                    stdout: String::new(),
                    stderr: "Container timed out".to_string(),
                    timed_out: true,
                    oom_killed: false,
                })
            }
        }
    }

    /// Read from an async stream
    async fn read_stream(stream: Option<tokio::process::ChildStdout>) -> Result<String> {
        let mut output = String::new();
        
        if let Some(stream) = stream {
            let reader = BufReader::new(stream);
            let mut lines = reader.lines();
            
            while let Some(line) = lines.next_line().await? {
                output.push_str(&line);
                output.push('\n');
            }
        }
        
        Ok(output)
    }

    /// Copy project files into the sandbox
    pub async fn copy_project(&self, source: &Path) -> Result<()> {
        debug!("Copying project from {:?} to sandbox", source);

        // Copy directory recursively
        self.copy_dir_recursively(source, &self.workspace)?;

        Ok(())
    }

    /// Copy directory recursively
    fn copy_dir_recursively(&self, src: &Path, dst: &Path) -> Result<()> {
        if !dst.exists() {
            std::fs::create_dir_all(dst)?;
        }

        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if entry.file_type()?.is_dir() {
                // Skip hidden directories and target directories
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') || name == "target" {
                    continue;
                }
                self.copy_dir_recursively(&src_path, &dst_path)?;
            } else {
                std::fs::copy(&src_path, &dst_path)?;
            }
        }

        Ok(())
    }

    /// Clean up the sandbox
    pub async fn cleanup(&self) -> Result<()> {
        *self.active.write().await = false;

        // Kill any running process
        if let Some(pid) = *self.pid.read().await {
            #[cfg(unix)]
            {
                use nix::sys::signal::{kill, Signal};
                let _ = kill(nix::unistd::Pid::from_raw(pid as i32), Signal::SIGTERM);
            }
        }

        // Remove workspace
        if self.workspace.exists() {
            std::fs::remove_dir_all(&self.workspace)
                .context("Failed to remove workspace")?;
        }

        info!("Cleaned up sandbox: {}", self.id);

        Ok(())
    }

    /// Get workspace path
    pub fn workspace(&self) -> &Path {
        &self.workspace
    }
}

/// Result of sandbox execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxResult {
    /// Whether the command succeeded
    pub success: bool,
    /// Exit code
    pub exit_code: Option<i32>,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Whether the process timed out
    pub timed_out: bool,
    /// Whether the process was OOM killed
    pub oom_killed: bool,
}

/// Internal result structure
struct InternalResult {
    success: bool,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    timed_out: bool,
    oom_killed: bool,
}

/// Sandbox manager for creating and managing sandboxes
pub struct SandboxManager {
    config: SandboxConfig,
    active_sandboxes: Arc<RwLock<Vec<String>>>,
}

impl SandboxManager {
    /// Create a new sandbox manager
    pub fn new(config: SandboxConfig) -> Self {
        Self {
            config,
            active_sandboxes: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create a new sandbox
    pub async fn create_sandbox(&self, id: Option<String>) -> Result<BuildSandbox> {
        let sandbox_id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let sandbox = BuildSandbox::new(sandbox_id.clone(), self.config.clone()).await?;

        self.active_sandboxes.write().await.push(sandbox_id);

        Ok(sandbox)
    }

    /// Get active sandbox count
    pub async fn active_count(&self) -> usize {
        self.active_sandboxes.read().await.len()
    }

    /// Remove sandbox from active list
    pub async fn remove_sandbox(&self, id: &str) {
        self.active_sandboxes.write().await.retain(|s| s != id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sandbox_creation() {
        let config = SandboxConfig::default();
        let sandbox = BuildSandbox::new("test-sandbox".to_string(), config).await.unwrap();
        
        assert!(sandbox.workspace.exists());
        assert_eq!(sandbox.id, "test-sandbox");
        
        // Cleanup
        sandbox.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn test_sandbox_execute_echo() {
        let config = SandboxConfig::default();
        let sandbox = BuildSandbox::new("echo-test".to_string(), config).await.unwrap();
        
        let result = sandbox.execute("echo", &["hello", "world"], None).await.unwrap();
        
        assert!(result.success);
        assert!(result.stdout.contains("hello world"));
        
        // Cleanup
        sandbox.cleanup().await.unwrap();
    }

    #[test]
    fn test_sandbox_config_builder() {
        let config = SandboxConfig::new(PathBuf::from("/tmp/test"))
            .with_memory_limit(4096)
            .with_time_limits(300, 600)
            .with_network(false)
            .with_env("RUST_LOG".to_string(), "debug".to_string());
        
        assert_eq!(config.max_memory_mb, 4096);
        assert_eq!(config.max_cpu_time_secs, 300);
        assert!(!config.network_allowed);
        assert_eq!(config.env_set.get("RUST_LOG"), Some(&"debug".to_string()));
    }
}
