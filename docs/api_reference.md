# ServantGuild API Reference

**Version**: 0.4.0  
**Last Updated**: 2025-01-16  
**Architecture**: [Whitepaper v1.1](./design/servant_guild_whitepaper_v1.1.md)

## Table of Contents

1. [Core Types](#core-types)
2. [Servant API](#servant-api)
3. [Consensus API](#consensus-api)
4. [Runtime API](#runtime-api)
5. [Evolution API](#evolution-api)
6. [Safety API](#safety-api)
7. [Security API](#security-api)
8. [Economic API](#economic-api)
9. [Error Handling](#error-handling)
10. [Configuration](#configuration)

---

## Core Types

### ServantId
```rust
pub struct ServantId {
    pub name: String,
    pub role: ServantRole,
    pub version: String,
}

pub enum ServantRole {
    Coordinator,  // 协调者 - 任务调度与冲突仲裁
    Worker,       // 执行者 - 任务执行与代码修改
    Warden,       // 监守者 - 质量保障与安全审计
    Speaker,      // 发言人 - 对外通信与决策汇报
    Contractor,   // 契约者 - 资源管理与外部对接
}
```

### Task
```rust
pub struct Task {
    pub id: String,
    pub task_type: TaskType,
    pub priority: Priority,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub deadline: Option<DateTime<Utc>>,
    pub assignee: Option<ServantRole>,  // 指派执行者
}

pub enum TaskType {
    // Core tasks
    Build,
    Test,
    Deploy,
    
    // Warden tasks
    Analyze,
    Audit,
    SecurityScan,
    
    // Speaker tasks
    Report,
    Alert,
    Communicate,
    
    // Contractor tasks
    ResourceAllocate,
    BudgetCheck,
    ExternalIntegrate,
    
    // Evolution tasks
    Evolve,
    SelfUpdate,
    HotSwap,
}
```

### Result
```rust
pub struct TaskResult {
    pub task_id: String,
    pub status: TaskStatus,
    pub output: Option<serde_json::Value>,
    pub errors: Vec<String>,
    pub duration_ms: u64,
    pub resources_used: ResourceUsage,  // Token, CPU, Memory
}

pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}
```

---

## Servant API

### Servant Trait (The Foundation)

All servants implement this core trait:

```rust
#[async_trait]
pub trait Servant: Send + Sync {
    /// Get servant identity
    fn id(&self) -> &ServantId;
    
    /// Get servant role
    fn role(&self) -> ServantRole;
    
    /// Initialize servant
    async fn initialize(&mut self, context: &GuildContext) -> Result<()>;
    
    /// Handle assigned task
    async fn handle_task(&mut self, task: Task) -> Result<TaskResult>;
    
    /// Handle incoming message
    async fn handle_message(&mut self, message: GuildMessage) -> Result<()>;
    
    /// Get current status
    fn status(&self) -> ServantStatus;
    
    /// Shutdown gracefully
    async fn shutdown(&mut self) -> Result<()>;
}
```

### Coordinator (协调者)

Task scheduling and conflict arbitration:

```rust
impl Coordinator {
    /// Create new coordinator
    pub fn new(id: ServantId, config: CoordinatorConfig) -> Self;
    
    /// Schedule a task to appropriate servant
    pub async fn schedule_task(&self, task: Task) -> Result<SchedulingDecision>;
    
    /// Resolve conflict between servants
    pub async fn resolve_conflict(&self, conflict: Conflict) -> Result<Resolution>;
    
    /// Get current task queue
    pub fn get_queue(&self) -> &TaskQueue;
    
    /// Prioritize tasks
    pub async fn prioritize(&mut self) -> Result<Vec<Task>>;
    
    /// Check resource availability
    pub async fn check_resources(&self) -> Result<ResourceStatus>;
}
```

### Worker (执行者)

Task execution and code modification:

```rust
impl Worker {
    /// Create new worker
    pub fn new(id: ServantId, config: WorkerConfig) -> Self;
    
    /// Execute a task
    pub async fn execute(&mut self, task: Task) -> Result<TaskResult>;
    
    /// Modify code (with audit)
    pub async fn modify_code(
        &mut self,
        target: &Path,
        changes: Vec<CodeChange>,
        audit_reason: &str,
    ) -> Result<ModificationResult>;
    
    /// Build project
    pub async fn build(&self, config: &BuildConfig) -> Result<BuildResult>;
    
    /// Run tests
    pub async fn test(&self, config: &TestConfig) -> Result<TestResult>;
    
    /// Get modification history
    pub fn get_modification_history(&self) -> &[ModificationRecord];
}
```

### Warden (监守者)

Quality assurance and security audit:

```rust
impl Warden {
    /// Create new warden
    pub fn new(id: ServantId, config: WardenConfig) -> Self;
    
    /// Audit a modification
    pub async fn audit_modification(&self, record: &ModificationRecord) -> Result<AuditResult>;
    
    /// Run security scan
    pub async fn security_scan(&self, target: &Path) -> Result<SecurityReport>;
    
    /// Validate task result
    pub async fn validate_result(&self, result: &TaskResult) -> Result<ValidationResult>;
    
    /// Create snapshot
    pub async fn create_snapshot(&self, reason: &str) -> Result<SnapshotId>;
    
    /// Check code quality
    pub async fn check_quality(&self, code: &str) -> Result<QualityReport>;
}
```

### Speaker (发言人)

External communication and decision reporting:

```rust
impl Speaker {
    /// Create new speaker
    pub fn new(id: ServantId, config: SpeakerConfig) -> Self;
    
    /// Report decision to owner
    pub async fn report_decision(&self, decision: &Decision) -> Result<()>;
    
    /// Send alert
    pub async fn send_alert(&self, alert: Alert) -> Result<()>;
    
    /// Handle incoming message from owner
    pub async fn handle_owner_message(&mut self, message: &str) -> Result<Option<Task>>;
    
    /// Generate status report
    pub async fn generate_report(&self) -> Result<StatusReport>;
    
    /// Get communication channels
    fn channels(&self) -> &[CommunicationChannel];
}
```

### Contractor (契约者)

Resource management and external integration:

```rust
impl Contractor {
    /// Create new contractor
    pub fn new(id: ServantId, config: ContractorConfig) -> Self;
    
    /// Allocate resources
    pub async fn allocate_resources(
        &mut self,
        request: ResourceRequest,
    ) -> Result<ResourceAllocation>;
    
    /// Check budget
    pub fn check_budget(&self) -> Result<BudgetStatus>;
    
    /// Integrate with external service
    pub async fn integrate(
        &mut self,
        service: ExternalService,
        config: IntegrationConfig,
    ) -> Result<IntegrationResult>;
    
    /// Manage GitHub repository
    pub async fn manage_repo(&self, action: RepoAction) -> Result<RepoResult>;
    
    /// Get resource usage
    fn get_usage(&self) -> &ResourceUsage;
}
```

---

## Consensus API

### ConsensusEngine

From the Whitepaper:
> **共识驱动 (Consensus-Driven)**: 所有关键决策需经多数同意，保障集体生存利益。

```rust
impl ConsensusEngine {
    /// Create a new consensus engine
    pub fn new() -> Self;
    
    /// Create with constitution (the rules)
    pub fn with_constitution(
        config: ConsensusConfig,
        constitution: Constitution,
    ) -> Self;
    
    /// Register a servant (gives voting rights)
    pub fn register_servant(&self, servant_id: String, role: ServantRole);
    
    /// Unregister a servant
    pub fn unregister_servant(&self, servant_id: &str);
    
    /// Create a proposal
    pub fn create_proposal(
        &self,
        title: String,
        description: String,
        proposer: String,
        decision_type: DecisionType,
        payload: Option<serde_json::Value>,
    ) -> Result<Proposal>;
    
    /// Cast a vote on a proposal
    pub fn cast_vote(
        &self,
        proposal_id: &str,
        voter: String,
        vote: Vote,
        reason: String,
    ) -> Result<()>;
    
    /// Evaluate a proposal
    pub fn evaluate_proposal(&self, proposal_id: &str) -> Result<VoteTally>;
    
    /// Owner veto a proposal
    pub fn veto_proposal(&self, proposal_id: &str, owner_id: &str) -> Result<()>;
}

pub enum DecisionType {
    Normal,    // 普通决策 - 3/5 通过
    Critical,  // 关键决策 - 5/5 全票通过
}
```

---

## Runtime API

### BuildSandbox

Secure isolated build environment:

```rust
impl BuildSandbox {
    /// Create a new build sandbox
    pub async fn new(id: String, config: SandboxConfig) -> Result<Self>;
    
    /// Execute a command in the sandbox
    pub async fn execute(
        &self,
        program: &str,
        args: &[&str],
        working_dir: Option<&Path>,
    ) -> Result<SandboxResult>;
    
    /// Copy project files into the sandbox
    pub async fn copy_project(&self, source: &Path) -> Result<()>;
    
    /// Clean up the sandbox
    pub async fn cleanup(&self) -> Result<()>;
    
    /// Get workspace path
    pub fn workspace(&self) -> &Path;
}
```

### HotSwapManager

Runtime module replacement:

```rust
impl HotSwapManager {
    /// Create new hot-swap manager
    pub fn new(config: HotSwapConfig) -> Self;
    
    /// Load a new module version
    pub async fn load_module(
        &mut self,
        module_id: String,
        wasm_path: PathBuf,
        version: ModuleVersion,
    ) -> Result<ModuleMetadata>;
    
    /// Perform hot-swap
    pub async fn hot_swap(
        &mut self,
        module_id: String,
        version: ModuleVersion,
        strategy: SwapStrategy,
    ) -> Result<SwapResult>;
    
    /// Get loaded modules
    pub fn get_modules(&self) -> &HashMap<String, Vec<ModuleMetadata>>;
    
    /// Get active version
    pub fn get_active_version(&self, module_id: &str) -> Option<&ModuleVersion>;
}

pub enum SwapStrategy {
    Immediate,                    // Instant swap
    Graceful { timeout_secs: u64 }, // Wait for in-flight ops
    Staged { stages: Vec<u8> },   // Gradual rollout
}
```

### RollbackManager

```rust
impl RollbackManager {
    /// Create new rollback manager
    pub fn new(config: RollbackConfig) -> Self;
    
    /// Create rollback point
    pub async fn create_rollback_point(
        &mut self,
        point_type: RollbackPointType,
        description: String,
    ) -> Result<RollbackPoint>;
    
    /// Perform rollback
    pub async fn rollback(&mut self, point_id: &str) -> Result<RollbackResult>;
    
    /// Get rollback points
    pub fn get_rollback_points(&self) -> &[RollbackPoint];
}

pub enum RollbackPointType {
    Manual,
    PreDeployment,
    PostDeployment,
    Periodic,
}
```

---

## Evolution API

### EvolutionEngine

From the Whitepaper:
> **进化 (Evolution)**: 通过 GitHub 仓库作为基因库，使魔团能够编写、测试、发布自己的新版本，实现自我迭代。

```rust
impl EvolutionEngine {
    /// Create new evolution engine
    pub fn new(config: EvolutionConfig) -> Self;
    
    /// Trigger evolution process
    pub async fn trigger_evolution(
        &mut self,
        trigger: EvolutionTrigger,
    ) -> Result<EvolutionPlan>;
    
    /// Execute evolution plan
    pub async fn execute_evolution(
        &mut self,
        plan_id: String,
        auto_approve: bool,
    ) -> Result<EvolutionResult>;
    
    /// Get active evolutions
    pub fn get_active_evolutions(&self) -> &[EvolutionPlan];
}

pub enum EvolutionTrigger {
    PerformanceDegradation { metric: String, current_value: f64, threshold: f64 },
    BugDetected { bug_id: String, severity: Severity },
    FeatureRequest { description: String },
    PeriodicUpdate { interval: Duration },
    OwnerCommand { command: String },
}
```

---

## Safety API

### CanaryTester

From the Whitepaper:
> **金丝雀发布 (Canary)**: 小范围先试，再逐步扩大，监控指标。

```rust
impl CanaryTester {
    /// Create with configuration
    pub fn new(config: CanaryConfig, metrics: Arc<dyn MetricsCollector>) -> Self;
    
    /// Start canary test
    pub async fn start_test(&self, module_id: &str, version: &str) -> Result<String>;
    
    /// Monitor test progress
    pub async fn monitor(&self, test_id: &str) -> Result<CanaryStatus>;
    
    /// Advance to next stage
    pub async fn advance(&self, test_id: &str) -> Result<CanaryStatus>;
    
    /// Abort and rollback
    pub async fn abort(&self, test_id: &str) -> Result<()>;
}
```

### SafetyManager

```rust
impl SafetyManager {
    /// Create new safety manager
    pub fn new(config: SafetyConfig) -> Self;
    
    /// Perform pre-check for operation
    pub async fn pre_check(&self, operation: &Operation) -> Result<PreCheckResult>;
    
    /// Create safety checkpoint
    pub async fn create_checkpoint(&mut self, reason: &str) -> Result<CheckpointId>;
    
    /// Verify operation safety
    pub async fn verify_safety(&self, operation: &Operation) -> Result<SafetyReport>;
}
```

---

## Security API

### AuditLogger

From the Architecture spec:
> **审计追踪**: 所有敏感操作需记录审计日志，支持合规审查。

```rust
impl AuditLogger {
    /// Create new audit logger
    pub fn new(config: AuditConfig) -> Self;
    
    /// Log audit event
    pub async fn log(&self, event: AuditEvent) -> Result<()>;
    
    /// Query audit log
    pub async fn query(&self, query: AuditQuery) -> Result<Vec<AuditEvent>>;
    
    /// Export for compliance
    pub async fn export(&self, format: ExportFormat) -> Result<Vec<u8>>;
}
```

### SecretsManager

```rust
impl SecretsManager {
    /// Create new secrets manager
    pub fn new(config: SecretsConfig) -> Result<Self>;
    
    /// Store secret
    pub async fn store(&mut self, key: &str, value: &[u8]) -> Result<()>;
    
    /// Retrieve secret
    pub async fn retrieve(&self, key: &str) -> Result<Vec<u8>>;
    
    /// Rotate secret
    pub async fn rotate(&mut self, key: &str) -> Result<()>;
}
```

### Encryption

```rust
impl Encryption {
    /// Create with ChaCha20-Poly1305 (recommended)
    pub fn new_chacha20(key: &[u8]) -> Result<Self>;
    
    /// Create with AES-256-GCM
    pub fn new_aes256(key: &[u8]) -> Result<Self>;
    
    /// Encrypt data
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<EncryptedData>;
    
    /// Decrypt data
    pub fn decrypt(&self, ciphertext: &EncryptedData) -> Result<Vec<u8>>;
}
```

---

## Economic API

### BudgetManager

From the Infrastructure spec:
> **经费管理**: 每个使魔有独立 Token 配额，余额不足时暂停活动并通知 Speaker。

```rust
impl BudgetManager {
    /// Create new budget manager
    pub fn new(config: BudgetConfig) -> Self;
    
    /// Allocate budget to servant
    pub async fn allocate(&mut self, servant_id: &str, amount: TokenAmount) -> Result<()>;
    
    /// Consume tokens
    pub async fn consume(&mut self, servant_id: &str, amount: u64) -> Result<()>;
    
    /// Check balance
    pub fn balance(&self, servant_id: &str) -> u64;
    
    /// Check if budget is healthy
    pub fn is_healthy(&self, servant_id: &str) -> bool;
}
```

### TokenOptimizer

```rust
impl TokenOptimizer {
    /// Create new optimizer
    pub fn new(config: OptimizerConfig) -> Self;
    
    /// Optimize prompt
    pub fn optimize_prompt(&self, prompt: &str) -> Result<OptimizedPrompt>;
    
    /// Compress context
    pub fn compress_context(&self, context: &str) -> Result<CompressedContext>;
    
    /// Select best provider
    pub fn select_provider(&self, requirements: &ProviderRequirements) -> Result<Provider>;
}
```

---

## Error Handling

All APIs return `Result<T, anyhow::Error>` for comprehensive error handling.

**Common Error Types**:
- `ServantError`: Servant lifecycle failures
- `ConsensusError`: Consensus decision failures
- `SandboxError`: Sandbox creation or execution failures
- `EvolutionError`: Evolution process failures
- `SecurityError`: Security operation failures
- `BudgetError`: Budget/token management failures

**Example**:
```rust
match coordinator.schedule_task(task).await {
    Ok(decision) => {
        println!("Task {} assigned to {:?}", task.id, decision.assignee);
    }
    Err(e) => {
        if let Some(se) = e.downcast_ref::<ServantError>() {
            eprintln!("Servant error: {:?}", se);
        } else if let Some(ce) = e.downcast_ref::<ConsensusError>() {
            eprintln!("Consensus error: {:?}", ce);
        } else {
            eprintln!("Unknown error: {}", e);
        }
    }
}
```

---

## Configuration

### Full Configuration Example

```toml
# servant-guild.toml

[guild]
name = "ServantGuild-Alpha"
version = "0.4.0"
admin_user = "your_telegram_id"

# Core Servants Configuration
[servants.coordinator]
enabled = true
max_concurrent_tasks = 10

[servants.worker]
enabled = true
build_timeout_secs = 300

[servants.warden]
enabled = true
audit_all_modifications = true

[servants.speaker]
enabled = true
channels = ["telegram", "slack"]

[servants.contractor]
enabled = true
github_integration = true

# Consensus Configuration
[consensus]
core_servants_count = 5
normal_quorum = 3    # 普通决策: 3/5 通过
critical_quorum = 5  # 关键决策: 5/5 全票
voting_timeout_secs = 3600
owner_veto_enabled = true

# Evolution Configuration
[evolution]
auto_evolve = false
max_concurrent = 5
require_human_approval = true
high_risk_threshold = 70
enable_canary = true

# Budget Configuration
[budget]
daily_limit_tokens = 1000000
warning_threshold = 0.8  # 80% used
critical_threshold = 0.95

[budget.servants.coordinator]
daily_limit = 200000

[budget.servants.worker]
daily_limit = 300000

[budget.servants.warden]
daily_limit = 150000

[budget.servants.speaker]
daily_limit = 50000

[budget.servants.contractor]
daily_limit = 300000

# Security Configuration
[security]
encryption_algorithm = "chacha20-poly1305"
audit_retention_days = 90
secret_rotation_interval_days = 30

# Infrastructure
[infrastructure]
runtime = "docker"
workspace_root = "/var/lib/servant-guild/workspace"
db_url = "postgres://..."
redis_url = "redis://..."

# Red Phone (Alert Channels)
[channels.telegram]
bot_token = "${TELEGRAM_BOT_TOKEN}"
allowed_users = ["${ADMIN_TELEGRAM_ID}"]

[channels.slack]
webhook_url = "${SLACK_WEBHOOK_URL}"
```

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 0.4.0 | 2025-01-16 | Phase 4: Autonomy - Full production-ready API |
| 0.3.0 | 2025-01-16 | Phase 3: Orchestration - Evolution APIs |
| 0.2.0 | 2025-01-15 | Phase 2: Cognition - Consensus APIs |
| 0.1.0 | 2025-01-14 | Phase 1: Core - Foundation APIs |
