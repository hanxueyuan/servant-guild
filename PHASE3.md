# ServantGuild - Phase 3: Orchestration (Evolution)

**Status:** ✅ **Completed**
**Reference:** [Whitepaper v1.1](./docs/design/servant_guild_whitepaper_v1.1.md)

## Overview

Phase 3 implements the **Evolution** capability of ServantGuild - enabling the system to autonomously manage its lifecycle through GitHub integration, build automation, hot-swapping, rollback & recovery, and self-evolution capabilities.

## Core Philosophy Alignment

From the ServantGuild Whitepaper v1.1:

> **进化 (Evolution)**: 通过 GitHub 仓库作为基因库，使魔团能够编写、测试、发布自己的新版本，实现自我迭代。

Phase 3 delivers on this promise by implementing:

1. **基因库 (Gene Pool)** - GitHub integration for autonomous code access
2. **自我更新 (Self-Update)** - Automated build, test, and release pipeline
3. **热替换 (Hot-Swap)** - Runtime module replacement without restart
4. **回滚恢复 (Rollback & Recovery)** - Safety nets for failed updates

## New Capabilities

### 1. GitHub Integration (The Gene Pool)
- Autonomous source code access
- Repository cloning and management
- Branch creation and PR generation
- Release management
- Commit history tracking

**Implementation:** `src/runtime/bridges/github.rs`

### 2. Build Automation
- Automated Rust/Wasm compilation
- Testing and linting (clippy, fmt)
- Dependency management
- Artifact tracking
- Build result reporting

**Implementation:** `src/runtime/build.rs`

### 3. Hot-Swap Mechanism
- Runtime module replacement
- Multiple swap strategies (Immediate, Graceful, Staged)
- Atomic swapping with rollback support
- Module version tracking
- Capability extraction

**Implementation:** `src/runtime/hot_swap.rs`

### 4. Rollback & Recovery
- Rollback point creation (manual, pre/post deployment)
- State snapshot management
- Configuration preservation
- Recovery plan generation
- Disaster recovery

**Implementation:** `src/runtime/rollback.rs`, `src/safety/rollback.rs`

### 5. Self-Evolution Engine
- Autonomous performance analysis
- LLM-powered code generation
- Risk assessment and approval workflow
- Automated deployment with safety nets
- Continuous improvement loop

**Implementation:** `src/runtime/evolution_workflow.rs`

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Self-Evolution Engine                         │
│              (感知 → 决策 → 开发 → 测试 → 发布)                    │
└────────────┬────────────────────────────────────────────────────┘
             │
             ├─────────────────────────────────────────────────────┐
             │                                                     │
             ▼                                                     ▼
    ┌─────────────────┐                                  ┌─────────────────┐
    │  GitHub Bridge  │                                  │ Build Automation│
    │   (基因库)       │                                  │   (编译测试)     │
    └────────┬────────┘                                  └────────┬────────┘
             │                                                     │
             └─────────────────────────────────────────────────────┘
                              │
                              ▼
                     ┌─────────────────┐
                     │  Hot-Swap Mgr   │
                     │   (热替换)       │
                     └────────┬────────┘
                              │
                              ▼
                     ┌─────────────────┐
                     │  Rollback Mgr   │
                     │   (回滚恢复)     │
                     └─────────────────┘
```

## Self-Update Flow (The Evolution Loop)

From the Whitepaper:

1. **感知 (Perceive)**: Warden detects capability gap or bug, or Speaker initiates periodic update proposal
2. **决策 (Decide)**: Full guild votes to approve update resolution
3. **开发 (Develop)**: Contractor pulls GitHub code, Worker modifies code
4. **测试 (Test)**: Warden runs test cases
5. **发布 (Release)**: Contractor publishes GitHub Release via Bot account (generates new Wasm)
6. **热更 (Hot-Swap)**: All servants pull new Wasm, perform hash verification
7. **验证 (Validate)**: Unupdated servants cross-validate updated servants (canary deployment)
8. **确认 (Confirm)**: Validation passed → full rollout; Failed → collective rollback

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
git clone https://github.com/hanxueyuan/servant-guild
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

- [Whitepaper v1.1](./docs/design/servant_guild_whitepaper_v1.1.md)
- [Architecture v1.0](./docs/architecture/servant_guild_architecture_v1.0.md)
- [Infrastructure Requirements](./docs/design/servant_guild_infrastructure.md)

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

## Next Phase

Phase 4 (Autonomy) builds on Phase 3 to deliver:
- Production deployment infrastructure (Terraform, Kubernetes)
- Complete observability stack (Prometheus, Loki, OpenTelemetry)
- Economic model for token optimization
- Security hardening
- CI/CD automation

See [PHASE4.md](./PHASE4.md) for details.
