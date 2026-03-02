#[cfg(feature = "phase3-orchestration")]
pub mod build;
pub mod docker;
#[cfg(feature = "phase3-orchestration")]
pub mod error_analyzer;
#[cfg(feature = "phase3-orchestration")]
pub mod evolution;
#[cfg(feature = "phase3-orchestration")]
pub mod evolution_workflow;
#[cfg(feature = "phase3-orchestration")]
pub mod hot_swap;
#[cfg(feature = "phase3-orchestration")]
pub mod manager;
pub mod native;
#[cfg(feature = "rollback-recovery")]
pub mod rollback;
pub mod sandbox;
#[cfg(feature = "phase3-orchestration")]
pub mod state_migration;
pub mod traits;
pub mod wasm;

#[cfg(feature = "runtime-wasm")]
pub mod bindings;
#[cfg(feature = "runtime-wasm")]
pub mod bridges;
#[cfg(feature = "runtime-wasm")]
pub mod state;

#[cfg(feature = "phase3-orchestration")]
pub use build::{BuildAutomation, BuildAutomationImpl, BuildConfig, BuildResult};
pub use docker::DockerRuntime;
#[cfg(feature = "phase3-orchestration")]
pub use error_analyzer::{
    AutoFixResult, AutoFixer, BuildContext, BuildError, ErrorAnalyzer, FixSuggestion,
};
#[cfg(feature = "phase3-orchestration")]
pub use evolution::{
    EvolutionConfig, EvolutionEngine, EvolutionPlan, EvolutionResult, EvolutionStatus,
    EvolutionTrigger, EvolutionType,
};
#[cfg(feature = "phase3-orchestration")]
pub use hot_swap::{
    HotSwap, HotSwapManager, ModuleMetadata, ModuleVersion, SwapResult, SwapStrategy,
};
pub use native::NativeRuntime;
#[cfg(feature = "rollback-recovery")]
pub use rollback::{
    BackupConfig, RecoveryPlan, RecoveryStep, RecoveryStepType, RollbackManager, RollbackPoint,
    RollbackPointType, RollbackResult,
};
pub use sandbox::{BuildSandbox, SandboxConfig, SandboxManager, SandboxResult};
#[cfg(feature = "phase3-orchestration")]
pub use state_migration::{MigrationPlan, StateMigrator, StateSnapshot};
pub use traits::RuntimeAdapter;
pub use wasm::{WasmCapabilities, WasmRuntime};

use crate::config::RuntimeConfig;

/// Factory: create the right runtime from config
pub fn create_runtime(config: &RuntimeConfig) -> anyhow::Result<Box<dyn RuntimeAdapter>> {
    match config.kind.as_str() {
        "native" => Ok(Box::new(NativeRuntime::new())),
        "docker" => Ok(Box::new(DockerRuntime::new(config.docker.clone()))),
        "wasm" => Ok(Box::new(WasmRuntime::new(config.wasm.clone()))),
        "cloudflare" => anyhow::bail!(
            "runtime.kind='cloudflare' is not implemented yet. Use runtime.kind='native' for now."
        ),
        other if other.trim().is_empty() => {
            anyhow::bail!("runtime.kind cannot be empty. Supported values: native, docker, wasm")
        }
        other => {
            anyhow::bail!("Unknown runtime kind '{other}'. Supported values: native, docker, wasm")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_native() {
        let cfg = RuntimeConfig {
            kind: "native".into(),
            ..RuntimeConfig::default()
        };
        let rt = create_runtime(&cfg).unwrap();
        assert_eq!(rt.name(), "native");
        assert!(rt.has_shell_access());
    }

    #[test]
    fn factory_docker() {
        let cfg = RuntimeConfig {
            kind: "docker".into(),
            ..RuntimeConfig::default()
        };
        let rt = create_runtime(&cfg).unwrap();
        assert_eq!(rt.name(), "docker");
        assert!(rt.has_shell_access());
    }

    #[test]
    fn factory_wasm() {
        let cfg = RuntimeConfig {
            kind: "wasm".into(),
            ..RuntimeConfig::default()
        };
        let rt = create_runtime(&cfg).unwrap();
        assert_eq!(rt.name(), "wasm");
        assert!(!rt.has_shell_access());
    }

    #[test]
    fn factory_cloudflare_errors() {
        let cfg = RuntimeConfig {
            kind: "cloudflare".into(),
            ..RuntimeConfig::default()
        };
        match create_runtime(&cfg) {
            Err(err) => assert!(err.to_string().contains("not implemented")),
            Ok(_) => panic!("cloudflare runtime should error"),
        }
    }

    #[test]
    fn factory_unknown_errors() {
        let cfg = RuntimeConfig {
            kind: "wasm-edge-unknown".into(),
            ..RuntimeConfig::default()
        };
        match create_runtime(&cfg) {
            Err(err) => assert!(err.to_string().contains("Unknown runtime kind")),
            Ok(_) => panic!("unknown runtime should error"),
        }
    }

    #[test]
    fn factory_empty_errors() {
        let cfg = RuntimeConfig {
            kind: String::new(),
            ..RuntimeConfig::default()
        };
        match create_runtime(&cfg) {
            Err(err) => assert!(err.to_string().contains("cannot be empty")),
            Ok(_) => panic!("empty runtime should error"),
        }
    }
}
