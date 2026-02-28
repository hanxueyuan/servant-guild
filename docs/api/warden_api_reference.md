# Warden API Reference

## Overview

The Warden is the "guardian" of the guild, responsible for:
- Auditing all operations for safety
- Enforcing security policies
- Managing snapshots and rollback points
- Validating tool execution requests
- Monitoring for suspicious activity

The Warden implements the **Prudent Agency** framework, ensuring all operations are safe, audited, and reversible.

## Table of Contents

- [Core Types](#core-types)
- [Initialization](#initialization)
- [Security Checks](#security-checks)
- [Policy Management](#policy-management)
- [Audit and Logging](#audit-and-logging)
- [Snapshot and Rollback](#snapshot-and-rollback)
- [Consensus Integration](#consensus-integration)
- [Event Monitoring](#event-monitoring)
- [Error Handling](#error-handling)
- [Usage Examples](#usage-examples)

---

## Core Types

### Warden

The main Warden servant structure.

```rust
pub struct Warden {
    id: ServantId,
    status: RwLock<ServantStatus>,
    consensus: Option<Arc<ConsensusEngine>>,
    policy: RwLock<SecurityPolicy>,
    audit_log: Option<Arc<AuditLog>>,
    rollback_manager: Option<Arc<RollbackManager>>,
    events: RwLock<Vec<SecurityEvent>>,
    rate_tracker: RwLock<HashMap<String, Vec<DateTime<Utc>>>>,
}
```

**Fields:**
- `id`: Unique identifier for the warden
- `status`: Current operational status
- `consensus`: Optional reference to consensus engine
- `policy`: Security policy configuration
- `audit_log`: Reference to audit logger
- `rollback_manager`: Reference to rollback manager
- `events`: Log of security events
- `rate_tracker`: Operation rate limiter

### SecurityPolicy

Security policy configuration.

```rust
pub struct SecurityPolicy {
    pub max_auto_risk_level: u8,
    pub require_snapshots: bool,
    pub enforce_audit: bool,
    pub rate_limit: u32,
    pub block_network: bool,
    pub allowed_domains: Vec<String>,
    pub blocked_patterns: Vec<String>,
}
```

**Fields:**
- `max_auto_risk_level`: Maximum auto-approved risk level (1-10)
- `require_snapshots`: Require snapshots before changes
- `enforce_audit`: Enforce audit logging
- `rate_limit`: Max operations per minute
- `block_network`: Block external network access
- `allowed_domains`: Whitelist of allowed domains
- `blocked_patterns`: Blocked file patterns (e.g., `.env`)

### SecurityCheckResult

Result of a security check.

```rust
pub struct SecurityCheckResult {
    pub allowed: bool,
    pub reason: String,
    pub risk_level: u8,
    pub requires_approval: bool,
    pub warnings: Vec<String>,
}
```

**Fields:**
- `allowed`: Whether operation is allowed
- `reason`: Explanation of decision
- `risk_level`: Risk level (1-10, 10 = highest)
- `requires_approval`: Whether manual approval needed
- `warnings`: List of warning messages

### SecurityEvent

Record of a security event.

```rust
pub struct SecurityEvent {
    pub id: String,
    pub event_type: SecurityEventType,
    pub description: String,
    pub source: String,
    pub risk_level: u8,
    pub timestamp: DateTime<Utc>,
    pub blocked: bool,
}
```

**Fields:**
- `id`: Unique event identifier
- `event_type`: Type of security event
- `description`: Event description
- `source`: Servant that triggered the event
- `risk_level`: Risk level at time of event
- `timestamp`: When the event occurred
- `blocked`: Whether the event was blocked

### SecurityEventType

Types of security events.

```rust
pub enum SecurityEventType {
    ToolExecution,
    FileAccess,
    NetworkRequest,
    PolicyViolation,
    SuspiciousActivity,
    RateLimitExceeded,
    SecurityScan,
}
```

---

## Initialization

### new()

Creates a new Warden instance with default security policy.

```rust
pub fn new() -> Self
```

**Returns:** A new Warden instance

**Example:**
```rust
let warden = Warden::new();
```

### with_consensus()

Sets the consensus engine for the warden.

```rust
pub fn with_consensus(mut self, consensus: Arc<ConsensusEngine>) -> Self
```

**Parameters:**
- `consensus`: Shared reference to consensus engine

**Returns:** Self for builder pattern chaining

**Example:**
```rust
let warden = Warden::new()
    .with_consensus(consensus_engine);
```

### with_audit_log()

Sets the audit logger for the warden.

```rust
pub fn with_audit_log(mut self, audit_log: Arc<AuditLog>) -> Self
```

**Parameters:**
- `audit_log`: Shared reference to audit logger

**Returns:** Self for builder pattern chaining

**Example:**
```rust
let warden = Warden::new()
    .with_audit_log(audit_logger);
```

### with_rollback_manager()

Sets the rollback manager for the warden.

```rust
pub fn with_rollback_manager(mut self, manager: Arc<RollbackManager>) -> Self
```

**Parameters:**
- `manager`: Shared reference to rollback manager

**Returns:** Self for builder pattern chaining

**Example:**
```rust
let warden = Warden::new()
    .with_rollback_manager(rollback_manager);
```

---

## Security Checks

### check_operation()

Checks if an operation is allowed based on security policy.

```rust
pub fn check_operation(
    &self,
    operation_type: &str,
    params: &serde_json::Value,
    source: &str
) -> SecurityCheckResult
```

**Parameters:**
- `operation_type`: Type of operation (e.g., "write_file", "run_command")
- `params`: Operation parameters
- `source`: Servant requesting the operation

**Returns:** `SecurityCheckResult` with decision and details

**Check Logic:**
1. Calculates risk level based on operation type
2. Checks against security policy
3. Verifies file patterns (if applicable)
4. Checks rate limits
5. Returns decision with reason

**Example:**
```rust
let params = serde_json::json!({
    "path": "/workspace/config.json",
    "content": "{}"
});

let result = warden.check_operation("write_file", &params, "worker");
if !result.allowed {
    println!("Operation blocked: {}", result.reason);
}
```

### calculate_risk_level()

Calculates the risk level of an operation.

```rust
fn calculate_risk_level(operation_type: &str, params: &serde_json::Value) -> u8
```

**Parameters:**
- `operation_type`: Type of operation
- `params`: Operation parameters

**Returns:** Risk level (1-10)

**Risk Levels:**
- `1-3`: Low risk (read operations, safe file writes)
- `4-6`: Medium risk (file modifications, network requests)
- `7-10`: High risk (system changes, critical file writes)

---

## Policy Management

### set_policy()

Sets a new security policy.

```rust
pub fn set_policy(&self, policy: SecurityPolicy)
```

**Parameters:**
- `policy`: New security policy

**Example:**
```rust
let policy = SecurityPolicy {
    max_auto_risk_level: 3,  // More restrictive
    require_snapshots: true,
    enforce_audit: true,
    rate_limit: 30,
    block_network: true,
    allowed_domains: vec![],
    blocked_patterns: vec!["**/.env".to_string()],
};

warden.set_policy(policy);
```

### get_policy()

Gets the current security policy.

```rust
pub fn get_policy(&self) -> SecurityPolicy
```

**Returns:** Current security policy

**Example:**
```rust
let policy = warden.get_policy();
println!("Max auto risk level: {}", policy.max_auto_risk_level);
```

---

## Audit and Logging

### audit_operation()

Audits an operation for security purposes.

```rust
pub async fn audit_operation(
    &self,
    operation_type: &str,
    params: &serde_json::Value,
    source: &str
) -> Result<(), ServantError>
```

**Parameters:**
- `operation_type`: Type of operation
- `params`: Operation parameters
- `source`: Servant requesting the operation

**Returns:**
- `Ok(())`: Audit successful
- `Err(ServantError)`: Audit failed

**Audit Process:**
1. Creates structured audit record
2. Calculates risk level
3. Logs to audit system
4. Records security event
5. Creates snapshot if required

**Example:**
```rust
warden.audit_operation(
    "write_file",
    &serde_json::json!({"path": "config.json"}),
    "worker"
).await?;
```

---

## Snapshot and Rollback

### create_snapshot()

Creates a snapshot of the current state.

```rust
pub async fn create_snapshot(&self, operation_id: &str) -> Result<String, ServantError>
```

**Parameters:**
- `operation_id`: ID of the operation being snapshot

**Returns:**
- `Ok(String)`: Snapshot ID
- `Err(ServantError)`: Snapshot creation failed

**Example:**
```rust
let snapshot_id = warden.create_snapshot("op-123").await?;
println!("Created snapshot: {}", snapshot_id);
```

### rollback()

Rolls back to a previous snapshot.

```rust
pub async fn rollback(&self, snapshot_id: &str) -> Result<(), ServantError>
```

**Parameters:**
- `snapshot_id`: ID of snapshot to restore

**Returns:**
- `Ok(())`: Rollback successful
- `Err(ServantError)`: Rollback failed

**Example:**
```rust
warden.rollback("snapshot-456").await?;
```

---

## Consensus Integration

### vote_on_proposal()

Votes on a proposal from the consensus engine.

```rust
pub async fn vote_on_proposal(
    &self,
    proposal_id: &str,
    approve: bool
) -> Result<(), ServantError>
```

**Parameters:**
- `proposal_id`: ID of the proposal
- `approve`: Whether to approve the proposal

**Returns:**
- `Ok(())`: Vote recorded
- `Err(ServantError)`: Vote failed

**Voting Logic:**
- Warden votes YES if risk level is acceptable
- Warden votes NO if operation violates policy
- Warden may abstain on ambiguous cases

**Example:**
```rust
warden.vote_on_proposal("proposal-789", true).await?;
```

---

## Event Monitoring

### get_events()

Gets security event history.

```rust
pub fn get_events(&self) -> Vec<SecurityEvent>
```

**Returns:** List of security events

**Example:**
```rust
let events = warden.get_events();
for event in events {
    println!("{}: {} (Risk: {}, Blocked: {})",
        event.timestamp, event.event_type, event.risk_level, event.blocked);
}
```

### check_rate_limit()

Checks if a source has exceeded its rate limit.

```rust
fn check_rate_limit(&self, source: &str, limit: u32) -> bool
```

**Parameters:**
- `source`: Servant identifier
- `limit`: Maximum operations per minute

**Returns:** `true` if within limit, `false` if exceeded

**Example:**
```rust
if !warden.check_rate_limit("worker", 60) {
    println!("Worker has exceeded rate limit");
}
```

---

## Error Handling

### ServantError

Error types for warden operations.

```rust
pub enum ServantError {
    AuditFailed(String),
    SnapshotFailed(String),
    RollbackFailed(String),
    PolicyViolation(String),
    RiskLevelTooHigh(String),
    RateLimitExceeded(String),
}
```

**Error Handling Example:**
```rust
match warden.audit_operation(op_type, params, source).await {
    Ok(()) => println!("Audit successful"),
    Err(ServantError::PolicyViolation(msg)) => {
        eprintln!("Policy violation: {}", msg);
    },
    Err(ServantError::RateLimitExceeded(msg)) => {
        eprintln!("Rate limit exceeded: {}", msg);
    },
    Err(e) => {
        eprintln!("Warden error: {:?}", e);
    }
}
```

---

## Usage Examples

### Example 1: Basic Security Check

```rust
use servant_guild::servants::Warden;
use serde_json::json;

fn main() {
    let warden = Warden::new();

    // Check a file write operation
    let params = json!({
        "path": "/workspace/config.json",
        "content": "{}"
    });

    let result = warden.check_operation("write_file", &params, "worker");
    if result.allowed {
        println!("Operation allowed (Risk: {})", result.risk_level);
    } else {
        println!("Operation blocked: {}", result.reason);
    }
}
```

### Example 2: With Audit Logging

```rust
use servant_guild::servants::Warden;
use servant_guild::safety::AuditLogger;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let audit_log = AuditLogger::new();
    let warden = Warden::new()
        .with_audit_log(Arc::new(audit_log));

    // Audit an operation
    warden.audit_operation(
        "write_file",
        &serde_json::json!({"path": "config.json"}),
        "worker"
    ).await?;

    println!("Operation audited successfully");
    Ok(())
}
```

### Example 3: Snapshot and Rollback

```rust
use servant_guild::servants::Warden;
use servant_guild::safety::TransactionManager;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rollback_manager = TransactionManager::new();
    let warden = Warden::new()
        .with_rollback_manager(Arc::new(rollback_manager));

    // Create snapshot before risky operation
    let snapshot_id = warden.create_snapshot("op-123").await?;
    println!("Created snapshot: {}", snapshot_id);

    // Perform risky operation here...

    // If something goes wrong, rollback
    warden.rollback(&snapshot_id).await?;
    println!("Rolled back to snapshot");

    Ok(())
}
```

### Example 4: Custom Security Policy

```rust
use servant_guild::servants::{Warden, SecurityPolicy};

fn main() {
    let warden = Warden::new();

    // Create strict policy
    let policy = SecurityPolicy {
        max_auto_risk_level: 2,  // Only allow low-risk operations
        require_snapshots: true,
        enforce_audit: true,
        rate_limit: 10,  // Very restrictive
        block_network: true,
        allowed_domains: vec![],
        blocked_patterns: vec![
            "**/.env".to_string(),
            "**/secrets.*".to_string(),
            "**/credentials.*".to_string(),
            "**/config/**/*.json".to_string(),
        ],
    };

    warden.set_policy(policy);
    println!("Strict policy applied");
}
```

### Example 5: Event Monitoring

```rust
use servant_guild::servants::Warden;

fn main() {
    let warden = Warden::new();

    // Perform some operations...
    warden.check_operation("write_file", &serde_json::json!({}), "worker");
    warden.check_operation("run_command", &serde_json::json!({}), "worker");

    // Check event history
    let events = warden.get_events();
    println!("Total events: {}", events.len());

    let blocked_count = events.iter().filter(|e| e.blocked).count();
    println!("Blocked operations: {}", blocked_count);

    let avg_risk: f32 = events.iter()
        .map(|e| e.risk_level as f32)
        .sum::<f32>() / events.len() as f32;
    println!("Average risk level: {:.2}", avg_risk);
}
```

---

## Best Practices

### 1. Always Audit Before Operations
```rust
// Prudent Agency: Audit First
warden.audit_operation(op_type, params, source).await?;
// Then perform operation
```

### 2. Create Snapshots for High-Risk Operations
```rust
if result.risk_level > 7 {
    let snapshot_id = warden.create_snapshot(op_id).await?;
    // Perform operation with rollback capability
}
```

### 3. Monitor Security Events Regularly
```rust
let events = warden.get_events();
for event in events.iter().filter(|e| e.blocked) {
    // Investigate blocked operations
}
```

### 4. Use Appropriate Risk Levels
```rust
// Understand risk levels before calling operations
let result = warden.check_operation("write_file", params, "worker");
if result.risk_level > 5 {
    // Require additional approval
}
```

### 5. Set Realistic Rate Limits
```rust
let policy = SecurityPolicy {
    rate_limit: 60,  // Reasonable for normal operations
    // ...
};
```

---

## Security Considerations

### Default Deny
- All operations are blocked unless explicitly allowed
- Use `allowed_domains` whitelist for network access
- Use `blocked_patterns` to protect sensitive files

### Least Privilege
- Set `max_auto_risk_level` to lowest acceptable value
- Require snapshots for all state changes
- Enforce audit logging for all operations

### Defense in Depth
- Combine multiple security measures
- Use both policy checks and runtime monitoring
- Maintain audit logs for forensic analysis

---

## Performance Considerations

- **Audit Overhead**: Audit operations add ~1-5ms latency
- **Snapshot Cost**: Snapshots can be expensive; use selectively
- **Rate Limiting**: Use efficient data structures for rate tracking
- **Event Storage**: Implement cleanup for old events

---

## Limitations

- Risk level calculation is heuristic-based (not AI-powered)
- Rate limiting is in-memory only (not persistent)
- Snapshot scope is limited to file system (not full system state)
- Policy enforcement is synchronous (may block operations)

---

## Future Enhancements

- **AI-Powered Risk Assessment**: Use ML for better risk prediction
- **Behavioral Analysis**: Detect anomalous patterns
- **Multi-tenancy**: Isolate security policies per tenant
- **External Integration**: SIEM and alerting system integration
- **Policy Versioning**: Track and revert policy changes
- **Real-time Monitoring**: Webhook-based event notifications

---

## See Also

- [Prudent Agency Framework](../../design/prudent_agency_whitepaper.md)
- [Safety Module](../../safety/README.md)
- [Consensus Engine](../../consensus/README.md)
- [Architecture Overview](../../architecture/servant_guild_architecture_v1.0.md)
