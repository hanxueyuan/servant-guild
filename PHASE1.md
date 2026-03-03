# ServantGuild - Phase 1: Genesis (Foundation)

**Status:** ✅ **Completed**
**Reference:** [Whitepaper v1.1](./docs/design/servant_guild_whitepaper_v1.1.md)

## Overview

Phase 1 implements the **Genesis** foundation of ServantGuild - establishing the Wasmtime-based runtime infrastructure, core traits, and the five permanent servant roles.

## Core Philosophy Alignment

From the ServantGuild Whitepaper v1.1:

> **Phase 1: 原型 (Genesis)**
> - 在 ZeroClaw 中集成 Wasmtime 宿主。
> - 将 `src/tools` 封装为 Wasm 可调用的 Host Functions。
> - 实现 CLI 交互界面适配使魔团指令。

Phase 1 delivers on this promise by implementing:

1. **Wasmtime Host Integration** - The foundation for sandboxed execution
2. **Host Functions** - Tools exposed to Wasm guests
3. **Core Trait Definitions** - Provider, Channel, Tool, Memory, Runtime
4. **Five Servant Roles** - Coordinator, Worker, Warden, Speaker, Contractor

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          Master Daemon (Rust)                            │
│                      ┌─────────────────────────────┐                     │
│                      │    Wasmtime Runtime Host    │                     │
│                      │    ┌───────────────────┐    │                     │
│                      │    │  Host Functions   │    │                     │
│                      │    │  (tools, network, │    │                     │
│                      │    │   memory, crypto) │    │                     │
│                      │    └─────────┬─────────┘    │                     │
│                      └──────────────┼──────────────┘                     │
│                                     │                                     │
└─────────────────────────────────────┼────────────────────────────────────┘
                                      │
           ┌──────────────────────────┼──────────────────────────┐
           │                          │                          │
           ▼                          ▼                          ▼
    ┌─────────────┐            ┌─────────────┐            ┌─────────────┐
    │ Coordinator │            │   Worker    │            │   Warden    │
    │   (Wasm)    │            │   (Wasm)    │            │   (Wasm)    │
    └─────────────┘            └─────────────┘            └─────────────┘
           │                          │                          │
           │                          │                          │
           ▼                          ▼                          ▼
    ┌─────────────┐            ┌─────────────┐
    │   Speaker   │            │ Contractor  │
    │   (Wasm)    │            │   (Wasm)    │
    └─────────────┘            └─────────────┘
```

## Core Components

### 1. Wasmtime Integration

The runtime layer that provides sandboxed execution for all servants:

```rust
// src/runtime/wasm.rs
pub struct WasmRuntime {
    engine: Engine,
    linker: Linker<HostState>,
    config: RuntimeConfig,
}

impl WasmRuntime {
    /// Create new runtime with security constraints
    pub fn new(config: RuntimeConfig) -> Result<Self>;
    
    /// Load and instantiate a servant module
    pub async fn load_servant(&mut self, wasm_bytes: &[u8]) -> Result<ServantInstance>;
    
    /// Execute a task within the sandbox
    pub async fn execute(&self, instance: &ServantInstance, task: Task) -> Result<TaskResult>;
}
```

**Security Constraints:**
- Memory limit: 512MB per instance
- CPU fuel: 5s max per call
- File system: Pre-opened directories only
- Network: Whitelisted domains only

### 2. Host Functions

Tools and capabilities exposed to Wasm guests:

```rust
// wit/host.wit
interface host {
    // File operations
    file-read: func(path: string) -> result<list<u8>, error>;
    file-write: func(path: string, data: list<u8>) -> result<_, error>;
    
    // Network operations
    http-fetch: func(url: string, opts: http-options) -> result<http-response, error>;
    
    // Memory operations
    memory-store: func(key: string, value: list<u8>) -> result<_, error>;
    memory-load: func(key: string) -> result<list<u8>, error>;
    
    // Crypto operations
    encrypt: func(data: list<u8>) -> result<list<u8>, error>;
    decrypt: func(data: list<u8>) -> result<list<u8>, error>;
    
    // LLM operations
    llm-complete: func(prompt: string, opts: llm-options) -> result<string, error>;
}
```

### 3. Core Traits

The foundation for all extensibility:

```rust
// src/providers/traits.rs
#[async_trait]
pub trait Provider: Send + Sync {
    async fn complete(&self, prompt: &str, opts: CompleteOptions) -> Result<String>;
}

// src/channels/traits.rs
#[async_trait]
pub trait Channel: Send + Sync {
    async fn send(&self, message: &Message) -> Result<()>;
    async fn receive(&self) -> Result<Vec<Message>>;
}

// src/tools/traits.rs
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    async fn execute(&self, input: Value) -> Result<Value>;
}

// src/memory/traits.rs
#[async_trait]
pub trait Memory: Send + Sync {
    async fn store(&self, key: &str, value: &[u8]) -> Result<()>;
    async fn load(&self, key: &str) -> Result<Vec<u8>>;
}
```

## The Five Servant Roles

### Coordinator (枢机团长)
- **Role**: Master communication interface, task dispatch, status reporting
- **Implementation**: `src/servants/coordinator/`
- **Wasm Module**: `coordinator.wasm`

```rust
pub struct Coordinator {
    id: ServantId,
    task_queue: TaskQueue,
    servants: HashMap<ServantRole, Vec<ServantId>>,
}

impl Coordinator {
    /// Dispatch task to appropriate servant
    pub async fn dispatch(&mut self, task: Task) -> Result<TaskResult>;
    
    /// Get guild status
    pub fn status(&self) -> GuildStatus;
}
```

### Worker (执行使魔)
- **Role**: Core execution role, code writing, tool invocation
- **Implementation**: `src/servants/worker/`
- **Wasm Module**: `worker.wasm`

```rust
pub struct Worker {
    id: ServantId,
    tools: Arc<ToolRegistry>,
    workspace: PathBuf,
}

impl Worker {
    /// Execute a task
    pub async fn execute(&mut self, task: Task) -> Result<TaskResult>;
    
    /// Modify code (with audit)
    pub async fn modify_code(&mut self, changes: Vec<CodeChange>) -> Result<()>;
}
```

### Warden (监工使魔)
- **Role**: Security audit, performance monitoring, version validation
- **Implementation**: `src/servants/warden/`
- **Wasm Module**: `warden.wasm`

```rust
pub struct Warden {
    id: ServantId,
    audit_log: Arc<AuditLogger>,
    metrics: Arc<MetricsCollector>,
}

impl Warden {
    /// Audit a modification
    pub async fn audit(&self, record: &ModificationRecord) -> Result<AuditResult>;
    
    /// Run security scan
    pub async fn scan(&self, target: &Path) -> Result<SecurityReport>;
}
```

### Speaker (议长使魔)
- **Role**: Organize guild meetings, collect votes, tally consensus
- **Implementation**: `src/servants/speaker/`
- **Wasm Module**: `speaker.wasm`

```rust
pub struct Speaker {
    id: ServantId,
    consensus: Arc<ConsensusEngine>,
}

impl Speaker {
    /// Propose a vote
    pub async fn propose(&mut self, proposal: Proposal) -> Result<()>;
    
    /// Tally votes
    pub async fn tally(&self, proposal_id: &str) -> Result<VoteTally>;
}
```

### Contractor (契约使魔)
- **Role**: Servant creation/destruction, config management, version release
- **Implementation**: `src/servants/contractor/`
- **Wasm Module**: `contractor.wasm`

```rust
pub struct Contractor {
    id: ServantId,
    registry: Arc<ServantRegistry>,
    github: Arc<GitHubBridge>,
}

impl Contractor {
    /// Create new servant instance
    pub async fn create_servant(&mut self, role: ServantRole) -> Result<ServantId>;
    
    /// Release new version
    pub async fn release(&mut self, version: &str) -> Result<()>;
}
```

## Communication Model

### Owner → Master → Servants

```
┌──────────┐      CLI/API      ┌───────────┐      Internal      ┌──────────┐
│  Owner   │ ──────────────────│   Master  │ ──────────────────▶│ Servants │
│ (Human)  │                    │  (Daemon) │                    │  (Wasm)  │
└──────────┘                    └───────────┘                    └──────────┘
     │                                │                                │
     │         Commands               │          Tasks                 │
     │◀───────────────────────────────│◀───────────────────────────────│
                                     │         Results                │
                                     │                                │
```

### Servant ↔ Servant (via Master)

All inter-servant communication goes through the Master daemon to ensure auditability:

```rust
pub enum GuildMessage {
    TaskAssign { servant: ServantRole, task: Task },
    TaskResult { servant: ServantId, result: TaskResult },
    VoteRequest { proposal: Proposal },
    VoteCast { voter: ServantId, vote: Vote },
    Alert { severity: Severity, message: String },
}
```

## Configuration

```toml
# servant-guild.toml

[guild]
name = "ServantGuild-Alpha"
version = "0.1.0"

[runtime]
engine = "wasmtime"
max_memory_mb = 512
max_cpu_time_ms = 5000
preopen_dirs = ["/workspace"]

[servants.coordinator]
enabled = true

[servants.worker]
enabled = true
workspace = "/workspace/worker"

[servants.warden]
enabled = true
audit_enabled = true

[servants.speaker]
enabled = true

[servants.contractor]
enabled = true
```

## Testing

```bash
# Run all tests
cargo test

# Run Phase 1 integration tests
cargo test --test phase1_integration_test

# Build all servant modules
cargo build --target wasm32-unknown-unknown --release
```

## Next Phase

Phase 2 (Assembly) builds on Phase 1 to deliver:
- Consensus Engine implementation
- LLM provider integration
- Context and memory management
- Inter-servant communication

See [PHASE2.md](./PHASE2.md) for details.
