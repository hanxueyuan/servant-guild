# Phase 3: Orchestration

## Overview

Phase 3 implements the orchestration layer that enables ServantGuild to autonomously manage its lifecycle, including GitHub integration, build automation, hot-swapping, rollback & recovery, and self-evolution capabilities.

## Objectives

1. **GitHub Integration**: Enable autonomous access to source code for self-evolution
2. **Build Automation**: Automate Rust/Wasm compilation and testing
3. **Hot-Swap Mechanism**: Enable runtime module replacement without restart
4. **Rollback & Recovery**: Provide safe version management and disaster recovery
5. **Self-Evolution**: Enable the system to improve itself autonomously

## Deliverables

### ✅ Completed Deliverables

#### 1. GitHub Bridge (`src/runtime/bridges/github.rs`)

**Purpose**: Provide autonomous access to source code and repository management.

**Key Features**:
- Repository cloning and pulling
- Branch creation and management
- File reading and writing
- Pull request creation and management
- Release management
- Commit history tracking

**Key Structures**:
```rust
pub struct GitHubCredentials { /* ... */ }
pub struct GitHubRepo { /* ... */ }
pub struct GitHubPullRequest { /* ... */ }
pub struct GitHubRelease { /* ... */ }

#[async_trait]
pub trait GitHubBridge: Send + Sync {
    async fn clone_repo(&self, path: PathBuf) -> Result<()>;
    async fn pull(&self, path: PathBuf) -> Result<()>;
    async fn commit(&self, path: PathBuf, message: String, author: Option<String>) -> Result<()>;
    async fn push(&self, path: PathBuf, branch: Option<String>) -> Result<()>;
    async fn create_branch(&self, path: PathBuf, branch_name: String) -> Result<()>;
    // ... and more
}
```

**Usage Example**:
```rust
use servant_guild::runtime::bridges::github::{GitHubBridge, GitHubCredentials};

let credentials = GitHubCredentials::new(
    "ghp_xxx".to_string(),
    "my-org".to_string(),
    "servant-guild".to_string(),
);

let bridge = GitHubBridgeImpl::new(credentials)
    .with_local_path(PathBuf::from("/workspace/projects"));

// Clone repository
bridge.clone_repo(PathBuf::from("/workspace")).await?;

// Create feature branch
bridge.create_branch(PathBuf::from("/workspace/servant-guild"), "feature/auto-opt".to_string()).await?;

// Update file
bridge.update_file(
    "src/servants/coordinator.rs".to_string(),
    new_content,
    "Add caching optimization".to_string(),
    Some("feature/auto-opt".to_string())
).await?;

// Create pull request
let pr = bridge.create_pr(
    "Auto-optimization: Add caching".to_string(),
    "This PR adds a caching layer to reduce latency".to_string(),
    "feature/auto-opt".to_string(),
    "main".to_string(),
).await?;
```

#### 2. Build Automation (`src/runtime/build.rs`)

**Purpose**: Automate Rust/Wasm compilation, testing, and dependency management.

**Key Features**:
- Cargo build execution (dev/release profiles)
- Automated testing
- Clippy linting
- Formatting checks
- Dependency management
- Artifact tracking

**Key Structures**:
```rust
pub enum BuildTarget {
    WasmComponent,
    NativeBinary,
    Dev,
    Release,
}

pub struct BuildConfig { /* ... */ }

#[async_trait]
pub trait BuildAutomation: Send + Sync {
    async fn build(&self, config: BuildConfig, project_path: PathBuf) -> Result<BuildResult>;
    async fn test(&self, project_path: PathBuf, args: Vec<String>) -> Result<BuildResult>;
    async fn check_fmt(&self, project_path: PathBuf) -> Result<bool>;
    async fn clippy(&self, project_path: PathBuf, args: Vec<String>) -> Result<BuildResult>;
    async fn update_deps(&self, project_path: PathBuf) -> Result<Vec<DependencyInfo>>;
    // ... and more
}
```

**Usage Example**:
```rust
use servant_guild::runtime::build::{BuildAutomation, BuildConfig, BuildTarget};

let build_config = BuildConfig::new(BuildTarget::WasmComponent)
    .with_feature("llm".to_string())
    .without_tests()
    .without_clippy();

let result = automation.build(build_config, PathBuf::from("/workspace/projects")).await?;

if result.success {
    println!("Build completed in {}ms", result.duration_ms);
    for artifact in result.artifacts {
        println!("Artifact: {} ({} bytes)", artifact.artifact_type, artifact.size);
    }
}
```

#### 3. Hot-Swap Mechanism (`src/runtime/hot_swap.rs`)

**Purpose**: Enable seamless runtime replacement of Wasm components without system restart.

**Key Features**:
- Multiple swap strategies (Immediate, Graceful, Staged)
- Module version tracking
- Atomic swapping
- Rollback support
- Capability extraction

**Key Structures**:
```rust
pub enum SwapStrategy {
    Immediate,
    Graceful { timeout_secs: u64 },
    Staged { initial_percent: u32, migration_interval_secs: u64 },
}

pub struct ModuleVersion { /* ... */ }
pub struct ModuleMetadata { /* ... */ }
pub struct SwapResult { /* ... */ }

#[async_trait]
pub trait HotSwap: Send + Sync {
    async fn load_module(&self, name: String, wasm_path: PathBuf, version: ModuleVersion) -> Result<ModuleMetadata>;
    async fn hot_swap(&self, module_name: String, new_version: ModuleVersion, strategy: SwapStrategy) -> Result<SwapResult>;
    async fn rollback(&self, module_name: String, target_version: ModuleVersion, reason: String) -> Result<SwapResult>;
    async fn get_active_version(&self, module_name: String) -> Option<ModuleVersion>;
    async fn get_module_history(&self, module_name: String) -> Vec<ModuleVersion>;
}
```

**Usage Example**:
```rust
use servant_guild::runtime::hot_swap::{HotSwap, ModuleVersion, SwapStrategy};

// Load new module
let metadata = manager.load_module(
    "coordinator".to_string(),
    PathBuf::from("target/wasm32-unknown-unknown/release/coordinator.wasm"),
    ModuleVersion::new("2.0.0".to_string()).with_commit("abc123".to_string()),
).await?;

// Hot-swap with graceful strategy
let result = manager.hot_swap(
    "coordinator".to_string(),
    ModuleVersion::new("2.0.0".to_string()),
    SwapStrategy::Graceful { timeout_secs: 30 },
).await?;

if result.success {
    println!("Hot-swap completed in {}ms", result.duration_ms);
}

// Rollback if needed
if issues_detected {
    manager.rollback(
        "coordinator".to_string(),
        ModuleVersion::new("1.0.0".to_string()),
        "Performance degradation detected".to_string(),
    ).await?;
}
```

#### 4. Rollback & Recovery (`src/runtime/rollback.rs`)

**Purpose**: Provide safe version management, state preservation, and disaster recovery.

**Key Features**:
- Rollback point creation (manual, pre-deployment, post-deployment, error recovery)
- State snapshot management
- Configuration preservation
- Recovery plan generation
- Automated rollback execution
- Backup retention policy

**Key Structures**:
```rust
pub enum RollbackPointType {
    ManualCheckpoint,
    PreDeployment,
    PostDeployment,
    ErrorRecovery,
    ScheduledBackup,
}

pub struct RollbackPoint { /* ... */ }
pub struct RollbackResult { /* ... */ }
pub struct RecoveryPlan { /* ... */ }

pub struct RollbackManager { /* ... */ }
```

**Usage Example**:
```rust
use servant_guild::runtime::rollback::{RollbackManager, RollbackPointType, BackupConfig};

// Create rollback point before deployment
let checkpoint = manager.create_rollback_point(
    RollbackPointType::PreDeployment,
    "Before deploying v2.0.0".to_string(),
).await?;

println!("Rollback point created: {}", checkpoint.id);

// Perform deployment
// ...

// If issues occur, rollback
if deployment_failed {
    let result = manager.rollback(checkpoint.id).await?;
    println!("Rollback completed: {}", result.success);
}

// List all rollback points
let points = manager.list_rollback_points(Some(10)).await?;
for point in points {
    println!("{} - {} ({:?})", point.timestamp, point.description, point.point_type);
}
```

#### 5. Self-Evolution Engine (`src/runtime/evolution.rs`)

**Purpose**: Enable the system to autonomously analyze performance, identify improvement opportunities, generate code changes, and deploy them safely.

**Key Features**:
- Evolution triggers (performance, user feedback, errors, scheduled, manual)
- Evolution types (optimization, bug fix, feature, dependency update, refactoring, security)
- LLM-powered analysis and code generation
- Risk assessment and approval workflow
- Automated deployment with rollback safety
- Impact estimation

**Key Structures**:
```rust
pub enum EvolutionTrigger {
    PerformanceThreshold { metric: String, threshold: f64 },
    UserFeedback,
    ErrorRateExceeded { rate: f64 },
    ScheduledEvolution,
    ManualTrigger,
    NewDependencyAvailable { dependency: String },
}

pub enum EvolutionType {
    PerformanceOptimization,
    BugFix,
    FeatureAddition,
    DependencyUpdate,
    Refactoring,
    SecurityImprovement,
}

pub enum EvolutionStatus {
    PendingAnalysis,
    Analyzing,
    GeneratingChanges,
    Building,
    Testing,
    Reviewing,
    PendingApproval,
    Deploying,
    Completed,
    RolledBack,
    Failed,
}

pub struct EvolutionPlan { /* ... */ }
pub struct EvolutionEngine { /* ... */ }
```

**Usage Example**:
```rust
use servant_guild::runtime::evolution::{EvolutionEngine, EvolutionConfig, EvolutionTrigger};

let config = EvolutionConfig {
    auto_evolution_enabled: true,
    min_confidence_threshold: 0.85,
    require_approval_for_features: true,
    ..Default::default()
};

let engine = EvolutionEngine::new(
    state,
    llm,
    github,
    build,
    hot_swap,
    rollback,
    config,
);

// Trigger evolution based on performance
let plan = engine.trigger_evolution(
    EvolutionTrigger::PerformanceThreshold {
        metric: "coordinator_latency_ms".to_string(),
        threshold: 1000.0,
    }
).await?;

println!("Evolution plan created: {} - {}", plan.id, plan.title);
println!("Risk level: {}", plan.risk_assessment.risk_level);

// Execute evolution (auto-approve if safe)
if !plan.risk_assessment.requires_human_approval {
    let result = engine.execute_evolution(plan.id.clone(), true).await?;
    if result.success {
        println!("Evolution completed: {}% performance improvement",
                 result.actual_performance_improvement.unwrap_or(0.0));
    }
}
```

## Integration Flow

The orchestration components work together as follows:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Self-Evolution Engine                         │
│  - Monitors system performance                                  │
│  - Triggers evolution based on metrics                           │
│  - Uses LLM to analyze and generate changes                      │
└────────────┬────────────────────────────────────────────────────┘
             │
             ├─────────────────────────────────────────────────────┐
             │                                                     │
             ▼                                                     ▼
    ┌─────────────────┐                                  ┌─────────────────┐
    │  GitHub Bridge  │                                  │ Build Automation│
    │  - Read code    │                                  │  - Compile      │
    │  - Write changes│                                  │  - Test         │
    │  - Create PR    │                                  │  - Check lint   │
    └────────┬────────┘                                  └────────┬────────┘
             │                                                     │
             └─────────────────────────────────────────────────────┘
                              │
                              ▼
                     ┌─────────────────┐
                     │  Hot-Swap Mgr   │
                     │  - Load module  │
                     │  - Swap         │
                     │  - Track version│
                     └────────┬────────┘
                              │
                              ▼
                     ┌─────────────────┐
                     │Rollback Manager │
                     │  - Create checkpoint │
                     │  - Restore state │
                     │  - Disaster recovery│
                     └─────────────────┘
```

## Testing

### Unit Tests

Each module includes comprehensive unit tests:

```bash
cargo test --lib runtime::bridges::github
cargo test --lib runtime::build
cargo test --lib runtime::hot_swap
cargo test --lib runtime::rollback
cargo test --lib runtime::evolution
```

### Integration Tests

Run full integration tests:

```bash
cargo test --test phase3_integration_test
```

## API Documentation

See the generated API documentation for detailed information:

```bash
cargo doc --open
```

## Configuration

### GitHub Configuration

Set environment variables:

```bash
export SERVANT_GITHUB_PAT="ghp_xxx"
export SERVANT_GITHUB_OWNER="my-org"
export SERVANT_GITHUB_REPO="servant-guild"
```

### Evolution Configuration

Configure evolution behavior in `config.toml`:

```toml
[evolution]
auto_evolution_enabled = false
min_confidence_threshold = 0.85
max_auto_approval_risk = "medium"
require_approval_for_features = true
monitoring_interval_secs = 300
evolution_history_limit = 1000
```

## Performance Considerations

1. **Hot-Swap Overhead**: Hot-swapping has minimal overhead (~10-100ms depending on strategy)
2. **Rollback Points**: Each rollback point consumes storage; implement retention policies
3. **Build Time**: Wasm builds take 2-5 minutes; consider incremental builds
4. **Evolution Analysis**: LLM analysis takes 5-30 seconds; cache results when possible

## Security Considerations

1. **GitHub PAT**: Store PAT securely; use least-privilege tokens
2. **Code Review**: Require human approval for high-risk changes
3. **Rollback Safety**: Always create rollback points before deployment
4. **Dependency Updates**: Review dependency updates before auto-approval
5. **Access Control**: Limit GitHub API access to necessary operations

## Future Enhancements

1. **Advanced Monitoring**: Integrate Prometheus/Grafana for metrics
2. **A/B Testing**: Support gradual rollout with metrics comparison
3. **Multi-Environment**: Support staging, production environments
4. **Rollback Preview**: Preview rollback before execution
5. **Evolution Templates**: Predefined evolution patterns

## Migration Notes

When migrating from Phase 2:

1. Update dependencies in `Cargo.toml`
2. Initialize orchestration components in main
3. Configure GitHub credentials
4. Set up backup storage
5. Enable/ disable auto-evolution as needed

## Support

For issues or questions:

1. Check the API documentation
2. Review test cases for usage examples
3. Enable debug logging for troubleshooting
4. Check rollback points for recovery options

## Status

✅ **Phase 3 Complete** - All orchestration features implemented and ready for integration testing.

## Next Steps

- **Phase 4**: Deployment & DevOps
- **Phase 5**: Monitoring & Observability
- **Phase 6**: Advanced Features (Multi-tenancy, Federation, etc.)
