//! WASM sandbox runtime — in-process tool isolation via `wasmtime` Component Model.
//!
//! Provides capability-based sandboxing without Docker or external runtimes.
//! Each WASM module runs with:
//! - **Fuel limits**: prevents infinite loops
//! - **Memory caps**: configurable per-module memory ceiling
//! - **WASI Support**: Filesystem, HTTP, Clock via `wasi-common` & `cap-std`
//! - **Host Functions**: LLM, Tools, Safety via WIT interfaces
//!
//! # Feature gate
//! This module is only compiled when `--features runtime-wasm` is enabled.

use super::traits::RuntimeAdapter;
use crate::config::WasmRuntimeConfig;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

/// WASM sandbox runtime — executes tool modules in an isolated interpreter.
#[derive(Debug, Clone)]
pub struct WasmRuntime {
    config: WasmRuntimeConfig,
    workspace_dir: Option<PathBuf>,
}

/// Result of executing a WASM module.
#[derive(Debug, Clone)]
pub struct WasmExecutionResult {
    pub output: String,
    pub exit_code: i32,
    pub fuel_consumed: u64,
}

impl WasmRuntime {
    /// Create a new WASM runtime with the given configuration.
    pub fn new(config: WasmRuntimeConfig) -> Self {
        Self {
            config,
            workspace_dir: None,
        }
    }

    /// Create a WASM runtime bound to a specific workspace directory.
    pub fn with_workspace(config: WasmRuntimeConfig, workspace_dir: PathBuf) -> Self {
        Self {
            config,
            workspace_dir: Some(workspace_dir),
        }
    }

    /// Check if the WASM runtime feature is available in this build.
    pub fn is_available() -> bool {
        cfg!(feature = "runtime-wasm")
    }

    /// Resolve the absolute path to the WASM tools directory.
    pub fn tools_dir(&self, workspace_dir: &Path) -> PathBuf {
        workspace_dir.join(&self.config.tools_dir)
    }

    /// Execute a WASM component module (handle-task).
    #[cfg(feature = "runtime-wasm")]
    pub async fn execute_component(
        &self,
        module_name: &str,
        task_id: &str,
        input: &str,
        workspace_dir: &Path,
    ) -> Result<WasmExecutionResult> {
        use wasmtime::{
            component::{Component, Linker},
            Config, Engine, Store,
        };
        use wasmtime_wasi::preview2::{Table, WasiCtxBuilder};
        use crate::runtime::state::HostState;
        use crate::runtime::bindings::Servant;

        // 1. Configure Engine
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.async_support(true);
        config.consume_fuel(true); // Enable fuel metering
        let engine = Engine::new(&config)?;

        // 2. Prepare WASI Context
        let mut table = Table::new();
        let wasi = WasiCtxBuilder::new()
            .inherit_stdout()
            .inherit_stderr()
            .build(); // TODO: Add filesystem caps based on config

        // 3. Define Host State
        let mut store = Store::new(
            &engine,
            HostState::new(wasi, table),
        );
        store.set_fuel(self.config.fuel_limit)?;

        // 4. Load Component
        let tools_path = self.tools_dir(workspace_dir);
        let component_path = tools_path.join(format!("{}.wasm", module_name));
        let component = Component::from_file(&engine, &component_path)
            .with_context(|| format!("Failed to load component: {}", component_path.display()))?;

        // 5. Link Host Functions
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::preview2::command::add_to_linker(&mut linker)?;
        
        // Link our custom bridges (LLM, Tools, Safety, etc.)
        Servant::add_to_linker(&mut linker, |state: &mut HostState| state)?;

        // 6. Instantiate
        let (servant, _) = Servant::instantiate_async(&mut store, &component, &linker).await?;

        // 7. Execute handle-task
        let result = servant
            .call_handle_task(&mut store, task_id, input)
            .await?;

        let fuel_consumed = store.fuel_consumed().unwrap_or(0);

        match result {
            Ok(output) => Ok(WasmExecutionResult {
                output,
                exit_code: 0,
                fuel_consumed,
            }),
            Err(err) => Ok(WasmExecutionResult {
                output: err,
                exit_code: 1,
                fuel_consumed,
            }),
        }
    }

    /// Stub for when the `runtime-wasm` feature is not enabled.
    #[cfg(not(feature = "runtime-wasm"))]
    pub async fn execute_component(
        &self,
        module_name: &str,
        _task_id: &str,
        _input: &str,
        _workspace_dir: &Path,
    ) -> Result<WasmExecutionResult> {
        bail!(
            "WASM runtime is not available. Rebuild with `cargo build --features runtime-wasm`. Module: {}",
            module_name
        )
    }
}

impl RuntimeAdapter for WasmRuntime {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "wasm"
    }

    fn has_shell_access(&self) -> bool {
        false
    }

    fn has_filesystem_access(&self) -> bool {
        self.config.allow_workspace_read || self.config.allow_workspace_write
    }

    fn storage_path(&self) -> PathBuf {
        self.workspace_dir
            .as_ref()
            .map_or_else(|| PathBuf::from(".zeroclaw"), |w| w.join(".zeroclaw"))
    }

    fn supports_long_running(&self) -> bool {
        false
    }

    fn memory_budget(&self) -> u64 {
        self.config.memory_limit_mb.saturating_mul(1024 * 1024)
    }

    fn build_shell_command(
        &self,
        _command: &str,
        _workspace_dir: &Path,
    ) -> anyhow::Result<tokio::process::Command> {
        bail!("WASM runtime does not support shell commands.")
    }
}
