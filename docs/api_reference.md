# ServantGuild API Reference

**Version**: 0.3.0  
**Last Updated**: 2025-01-16

## Table of Contents

1. [Core Types](#core-types)
2. [Runtime API](#runtime-api)
3. [Consensus API](#consensus-api)
4. [Safety API](#safety-api)
5. [Servant API](#servant-api)
6. [Error Handling](#error-handling)
7. [Configuration](#configuration)

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
    Coordinator,
    Worker,
    Warden,
    Speaker,
    Contractor,
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
}

pub enum TaskType {
    Build,
    Test,
    Deploy,
    Analyze,
    Report,
    Evolve,
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

## Runtime API

### BuildSandbox

Secure isolated build environment.

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

**Example**:
```rust
let config = SandboxConfig::default()
    .with_memory_limit(4096)
    .with_time_limits(300, 600)
    .with_network(false);

let sandbox = BuildSandbox::new("build-123", config).await?;
sandbox.copy_project(&project_path).await?;

let result = sandbox.execute("cargo", &["build", "--release"], None).await?;

if result.success {
    println!("Build succeeded in {}ms", result.duration_ms);
}
```

### SandboxConfig

```rust
impl SandboxConfig {
    /// Create with sandbox root directory
    pub fn new(sandbox_root: PathBuf) -> Self;
    
    /// Set memory limit (MB)
    pub fn with_memory_limit(self, mb: u64) -> Self;
    
    /// Set time limits (CPU seconds, wall seconds)
    pub fn with_time_limits(self, cpu_secs: u64, wall_secs: u64) -> Self;
    
    /// Enable/disable network access
    pub fn with_network(self, allowed: bool) -> Self;
    
    /// Use container isolation
    pub fn with_container(self, image: String) -> Self;
    
    /// Add allowed domain for network
    pub fn with_allowed_domain(self, domain: String) -> Self;
    
    /// Set environment variable
    pub fn with_env(self, key: String, value: String) -> Self;
}
```

### ErrorAnalyzer

Intelligent error analysis and fix suggestion.

```rust
impl ErrorAnalyzer {
    /// Create a new error analyzer
    pub fn new() -> Self;
    
    /// Create with LLM support
    pub fn with_llm(llm: Option<Arc<dyn LLMProvider>>) -> Self;
    
    /// Analyze build output and extract errors
    pub async fn analyze(&self, output: &str) -> Result<Vec<BuildError>>;
    
    /// Generate fix suggestions for errors
    pub async fn suggest_fixes(
        &self,
        errors: &[BuildError],
        context: &BuildContext,
    ) -> Result<Vec<FixSuggestion>>;
    
    /// Apply a fix suggestion
    pub async fn apply_fix(
        &self,
        suggestion: &FixSuggestion,
        project_path: &Path,
    ) -> Result<FixResult>;
    
    /// Get success rate for a category
    pub async fn get_success_rate(&self, category: FixCategory) -> f64;
}
```

**Example**:
```rust
let analyzer = ErrorAnalyzer::new();
let errors = analyzer.analyze(&build_output).await?;

for error in &errors {
    println!("[{}] {}:{}", error.error_code, error.file, error.line);
}

let context = BuildContext {
    available_modules: vec!["crate::module".to_string()],
    ..Default::default()
};

let fixes = analyzer.suggest_fixes(&errors, &context).await?;
for fix in &fixes {
    println!("Fix: {} (confidence: {}%)", fix.description, fix.confidence);
}
```

### BuildPipeline

Multi-stage build pipeline.

```rust
impl BuildPipeline {
    /// Create a new build pipeline
    pub fn new(config: PipelineConfig) -> Result<Self>;
    
    /// Create with LLM support
    pub fn with_llm(config: PipelineConfig, llm: Arc<dyn LLMProvider>) -> Result<Self>;
    
    /// Run the full pipeline
    pub async fn run(&self, proposal: Option<&Proposal>) -> Result<PipelineResult>;
    
    /// Get pipeline result by ID
    pub async fn get_result(&self, id: &str) -> Option<PipelineResult>;
    
    /// List all pipeline runs
    pub async fn list_runs(&self) -> Vec<(String, bool, u64)>;
}
```

**Pipeline Stages**:
```rust
pub enum PipelineStage {
    Prepare,   // Validate inputs, create workspace
    Fetch,     // Download dependencies
    Build,     // Compile the project
    Test,      // Run test suite
    Package,   // Create deployment artifact
    Deploy,    // Deploy to target environment
}
```

**Example**:
```rust
let config = PipelineConfig {
    project_root: PathBuf::from("./my-project"),
    build_target: BuildTarget::Release,
    auto_fix: true,
    ..Default::default()
};

let pipeline = BuildPipeline::new(config)?;
let result = pipeline.run(None).await?;

println!("Pipeline {} finished: {}", result.id, result.success);
for (stage, stage_result) in &result.stages {
    println!("  {:?}: {}ms", stage, stage_result.duration_ms);
}
```

### StateMigrator

State migration for hot-swap.

```rust
impl StateMigrator {
    /// Create a new state migrator
    pub fn new() -> Self;
    
    /// Register a schema
    pub async fn register_schema(&self, version: &str, schema: StateSchema);
    
    /// Register a transform function
    pub async fn register_transform(&self, name: &str, transform: TransformFn);
    
    /// Create a state snapshot
    pub async fn create_snapshot(
        &self,
        module_id: &str,
        module_version: &str,
        schema_version: &str,
        data: serde_json::Value,
    ) -> Result<StateSnapshot>;
    
    /// Plan a migration
    pub async fn plan_migration(
        &self,
        from_version: &str,
        to_version: &str,
        source_schema: StateSchema,
        target_schema: StateSchema,
    ) -> Result<MigrationPlan>;
    
    /// Execute migration
    pub async fn migrate(
        &self,
        snapshot: &StateSnapshot,
        plan: &MigrationPlan,
    ) -> Result<MigrationResult>;
    
    /// Get migration statistics
    pub async fn get_stats(&self) -> MigrationStats;
}
```

**Example**:
```rust
let migrator = StateMigrator::new();

// Create snapshot
let data = serde_json::json!({ "counter": 42, "name": "test" });
let snapshot = migrator.create_snapshot("module-1", "1.0.0", "1.0.0", data).await?;

// Plan migration
let plan = migrator.plan_migration("1.0.0", "1.1.0", source_schema, target_schema).await?;

// Execute migration
let result = migrator.migrate(&snapshot, &plan).await?;
println!("Migrated {} fields", result.fields_migrated);
```

### EvolutionWorkflow

Complete self-evolution pipeline.

```rust
impl EvolutionWorkflow {
    /// Create a new evolution workflow manager
    pub fn new(
        consensus: Arc<ConsensusEngine>,
        pipeline: Arc<BuildPipeline>,
        canary: Arc<CanaryTester>,
        config: WorkflowConfig,
    ) -> Self;
    
    /// Add LLM provider
    pub fn with_llm(self, llm: Arc<dyn LLMProvider>) -> Self;
    
    /// Start a new evolution workflow
    pub async fn start(&self, trigger: EvolutionTrigger) -> Result<String>;
    
    /// Approve workflow (for human approval)
    pub async fn approve(&self, workflow_id: &str, approved: bool) -> Result<()>;
    
    /// Get workflow state
    pub async fn get_state(&self, workflow_id: &str) -> Option<WorkflowState>;
    
    /// Get workflow history
    pub async fn get_history(&self) -> Vec<WorkflowRecord>;
    
    /// Get workflow statistics
    pub async fn get_stats(&self) -> WorkflowStats;
}
```

**Example**:
```rust
let workflow = EvolutionWorkflow::new(
    consensus,
    pipeline,
    canary,
    WorkflowConfig::default(),
);

// Start evolution
let id = workflow.start(EvolutionTrigger::PerformanceDegradation {
    metric: "latency".to_string(),
    current_value: 500.0,
    threshold: 200.0,
}).await?;

// Monitor progress
let state = workflow.get_state(&id).await.unwrap();
println!("Stage: {:?}", state.stage);

// Approve if needed
if state.human_approval_required {
    workflow.approve(&id, true).await?;
}
```

---

## Consensus API

### ConsensusEngine

```rust
impl ConsensusEngine {
    /// Create a new consensus engine
    pub fn new() -> Self;
    
    /// Create with custom configuration
    pub fn with_config(config: ConsensusConfig, constitution: Constitution) -> Self;
    
    /// Register a servant (gives voting rights)
    pub fn register_servant(&self, servant_id: String);
    
    /// Unregister a servant
    pub fn unregister_servant(&self, servant_id: &str);
    
    /// Create a new proposal
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
```

### UpdateProposal

```rust
impl UpdateProposal {
    /// Create a new update proposal
    pub fn new(
        title: String,
        description: String,
        proposer: String,
        update_type: UpdateType,
    ) -> Self;
    
    /// Add rationale
    pub fn with_rationale(self, rationale: String) -> Self;
    
    /// Add benefits
    pub fn with_benefits(self, benefits: Vec<String>) -> Self;
    
    /// Add risks
    pub fn with_risks(self, risks: Vec<String>) -> Self;
    
    /// Add rollback plan
    pub fn with_rollback_plan(self, plan: RollbackPlan) -> Self;
    
    /// Add test results
    pub fn with_test_results(self, results: TestResults) -> Self;
    
    /// Set confidence (0-100)
    pub fn with_confidence(self, confidence: u8) -> Self;
    
    /// Calculate risk score (0-100)
    pub fn calculate_risk_score(&self) -> u8;
    
    /// Check if safe to execute
    pub fn is_safe_to_execute(&self) -> bool;
    
    /// Convert to consensus proposal
    pub fn into_proposal(self) -> Proposal;
}
```

**Example**:
```rust
let proposal = UpdateProposalBuilder::new(
    "Update Coordinator Module".to_string(),
    "Performance improvements".to_string(),
    "warden-1".to_string(),
    UpdateType::ModuleUpdate {
        module_id: "coordinator".to_string(),
        from_version: "1.0.0".to_string(),
        to_version: "1.1.0".to_string(),
    },
)
.rationale("Improve scheduling efficiency".to_string())
.benefit("20% latency reduction".to_string())
.risk("Potential scheduling bugs".to_string())
.confidence(85)
.build();

if proposal.is_safe_to_execute() {
    let consensus_proposal = proposal.into_proposal();
    // Submit to consensus engine
}
```

---

## Safety API

### CanaryTester

```rust
impl CanaryTester {
    /// Create with configuration
    pub fn new(config: CanaryConfig, metrics_collector: Arc<dyn MetricsCollector>) -> Self;
    
    /// Create with defaults
    pub fn with_defaults(metrics_collector: Arc<dyn MetricsCollector>) -> Self;
    
    /// Start a canary test
    pub async fn start_test(&self, module_id: &str, new_version: &str) -> Result<String>;
    
    /// Monitor active test
    pub async fn monitor(&self, test_id: &str) -> Result<CanaryStatus>;
    
    /// Advance to next step
    pub async fn advance(&self, test_id: &str) -> Result<CanaryStatus>;
    
    /// Pause a test
    pub async fn pause_test(&self, test_id: &str) -> Result<()>;
    
    /// Abort a test
    pub async fn abort_test(&self, test_id: &str) -> Result<()>;
    
    /// Get test status
    pub async fn get_status(&self, test_id: &str) -> Option<CanaryStatus>;
    
    /// Calculate health score
    pub fn calculate_health_score(&self, status: &CanaryStatus) -> f64;
}
```

### RecoveryManager

```rust
impl RecoveryManager {
    /// Create a new recovery manager
    pub fn new(
        migrator: Arc<StateMigrator>,
        snapshots_dir: PathBuf,
        config: RecoveryConfig,
    ) -> Result<Self>;
    
    /// Start recovery from snapshot
    pub async fn recover(&self, snapshot_id: &str) -> Result<String>;
    
    /// Get recovery status
    pub async fn get_status(&self, recovery_id: &str) -> Option<RecoveryStatus>;
    
    /// Cancel an active recovery
    pub async fn cancel_recovery(&self, recovery_id: &str) -> Result<()>;
    
    /// Get recovery history
    pub async fn get_history(&self) -> Vec<RecoveryRecord>;
    
    /// Get recovery statistics
    pub async fn get_stats(&self) -> RecoveryStats;
}
```

---

## Error Handling

All APIs return `Result<T, anyhow::Error>` for comprehensive error handling.

**Common Error Types**:
- `SandboxError`: Sandbox creation or execution failures
- `BuildError`: Build compilation failures
- `MigrationError`: State migration failures
- `ConsensusError`: Consensus decision failures
- `RecoveryError`: Recovery operation failures

**Example**:
```rust
match sandbox.execute("cargo", &["build"], None).await {
    Ok(result) => {
        if result.timed_out {
            eprintln!("Build timed out");
        } else if !result.success {
            eprintln!("Build failed: {}", result.stderr);
        }
    }
    Err(e) => {
        eprintln!("Execution error: {}", e);
    }
}
```

---

## Configuration

### Full Configuration Example

```toml
# servant-guild.toml

[project]
name = "my-servant-guild"
version = "0.3.0"

[consensus]
core_servants_count = 5
normal_quorum = 3
critical_quorum = 5
voting_timeout_secs = 3600
owner_veto_enabled = true

[sandbox]
max_memory_mb = 2048
max_cpu_time_secs = 600
max_wall_time_secs = 900
network_allowed = true
use_container = false

[canary]
initial_percentage = 5.0
increment_percentage = 10.0
step_duration_secs = 300
auto_rollback = true

[canary.thresholds.error_rate]
warning = 0.05
critical = 0.10

[canary.thresholds.latency_p99]
warning = 100.0
critical = 200.0

[evolution]
auto_evolve = false
max_concurrent = 5
require_human_approval = true
high_risk_threshold = 70
enable_canary = true
auto_rollback = true
learning_mode = false

[pipeline]
build_target = "release"
auto_fix = true
max_auto_fix_attempts = 3
incremental = true
timeout_secs = 600

[recovery]
max_concurrent = 3
max_retries = 3
backoff_base_ms = 1000
verify_after = true
auto_rollback = true
```

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 0.3.0 | 2025-01-16 | Phase 3: Orchestration complete |
| 0.2.0 | 2025-01-15 | Phase 2: Assembly complete |
| 0.1.0 | 2025-01-14 | Phase 1: Foundation complete |
