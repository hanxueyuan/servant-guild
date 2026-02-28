# Contractor API Reference

## Overview

The Contractor is the "builder" of the guild, responsible for:
- Managing resources and configurations
- Handling deployments and scaling
- Managing external service integrations
- Maintaining system health
- Environment and secrets management

The Contractor provides complete lifecycle management for resources (Create, Start, Stop, Destroy) and tracks usage statistics for all managed resources.

## Table of Contents

- [Core Types](#core-types)
- [Initialization](#initialization)
- [Resource Management](#resource-management)
- [Configuration Management](#configuration-management)
- [Lifecycle Management](#lifecycle-management)
- [Health Monitoring](#health-monitoring)
- [Usage Tracking](#usage-tracking)
- [Consensus Integration](#consensus-integration)
- [Error Handling](#error-handling)
- [Usage Examples](#usage-examples)

---

## Core Types

### Contractor

The main Contractor servant structure.

```rust
pub struct Contractor {
    id: ServantId,
    status: RwLock<ServantStatus>,
    consensus: Option<Arc<ConsensusEngine>>,
    resources: RwLock<HashMap<String, Resource>>,
    configs: RwLock<HashMap<String, ConfigEntry>>,
    lifecycle_events: RwLock<Vec<LifecycleEvent>>,
    usage_stats: RwLock<HashMap<String, UsageStatistics>>,
}
```

**Fields:**
- `id`: Unique identifier for the contractor
- `status`: Current operational status
- `consensus`: Optional reference to consensus engine
- `resources`: Managed resources map
- `configs`: Configuration entries
- `lifecycle_events`: History of lifecycle events
- `usage_stats`: Resource usage statistics

### Resource

A resource managed by the Contractor.

```rust
pub struct Resource {
    pub id: String,
    pub name: String,
    pub resource_type: ResourceType,
    pub status: ResourceStatus,
    pub config: serde_json::Value,
    pub health: HealthStatus,
    pub last_check: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
}
```

**Fields:**
- `id`: Unique resource identifier
- `name`: Resource name
- `resource_type`: Type of resource
- `status`: Current status
- `config`: Resource configuration
- `health`: Health status information
- `last_check`: When health was last checked
- `tags`: Resource tags for categorization

### ResourceType

Types of resources managed by Contractor.

```rust
pub enum ResourceType {
    Database,
    Cache,
    MessageQueue,
    ExternalAPI,
    Storage,
    Compute,
    ConfigStore,
    Custom(String),
}
```

### ResourceStatus

Status of a resource.

```rust
pub enum ResourceStatus {
    Starting,
    Healthy,
    Degraded,
    Unhealthy,
    Stopped,
    Maintenance,
}
```

### HealthStatus

Health status of a resource.

```rust
pub struct HealthStatus {
    pub score: u8,
    pub responding: bool,
    pub last_error: Option<String>,
    pub metrics: HashMap<String, f64>,
}
```

**Fields:**
- `score`: Health score (0-100)
- `responding`: Whether responding to health checks
- `last_error`: Last error message (if any)
- `metrics`: Health metrics (e.g., CPU, memory, latency)

### ConfigEntry

Configuration entry.

```rust
pub struct ConfigEntry {
    pub key: String,
    pub value: serde_json::Value,
    pub is_secret: bool,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
    pub version: u32,
}
```

**Fields:**
- `key`: Configuration key
- `value`: Configuration value
- `is_secret`: Whether this is a secret (should be encrypted)
- `updated_at`: Last update timestamp
- `updated_by`: Who made the update
- `version`: Version number for change tracking

### LifecycleEvent

Lifecycle event for a resource.

```rust
pub struct LifecycleEvent {
    pub id: String,
    pub resource_id: String,
    pub event_type: LifecycleEventType,
    pub timestamp: DateTime<Utc>,
    pub triggered_by: String,
    pub details: Option<serde_json::Value>,
}
```

### LifecycleEventType

Types of lifecycle events.

```rust
pub enum LifecycleEventType {
    Created,
    Started,
    Stopped,
    Destroyed,
    Updated,
    HealthCheckFailed,
    MaintenanceStarted,
    MaintenanceEnded,
}
```

### UsageStatistics

Usage statistics for a resource.

```rust
pub struct UsageStatistics {
    pub resource_id: String,
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub total_runtime_ms: u64,
    pub average_latency_ms: f64,
    pub last_used: Option<DateTime<Utc>>,
}
```

---

## Initialization

### new()

Creates a new Contractor instance.

```rust
pub fn new() -> Self
```

**Returns:** A new Contractor instance

**Example:**
```rust
let contractor = Contractor::new();
```

### with_consensus()

Sets the consensus engine for the contractor.

```rust
pub fn with_consensus(mut self, consensus: Arc<ConsensusEngine>) -> Self
```

**Parameters:**
- `consensus`: Shared reference to consensus engine

**Returns:** Self for builder pattern chaining

**Example:**
```rust
let contractor = Contractor::new()
    .with_consensus(consensus_engine);
```

---

## Resource Management

### create_resource()

Creates a new resource.

```rust
pub fn create_resource(
    &self,
    name: String,
    resource_type: ResourceType,
    config: serde_json::Value
) -> Result<String, ServantError>
```

**Parameters:**
- `name`: Resource name
- `resource_type`: Type of resource
- `config`: Resource configuration

**Returns:**
- `Ok(String)`: Resource ID
- `Err(ServantError)`: Error if creation failed

**Example:**
```rust
let resource_id = contractor.create_resource(
    "primary-database".to_string(),
    ResourceType::Database,
    serde_json::json!({
        "host": "localhost",
        "port": 5432,
        "database": "guild_db"
    })
)?;
```

### get_resource()

Gets a resource by ID.

```rust
pub fn get_resource(&self, resource_id: &str) -> Option<Resource>
```

**Parameters:**
- `resource_id`: ID of the resource

**Returns:** `Option<Resource>` if found

**Example:**
```rust
if let Some(resource) = contractor.get_resource(&resource_id) {
    println!("Resource: {} - {:?}", resource.name, resource.status);
}
```

### list_resources()

Lists all resources, optionally filtered by type.

```rust
pub fn list_resources(&self, resource_type: Option<ResourceType>) -> Vec<Resource>
```

**Parameters:**
- `resource_type`: Optional resource type filter

**Returns:** List of resources

**Example:**
```rust
// List all resources
let all_resources = contractor.list_resources(None);

// List only databases
let databases = contractor.list_resources(Some(ResourceType::Database));
```

---

## Configuration Management

### set_config()

Sets a configuration value.

```rust
pub fn set_config(
    &self,
    key: String,
    value: serde_json::Value,
    is_secret: bool,
    updated_by: String
) -> Result<(), ServantError>
```

**Parameters:**
- `key`: Configuration key
- `value`: Configuration value
- `is_secret`: Whether this is a secret
- `updated_by`: Who is making the change

**Returns:**
- `Ok(())`: Configuration set successfully
- `Err(ServantError)`: Error if set failed

**Example:**
```rust
contractor.set_config(
    "database.host".to_string(),
    serde_json::json!("localhost"),
    false,
    "admin".to_string()
)?;
```

### get_config()

Gets a configuration value.

```rust
pub fn get_config(&self, key: &str) -> Option<ConfigEntry>
```

**Parameters:**
- `key`: Configuration key

**Returns:** `Option<ConfigEntry>` if found

**Example:**
```rust
if let Some(config) = contractor.get_config("database.host") {
    println!("Database host: {}", config.value);
}
```

### list_configs()

Lists all configuration entries, optionally filtering secrets.

```rust
pub fn list_configs(&self, include_secrets: bool) -> Vec<ConfigEntry>
```

**Parameters:**
- `include_secrets`: Whether to include secret values

**Returns:** List of configuration entries

**Example:**
```rust
// List all configs (secrets masked)
let configs = contractor.list_configs(false);

// List configs including secrets
let all_configs = contractor.list_configs(true);
```

---

## Lifecycle Management

### start_resource()

Starts a resource.

```rust
pub async fn start_resource(&self, resource_id: &str) -> Result<(), ServantError>
```

**Parameters:**
- `resource_id`: ID of the resource to start

**Returns:**
- `Ok(())`: Resource started successfully
- `Err(ServantError)`: Error if start failed

**Example:**
```rust
contractor.start_resource(&resource_id).await?;
```

### stop_resource()

Stops a resource.

```rust
pub async fn stop_resource(&self, resource_id: &str) -> Result<(), ServantError>
```

**Parameters:**
- `resource_id`: ID of the resource to stop

**Returns:**
- `Ok(())`: Resource stopped successfully
- `Err(ServantError)`: Error if stop failed

**Example:**
```rust
contractor.stop_resource(&resource_id).await?;
```

### destroy_resource()

Destroys a resource.

```rust
pub async fn destroy_resource(&self, resource_id: &str) -> Result<(), ServantError>
```

**Parameters:**
- `resource_id`: ID of the resource to destroy

**Returns:**
- `Ok(())`: Resource destroyed successfully
- `Err(ServantError)`: Error if destruction failed

**Example:**
```rust
contractor.destroy_resource(&resource_id).await?;
```

---

## Health Monitoring

### check_health()

Checks health of a resource.

```rust
pub async fn check_health(&self, resource_id: &str) -> Result<HealthStatus, ServantError>
```

**Parameters:**
- `resource_id`: ID of the resource to check

**Returns:**
- `Ok(HealthStatus)`: Health status
- `Err(ServantError)`: Error if check failed

**Example:**
```rust
let health = contractor.check_health(&resource_id).await?;
println!("Health score: {}", health.score);
println!("Responding: {}", health.responding);
```

### check_all_resources()

Checks health of all resources.

```rust
pub async fn check_all_resources(&self) -> HashMap<String, HealthStatus>
```

**Returns:** Map of resource IDs to health status

**Example:**
```rust
let health_map = contractor.check_all_resources().await;
for (resource_id, health) in health_map {
    println!("{}: {}", resource_id, health.score);
}
```

---

## Usage Tracking

### get_usage_stats()

Gets usage statistics for a resource.

```rust
pub fn get_usage_stats(&self, resource_id: &str) -> Option<UsageStatistics>
```

**Parameters:**
- `resource_id`: ID of the resource

**Returns:** `Option<UsageStatistics>` if found

**Example:**
```rust
if let Some(stats) = contractor.get_usage_stats(&resource_id) {
    println!("Total operations: {}", stats.total_operations);
    println!("Success rate: {:.2}%",
        stats.successful_operations as f64 / stats.total_operations as f64 * 100.0
    );
}
```

### record_usage()

Records a resource usage event.

```rust
pub fn record_usage(
    &self,
    resource_id: &str,
    success: bool,
    latency_ms: u64
)
```

**Parameters:**
- `resource_id`: ID of the resource
- `success`: Whether operation was successful
- `latency_ms`: Operation latency in milliseconds

**Example:**
```rust
contractor.record_usage(&resource_id, true, 45);
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
- Contractor votes YES if resource changes are safe
- Contractor votes NO if resource destruction would affect system stability
- Contractor may request additional information for complex changes

**Example:**
```rust
contractor.vote_on_proposal("proposal-789", true).await?;
```

---

## Error Handling

### ServantError

Error types for contractor operations.

```rust
pub enum ServantError {
    ResourceNotFound(String),
    ResourceAlreadyExists(String),
    ResourceCreationFailed(String),
    ResourceStartFailed(String),
    ResourceStopFailed(String),
    ResourceDestroyFailed(String),
    ConfigNotFound(String),
    InvalidConfig(String),
}
```

**Error Handling Example:**
```rust
match contractor.create_resource(name, resource_type, config) {
    Ok(resource_id) => println!("Resource created: {}", resource_id),
    Err(ServantError::ResourceAlreadyExists(msg)) => {
        eprintln!("Resource already exists: {}", msg);
    },
    Err(ServantError::ResourceCreationFailed(msg)) => {
        eprintln!("Failed to create resource: {}", msg);
    },
    Err(e) => {
        eprintln!("Contractor error: {:?}", e);
    }
}
```

---

## Usage Examples

### Example 1: Basic Resource Creation

```rust
use servant_guild::servants::{Contractor, ResourceType};
use serde_json::json;

fn main() {
    let contractor = Contractor::new();

    // Create a database resource
    let resource_id = contractor.create_resource(
        "primary-db".to_string(),
        ResourceType::Database,
        json!({
            "host": "localhost",
            "port": 5432,
            "database": "guild_db",
            "username": "admin"
        })
    ).expect("Failed to create resource");

    println!("Resource created: {}", resource_id);
}
```

### Example 2: Resource Lifecycle Management

```rust
use servant_guild::servants::Contractor;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let contractor = Contractor::new();

    let resource_id = contractor.create_resource(
        "cache-service".to_string(),
        ResourceType::Cache,
        serde_json::json!({"host": "localhost", "port": 6379})
    )?;

    // Start the resource
    contractor.start_resource(&resource_id).await?;
    println!("Resource started");

    // Check health
    let health = contractor.check_health(&resource_id).await?;
    println!("Health: {}", health.score);

    // Stop the resource
    contractor.stop_resource(&resource_id).await?;
    println!("Resource stopped");

    Ok(())
}
```

### Example 3: Configuration Management

```rust
use servant_guild::servants::Contractor;
use serde_json::json;

fn main() {
    let contractor = Contractor::new();

    // Set configuration
    contractor.set_config(
        "database.host".to_string(),
        json!("localhost"),
        false,
        "admin".to_string()
    ).expect("Failed to set config");

    contractor.set_config(
        "database.password".to_string(),
        json!("secret123"),
        true,  // This is a secret
        "admin".to_string()
    ).expect("Failed to set config");

    // Get configuration
    if let Some(config) = contractor.get_config("database.host") {
        println!("Database host: {}", config.value);
    }

    // List configs (secrets masked)
    let configs = contractor.list_configs(false);
    println!("Total configs: {}", configs.len());
}
```

### Example 4: Health Monitoring

```rust
use servant_guild::servants::Contractor;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let contractor = Contractor::new();

    // Check all resources
    let health_map = contractor.check_all_resources().await;
    for (resource_id, health) in health_map {
        println!("Resource {}: Score={}, Responding={}",
            resource_id, health.score, health.responding
        );

        if health.score < 70 {
            println!("  Warning: Low health score!");
            if let Some(error) = health.last_error {
                println!("  Last error: {}", error);
            }
        }
    }

    Ok(())
}
```

### Example 5: Usage Statistics

```rust
use servant_guild::servants::Contractor;

fn main() {
    let contractor = Contractor::new();

    // Simulate some usage
    let resource_id = "resource-123";
    contractor.record_usage(resource_id, true, 45);
    contractor.record_usage(resource_id, true, 52);
    contractor.record_usage(resource_id, false, 100);

    // Get statistics
    if let Some(stats) = contractor.get_usage_stats(resource_id) {
        println!("Total operations: {}", stats.total_operations);
        println!("Successful: {}", stats.successful_operations);
        println!("Failed: {}", stats.failed_operations);
        println!("Average latency: {:.2}ms", stats.average_latency_ms);
        println!("Success rate: {:.2}%",
            stats.successful_operations as f64 / stats.total_operations as f64 * 100.0
        );
    }
}
```

### Example 6: Resource Listing and Filtering

```rust
use servant_guild::servants::{Contractor, ResourceType};

fn main() {
    let contractor = Contractor::new();

    // List all resources
    let all_resources = contractor.list_resources(None);
    println!("Total resources: {}", all_resources.len());

    // List only databases
    let databases = contractor.list_resources(Some(ResourceType::Database));
    println!("Databases: {}", databases.len());

    for db in databases {
        println!("  - {} ({:?})", db.name, db.status);
    }
}
```

---

## Best Practices

### 1. Always Check Resource Status
```rust
let resource = contractor.get_resource(&resource_id);
if let Some(res) = resource {
    if res.status == ResourceStatus::Healthy {
        // Use resource
    } else {
        println!("Resource not healthy: {:?}", res.status);
    }
}
```

### 2. Use Lifecycle Events for Audit Trail
```rust
let events = contractor.get_lifecycle_events(&resource_id);
for event in events {
    println!("{}: {:?} by {} at {}",
        event.resource_id,
        event.event_type,
        event.triggered_by,
        event.timestamp
    );
}
```

### 3. Monitor Health Regularly
```rust
// Periodic health checks
tokio::spawn(async move {
    loop {
        let health_map = contractor.check_all_resources().await;
        // Alert on unhealthy resources
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
});
```

### 4. Track Usage for Optimization
```rust
// Analyze usage patterns
let stats = contractor.get_usage_stats(&resource_id);
if let Some(s) = stats {
    if s.average_latency_ms > 1000 {
        // Consider scaling or optimization
    }
}
```

### 5. Secure Secret Configuration
```rust
// Always mark secrets appropriately
contractor.set_config(
    "api.key".to_string(),
    json!("secret-key"),
    true,  // is_secret = true
    "admin".to_string()
)?;
```

---

## Security Considerations

### Secret Management
- Always mark sensitive configuration as secrets
- Use encryption for stored secrets
- Never log secret values
- Implement secret rotation policies

### Resource Access Control
- Validate resource ownership before operations
- Implement RBAC for resource access
- Audit all lifecycle operations
- Use consensus for destructive operations

### Configuration Validation
- Validate configuration values before setting
- Use schema validation for complex configs
- Implement configuration versioning
- Provide rollback capability

---

## Performance Considerations

- **Health Check Overhead**: Don't check health too frequently (recommend 30-60s interval)
- **Usage Tracking**: Batch usage updates if high traffic
- **Resource Listing**: Implement pagination for large resource sets
- **Configuration Caching**: Cache frequently accessed configurations

---

## Limitations

- Resources are in-memory only (not persistent across restarts)
- Health checks are synchronous (may block)
- No automatic scaling based on usage
- No resource dependencies management
- No resource discovery across distributed systems

---

## Future Enhancements

- **Persistent Storage**: Save resources to database
- **Auto-scaling**: Scale resources based on usage metrics
- **Resource Dependencies**: Manage dependencies between resources
- **Health Recovery**: Automatic recovery from degraded state
- **Resource Discovery**: Discover resources in distributed environment
- **Resource Templates**: Predefined resource configurations
- **Cost Tracking**: Track cloud resource costs
- **Multi-cloud Support**: Manage resources across cloud providers

---

## See Also

- [Resource Management Design](../../design/resource_management.md)
- [Consensus Engine](../../consensus/README.md)
- [Architecture Overview](../../architecture/servant_guild_architecture_v1.0.md)
- [Configuration Guide](../../guides/configuration.md)
