# ServantGuild - Phase 3: Orchestration

## What's New in Phase 3

Phase 3 implements the orchestration layer that enables ServantGuild to autonomously manage its lifecycle, including GitHub integration, build automation, hot-swapping, rollback & recovery, and self-evolution capabilities.

### New Capabilities

#### 1. GitHub Integration
- Autonomous source code access
- Repository cloning and management
- Branch creation and PR generation
- Release management
- Commit history tracking

#### 2. Build Automation
- Automated Rust/Wasm compilation
- Testing and linting (clippy, fmt)
- Dependency management
- Artifact tracking
- Build result reporting

#### 3. Hot-Swap Mechanism
- Runtime module replacement
- Multiple swap strategies (Immediate, Graceful, Staged)
- Atomic swapping with rollback support
- Module version tracking
- Capability extraction

#### 4. Rollback & Recovery
- Rollback point creation (manual, pre/post deployment)
- State snapshot management
- Configuration preservation
- Recovery plan generation
- Disaster recovery

#### 5. Self-Evolution Engine
- Autonomous performance analysis
- LLM-powered code generation
- Risk assessment and approval workflow
- Automated deployment with safety nets
- Continuous improvement loop

## Quick Start

### Prerequisites

- Rust 1.87+ (⚠️ Current environment: 1.75.0, needs upgrade)
- Git 2.x+
- libgit2-dev (for GitHub integration)
- binaryen (optional, for Wasm optimization)

### Installation

```bash
# Install system dependencies
sudo apt-get install libgit2-dev binaryen

# Upgrade Rust
rustup update stable
rustup default stable

# Clone repository
git clone https://github.com/your-org/servant-guild
cd servant-guild

# Build with Phase 3 features
cargo build --release --features phase3-orchestration
```

### Configuration

Create `config.toml`:

```toml
[github]
pat = "ghp_your_token"
owner = "your-org"
repo = "servant-guild"

[evolution]
auto_evolution_enabled = false
min_confidence_threshold = 0.85
require_approval_for_features = true

[build]
run_tests = true
run_clippy = true
check_fmt = false

[backup]
backup_interval_secs = 3600
max_rollback_points = 100
include_state_snapshots = true
```

### Usage

#### Hot-Swap a Module

```rust
use servant_guild::runtime::hot_swap::{HotSwap, SwapStrategy, ModuleVersion};

// Load new module
let metadata = manager.load_module(
    "coordinator".to_string(),
    PathBuf::from("target/wasm32-unknown-unknown/release/coordinator.wasm"),
    ModuleVersion::new("2.0.0".to_string()),
).await?;

// Hot-swap with graceful strategy
let result = manager.hot_swap(
    "coordinator".to_string(),
    ModuleVersion::new("2.0.0".to_string()),
    SwapStrategy::Graceful { timeout_secs: 30 },
).await?;
```

#### Create Rollback Point

```rust
use servant_guild::runtime::rollback::{RollbackPointType, RollbackManager};

// Create rollback point before deployment
let checkpoint = manager.create_rollback_point(
    RollbackPointType::PreDeployment,
    "Before deploying v2.0.0".to_string(),
).await?;
```

#### Trigger Self-Evolution

```rust
use servant_guild::runtime::evolution::{EvolutionEngine, EvolutionTrigger};

// Trigger evolution based on performance
let plan = engine.trigger_evolution(
    EvolutionTrigger::PerformanceThreshold {
        metric: "coordinator_latency_ms".to_string(),
        threshold: 1000.0,
    }
).await?;

// Execute evolution (auto-approve if safe)
if !plan.risk_assessment.requires_human_approval {
    let result = engine.execute_evolution(plan.id.clone(), true).await?;
}
```

## Documentation

- [Phase 3 Documentation](./docs/phase3_orchestration.md)
- [Build Status](./docs/phase3_build_status.md)
- [API Reference](https://docs.rs/servant-guild)

## Testing

```bash
# Run all tests
cargo test

# Run Phase 3 integration tests
cargo test --test phase3_integration_test -- --ignored

# Generate documentation
cargo doc --open
```

## Feature Flags

- `runtime-wasm`: Wasm runtime support (default)
- `github-integration`: GitHub bridge
- `wasm-optimization`: Wasm optimization tools
- `rollback-recovery`: Rollback and recovery
- `phase3-orchestration`: All Phase 3 features
- `full`: All features

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Self-Evolution Engine                         │
└────────────┬────────────────────────────────────────────────────┘
             │
             ├─────────────────────────────────────────────────────┐
             │                                                     │
             ▼                                                     ▼
    ┌─────────────────┐                                  ┌─────────────────┐
    │  GitHub Bridge  │                                  │ Build Automation│
    └────────┬────────┘                                  └────────┬────────┘
             │                                                     │
             └─────────────────────────────────────────────────────┘
                              │
                              ▼
                     ┌─────────────────┐
                     │  Hot-Swap Mgr   │
                     └────────┬────────┘
                              │
                              ▼
                     ┌─────────────────┐
                     │Rollback Manager │
                     └─────────────────┘
```

## Progress

- ✅ GitHub Bridge
- ✅ Build Automation
- ✅ Hot-Swap Mechanism
- ✅ Rollback & Recovery
- ✅ Self-Evolution Engine
- ⏸️ Integration Testing (requires full environment)

## Known Issues

1. **Rust Version**: Requires Rust 1.87+, current environment is 1.75.0
2. **Git2 Dependency**: Requires `libgit2-dev` system library
3. **Integration Tests**: Require external credentials and full setup

## Next Steps

- Upgrade build environment to Rust 1.87+
- Run full integration test suite
- Performance benchmarking
- Security audit
- Production deployment

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

## License

MIT OR Apache-2.0

---

**Status**: Phase 3 Implemented | **Next**: Phase 4 - Deployment & DevOps
