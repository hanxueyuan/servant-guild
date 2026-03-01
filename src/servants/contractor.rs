//! Contractor Servant - Resource Management and Configuration
//!
//! The Contractor is the "builder" of the guild, responsible for:
//! - Managing resources and configurations
//! - Handling deployments and scaling
//! - Managing external service integrations
//! - Maintaining system health
//! - Environment and secrets management

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use super::{
    Servant, ServantError, ServantId, ServantResult, ServantRole, ServantStatus, ServantTask,
};
use crate::consensus::{ConsensusEngine, DecisionType, Vote};

/// A resource managed by the Contractor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// Resource ID
    pub id: String,
    /// Resource name
    pub name: String,
    /// Resource type
    pub resource_type: ResourceType,
    /// Current status
    pub status: ResourceStatus,
    /// Configuration
    pub config: serde_json::Value,
    /// Health status
    pub health: HealthStatus,
    /// When last checked
    pub last_check: Option<DateTime<Utc>>,
    /// Tags
    pub tags: Vec<String>,
}

/// Types of resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceType {
    /// Database connection
    Database,
    /// Cache service
    Cache,
    /// Message queue
    MessageQueue,
    /// External API
    ExternalAPI,
    /// File storage
    Storage,
    /// Computation resource
    Compute,
    /// Configuration store
    ConfigStore,
    /// Custom resource
    Custom(String),
}

/// Status of a resource
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResourceStatus {
    /// Resource is starting up
    Starting,
    /// Resource is healthy and available
    Healthy,
    /// Resource is degraded but functional
    Degraded,
    /// Resource is unhealthy
    Unhealthy,
    /// Resource is stopped
    Stopped,
    /// Resource is in maintenance mode
    Maintenance,
}

/// Health status of a resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Overall health (0-100)
    pub score: u8,
    /// Whether responding to health checks
    pub responding: bool,
    /// Last error (if any)
    pub last_error: Option<String>,
    /// Metrics
    pub metrics: HashMap<String, f64>,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self {
            score: 100,
            responding: true,
            last_error: None,
            metrics: HashMap::new(),
        }
    }
}

/// Configuration entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigEntry {
    /// Key
    pub key: String,
    /// Value
    pub value: serde_json::Value,
    /// Whether this is a secret
    pub is_secret: bool,
    /// Last updated
    pub updated_at: DateTime<Utc>,
    /// Who updated it
    pub updated_by: String,
    /// Version number
    pub version: u32,
}

/// Lifecycle event for a resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleEvent {
    /// Event ID
    pub id: String,
    /// Resource ID
    pub resource_id: String,
    /// Event type
    pub event_type: LifecycleEventType,
    /// When occurred
    pub timestamp: DateTime<Utc>,
    /// Who triggered it
    pub triggered_by: String,
    /// Additional details
    pub details: Option<serde_json::Value>,
}

/// Types of lifecycle events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LifecycleEventType {
    /// Resource created
    Created,
    /// Resource started
    Started,
    /// Resource stopped
    Stopped,
    /// Resource destroyed
    Destroyed,
    /// Resource updated
    Updated,
    /// Resource scaled
    Scaled,
    /// Resource failed
    Failed,
    /// Resource recovered
    Recovered,
}

/// Resource usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Resource ID
    pub resource_id: String,
    /// Total requests handled
    pub total_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Average response time (ms)
    pub avg_response_time: f64,
    /// Current connections
    pub current_connections: u32,
    /// Maximum connections
    pub max_connections: u32,
    /// CPU usage (0-100)
    pub cpu_usage: f64,
    /// Memory usage in MB
    pub memory_usage: f64,
    /// Last updated
    pub last_updated: DateTime<Utc>,
}

/// The Contractor servant
pub struct Contractor {
    /// Unique ID
    id: ServantId,
    /// Current status
    status: RwLock<ServantStatus>,
    /// Consensus engine reference
    consensus: Option<Arc<ConsensusEngine>>,
    /// Managed resources
    resources: RwLock<HashMap<String, Resource>>,
    /// Configuration store
    config_store: RwLock<HashMap<String, ConfigEntry>>,
    /// Health check interval in seconds
    health_check_interval: u64,
    /// Lifecycle event history
    lifecycle_events: RwLock<Vec<LifecycleEvent>>,
    /// Resource usage statistics
    usage_stats: RwLock<HashMap<String, ResourceUsage>>,
}

impl Contractor {
    /// Create a new Contractor
    pub fn new() -> Self {
        Self {
            id: ServantId::new(ServantRole::Contractor.default_id()),
            status: RwLock::new(ServantStatus::Starting),
            consensus: None,
            resources: RwLock::new(HashMap::new()),
            config_store: RwLock::new(HashMap::new()),
            health_check_interval: 60,
            lifecycle_events: RwLock::new(Vec::new()),
            usage_stats: RwLock::new(HashMap::new()),
        }
    }

    /// Set the consensus engine
    pub fn with_consensus(mut self, consensus: Arc<ConsensusEngine>) -> Self {
        self.consensus = Some(consensus);
        self
    }

    /// Register a resource with lifecycle tracking
    pub fn register_resource(&self, mut resource: Resource, triggered_by: String) {
        resource.id = uuid::Uuid::new_v4().to_string();

        // Log creation event
        self.log_lifecycle_event(LifecycleEvent {
            id: uuid::Uuid::new_v4().to_string(),
            resource_id: resource.id.clone(),
            event_type: LifecycleEventType::Created,
            timestamp: Utc::now(),
            triggered_by,
            details: Some(serde_json::json!({
                "name": resource.name,
                "type": format!("{:?}", resource.resource_type),
            })),
        });

        // Initialize usage stats
        self.usage_stats.write().insert(
            resource.id.clone(),
            ResourceUsage {
                resource_id: resource.id.clone(),
                total_requests: 0,
                failed_requests: 0,
                avg_response_time: 0.0,
                current_connections: 0,
                max_connections: 100,
                cpu_usage: 0.0,
                memory_usage: 0.0,
                last_updated: Utc::now(),
            },
        );

        self.resources.write().insert(resource.id.clone(), resource);
    }

    /// Unregister a resource with lifecycle tracking
    pub fn unregister_resource(
        &self,
        resource_id: &str,
        triggered_by: String,
    ) -> Result<(), ServantError> {
        // Log destruction event
        self.log_lifecycle_event(LifecycleEvent {
            id: uuid::Uuid::new_v4().to_string(),
            resource_id: resource_id.to_string(),
            event_type: LifecycleEventType::Destroyed,
            timestamp: Utc::now(),
            triggered_by,
            details: None,
        });

        // Remove resource
        if self.resources.write().remove(resource_id).is_some() {
            // Remove usage stats
            self.usage_stats.write().remove(resource_id);
            Ok(())
        } else {
            Err(ServantError::InvalidTask(format!(
                "Resource {} not found",
                resource_id
            )))
        }
    }

    /// Start a resource
    pub async fn start_resource(
        &self,
        resource_id: &str,
        triggered_by: String,
    ) -> Result<(), ServantError> {
        let mut resources = self.resources.write();
        let resource = resources.get_mut(resource_id).ok_or_else(|| {
            ServantError::InvalidTask(format!("Resource {} not found", resource_id))
        })?;

        // Update status
        let old_status = resource.status.clone();
        resource.status = ResourceStatus::Starting;

        // Log start event
        self.log_lifecycle_event(LifecycleEvent {
            id: uuid::Uuid::new_v4().to_string(),
            resource_id: resource_id.to_string(),
            event_type: LifecycleEventType::Started,
            timestamp: Utc::now(),
            triggered_by,
            details: Some(serde_json::json!({
                "previous_status": format!("{:?}", old_status),
            })),
        });

        // TODO: Implement actual resource start
        // For now, mark as healthy
        resource.status = ResourceStatus::Healthy;

        Ok(())
    }

    /// Stop a resource
    pub async fn stop_resource(
        &self,
        resource_id: &str,
        triggered_by: String,
    ) -> Result<(), ServantError> {
        let mut resources = self.resources.write();
        let resource = resources.get_mut(resource_id).ok_or_else(|| {
            ServantError::InvalidTask(format!("Resource {} not found", resource_id))
        })?;

        // Update status
        let old_status = resource.status.clone();
        resource.status = ResourceStatus::Stopped;

        // Log stop event
        self.log_lifecycle_event(LifecycleEvent {
            id: uuid::Uuid::new_v4().to_string(),
            resource_id: resource_id.to_string(),
            event_type: LifecycleEventType::Stopped,
            timestamp: Utc::now(),
            triggered_by,
            details: Some(serde_json::json!({
                "previous_status": format!("{:?}", old_status),
            })),
        });

        // TODO: Implement actual resource stop

        Ok(())
    }

    /// Log a lifecycle event
    fn log_lifecycle_event(&self, event: LifecycleEvent) {
        self.lifecycle_events.write().push(event);
    }

    /// Get lifecycle events for a resource
    pub fn get_lifecycle_events(&self, resource_id: &str) -> Vec<LifecycleEvent> {
        self.lifecycle_events
            .read()
            .iter()
            .filter(|e| e.resource_id == resource_id)
            .cloned()
            .collect()
    }

    /// Get all lifecycle events
    pub fn get_all_lifecycle_events(&self) -> Vec<LifecycleEvent> {
        self.lifecycle_events.read().clone()
    }

    /// Update resource usage statistics
    pub fn update_usage_stats(
        &self,
        resource_id: &str,
        stats: ResourceUsage,
    ) -> Result<(), ServantError> {
        let mut usage_stats = self.usage_stats.write();

        if !usage_stats.contains_key(resource_id) {
            return Err(ServantError::InvalidTask(format!(
                "Resource {} not found",
                resource_id
            )));
        }

        usage_stats.insert(resource_id.to_string(), stats);
        Ok(())
    }

    /// Get usage statistics for a resource
    pub fn get_usage_stats(&self, resource_id: &str) -> Option<ResourceUsage> {
        self.usage_stats.read().get(resource_id).cloned()
    }

    /// Get all usage statistics
    pub fn get_all_usage_stats(&self) -> Vec<ResourceUsage> {
        self.usage_stats.read().values().cloned().collect()
    }

    /// Log a resource request
    pub fn log_request(&self, resource_id: &str, success: bool, response_time_ms: f64) {
        if let Some(stats) = self.usage_stats.write().get_mut(resource_id) {
            stats.total_requests += 1;
            if !success {
                stats.failed_requests += 1;
            }

            // Update average response time
            let total_time = stats.avg_response_time * (stats.total_requests - 1) as f64;
            stats.avg_response_time = (total_time + response_time_ms) / stats.total_requests as f64;

            stats.last_updated = Utc::now();
        }
    }

    /// Unregister a resource (deprecated, use unregister_resource with triggered_by)
    pub fn unregister_resource_deprecated(&self, resource_id: &str) {
        self.resources.write().remove(resource_id);
    }

    /// Get all resources
    pub fn get_resources(&self) -> Vec<Resource> {
        self.resources.read().values().cloned().collect()
    }

    /// Get a specific resource
    pub fn get_resource(&self, resource_id: &str) -> Option<Resource> {
        self.resources.read().get(resource_id).cloned()
    }

    /// Update resource status
    pub fn update_resource_status(
        &self,
        resource_id: &str,
        status: ResourceStatus,
    ) -> Result<(), ServantError> {
        let mut resources = self.resources.write();
        let resource = resources.get_mut(resource_id).ok_or_else(|| {
            ServantError::InvalidTask(format!("Resource {} not found", resource_id))
        })?;

        resource.status = status;
        resource.last_check = Some(Utc::now());
        Ok(())
    }

    /// Perform health check on a resource
    pub async fn health_check(&self, resource_id: &str) -> Result<HealthStatus, ServantError> {
        let mut resources = self.resources.write();
        let resource = resources.get_mut(resource_id).ok_or_else(|| {
            ServantError::InvalidTask(format!("Resource {} not found", resource_id))
        })?;

        // TODO: Implement actual health check based on resource type
        // For now, return a mock healthy status

        let health = HealthStatus {
            score: 100,
            responding: true,
            last_error: None,
            metrics: HashMap::new(),
        };

        resource.health = health.clone();
        resource.last_check = Some(Utc::now());

        Ok(health)
    }

    /// Perform health checks on all resources
    pub async fn health_check_all(&self) -> HashMap<String, HealthStatus> {
        let resource_ids: Vec<String> = self.resources.read().keys().cloned().collect();
        let mut results = HashMap::new();

        for id in resource_ids {
            if let Ok(health) = self.health_check(&id).await {
                results.insert(id, health);
            }
        }

        results
    }

    /// Set a configuration value with version tracking
    pub fn set_config(
        &self,
        key: String,
        value: serde_json::Value,
        is_secret: bool,
        updated_by: String,
    ) -> Result<(), ServantError> {
        // If this is a secret, it requires approval
        if is_secret {
            if let Some(consensus) = &self.consensus {
                if consensus.requires_vote(&DecisionType::SystemUpdate) {
                    return Err(ServantError::Internal(
                        "Secret configuration requires approval".to_string(),
                    ));
                }
            }
        }

        let mut store = self.config_store.write();

        // Get old version
        let old_version = store.get(&key).map(|e| e.version).unwrap_or(0);
        let new_version = old_version + 1;

        let entry = ConfigEntry {
            key: key.clone(),
            value: if is_secret {
                // Mask secret values in logs (but store them)
                serde_json::json!("*****HIDDEN*****")
            } else {
                value.clone()
            },
            is_secret,
            updated_at: Utc::now(),
            updated_by,
            version: new_version,
        };

        // Store the actual value (even if secret)
        let mut actual_entry = entry.clone();
        if is_secret {
            actual_entry.value = value;
        }

        store.insert(key, actual_entry);

        println!(
            "[Contractor] Config updated: {} (v{}, by {})",
            key, new_version, updated_by
        );

        Ok(())
    }

    /// Get a configuration value (without exposing secrets unless authorized)
    pub fn get_config(&self, key: &str) -> Option<ConfigEntry> {
        let entry = self.config_store.read().get(key).cloned()?;

        // Mask secrets in returned value
        if entry.is_secret {
            let mut masked = entry.clone();
            masked.value = serde_json::json!("*****HIDDEN*****");
            Some(masked)
        } else {
            Some(entry)
        }
    }

    /// Get a configuration value with secret revealed (use with caution)
    pub fn get_config_with_secret(&self, key: &str) -> Option<ConfigEntry> {
        self.config_store.read().get(key).cloned()
    }

    /// Get all configuration keys (not values, for security)
    pub fn get_config_keys(&self) -> Vec<String> {
        self.config_store.read().keys().cloned().collect()
    }

    /// Get configuration metadata (keys, versions, update times, but not values)
    pub fn get_config_metadata(&self) -> Vec<serde_json::Value> {
        self.config_store
            .read()
            .values()
            .map(|e| {
                serde_json::json!({
                    "key": e.key,
                    "is_secret": e.is_secret,
                    "version": e.version,
                    "updated_at": e.updated_at,
                    "updated_by": e.updated_by,
                })
            })
            .collect()
    }

    /// Delete a configuration value
    pub fn delete_config(&self, key: &str) -> Result<(), ServantError> {
        if self.config_store.write().remove(key).is_some() {
            println!("[Contractor] Config deleted: {}", key);
            Ok(())
        } else {
            Err(ServantError::InvalidTask(format!(
                "Config key {} not found",
                key
            )))
        }
    }

    /// Rollback a configuration to a previous version
    pub fn rollback_config(&self, key: &str, to_version: u32) -> Result<(), ServantError> {
        // TODO: Implement config version history and rollback
        // For now, this is a placeholder
        Err(ServantError::Internal(
            "Config rollback not implemented yet".to_string(),
        ))
    }

    /// Scale a resource (if supported)
    pub async fn scale_resource(
        &self,
        resource_id: &str,
        scale_factor: f32,
    ) -> Result<(), ServantError> {
        // This requires consensus for resource allocation
        if let Some(consensus) = &self.consensus {
            if consensus.requires_vote(&DecisionType::ResourceAllocation) {
                // TODO: Create proposal and wait for approval
                return Err(ServantError::Internal(
                    "Scaling requires approval".to_string(),
                ));
            }
        }

        // TODO: Implement actual scaling
        // For now, just update the config
        let mut resources = self.resources.write();
        if let Some(resource) = resources.get_mut(resource_id) {
            resource.config["scale_factor"] = serde_json::json!(scale_factor);
        }

        Ok(())
    }

    /// Deploy a new resource or update an existing one
    pub async fn deploy(&self, mut resource: Resource) -> Result<String, ServantError> {
        // This requires consensus
        if let Some(consensus) = &self.consensus {
            if consensus.requires_vote(&DecisionType::SystemUpdate) {
                // TODO: Create proposal and wait for approval
                return Err(ServantError::Internal(
                    "Deployment requires approval".to_string(),
                ));
            }
        }

        let id = resource.id.clone();
        resource.status = ResourceStatus::Starting;

        self.resources.write().insert(id.clone(), resource);

        // TODO: Implement actual deployment

        Ok(id)
    }

    /// Get system health overview
    pub fn get_system_health(&self) -> SystemHealth {
        let resources = self.resources.read();
        let total = resources.len();
        let healthy = resources
            .values()
            .filter(|r| r.status == ResourceStatus::Healthy)
            .count();
        let degraded = resources
            .values()
            .filter(|r| r.status == ResourceStatus::Degraded)
            .count();
        let unhealthy = resources
            .values()
            .filter(|r| r.status == ResourceStatus::Unhealthy)
            .count();

        let overall_score = if total == 0 {
            100
        } else {
            ((healthy * 100 + degraded * 50) / total) as u8
        };

        SystemHealth {
            overall_score,
            total_resources: total,
            healthy_count: healthy,
            degraded_count: degraded,
            unhealthy_count: unhealthy,
        }
    }

    /// Vote on a proposal
    pub async fn vote_on_proposal(
        &self,
        proposal_id: &str,
        vote: Vote,
        reason: String,
    ) -> Result<(), ServantError> {
        if let Some(consensus) = &self.consensus {
            consensus
                .cast_vote(proposal_id, self.id.as_str().to_string(), vote, reason)
                .map_err(|e| ServantError::Internal(e.to_string()))?;
        }
        Ok(())
    }
}

impl Default for Contractor {
    fn default() -> Self {
        Self::new()
    }
}

/// System health overview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    /// Overall health score (0-100)
    pub overall_score: u8,
    /// Total number of resources
    pub total_resources: usize,
    /// Number of healthy resources
    pub healthy_count: usize,
    /// Number of degraded resources
    pub degraded_count: usize,
    /// Number of unhealthy resources
    pub unhealthy_count: usize,
}

#[async_trait]
impl Servant for Contractor {
    fn id(&self) -> &ServantId {
        &self.id
    }

    fn role(&self) -> ServantRole {
        ServantRole::Contractor
    }

    fn status(&self) -> ServantStatus {
        self.status.read().clone()
    }

    async fn start(&mut self) -> Result<(), ServantError> {
        *self.status.write() = ServantStatus::Ready;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), ServantError> {
        *self.status.write() = ServantStatus::Stopping;
        *self.status.write() = ServantStatus::Paused;
        Ok(())
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "resource_management".to_string(),
            "config_management".to_string(),
            "health_check".to_string(),
            "deployment".to_string(),
            "scaling".to_string(),
            "lifecycle_management".to_string(),
            "usage_tracking".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_contractor_creation() {
        let contractor = Contractor::new();
        assert_eq!(contractor.role(), ServantRole::Contractor);
        assert_eq!(contractor.status(), ServantStatus::Starting);
    }

    #[tokio::test]
    async fn test_contractor_start_stop() {
        let mut contractor = Contractor::new();

        contractor.start().await.unwrap();
        assert_eq!(contractor.status(), ServantStatus::Ready);

        contractor.stop().await.unwrap();
        assert_eq!(contractor.status(), ServantStatus::Paused);
    }

    #[tokio::test]
    async fn test_resource_management() {
        let contractor = Contractor::new();

        let resource = Resource {
            id: "res-001".to_string(),
            name: "Test Database".to_string(),
            resource_type: ResourceType::Database,
            status: ResourceStatus::Healthy,
            config: serde_json::json!({"host": "localhost"}),
            health: HealthStatus::default(),
            last_check: None,
            tags: vec!["primary".to_string()],
        };

        contractor.register_resource(resource);

        assert_eq!(contractor.get_resources().len(), 1);
        assert!(contractor.get_resource("res-001").is_some());

        contractor.unregister_resource("res-001");
        assert_eq!(contractor.get_resources().len(), 0);
    }

    #[tokio::test]
    async fn test_config_management() {
        let contractor = Contractor::new();

        contractor
            .set_config(
                "app.port".to_string(),
                serde_json::json!(8080),
                false,
                "coordinator".to_string(),
            )
            .unwrap();

        let config = contractor.get_config("app.port").unwrap();
        assert_eq!(config.value, serde_json::json!(8080));
        assert!(!config.is_secret);

        // Update the config
        contractor
            .set_config(
                "app.port".to_string(),
                serde_json::json!(9090),
                false,
                "warden".to_string(),
            )
            .unwrap();

        let updated = contractor.get_config("app.port").unwrap();
        assert_eq!(updated.version, 2);
        assert_eq!(updated.updated_by, "warden");
    }

    #[tokio::test]
    async fn test_health_check() {
        let mut contractor = Contractor::new();
        *contractor.status.write() = ServantStatus::Ready;

        let resource = Resource {
            id: "res-001".to_string(),
            name: "Test".to_string(),
            resource_type: ResourceType::Cache,
            status: ResourceStatus::Healthy,
            config: serde_json::json!({}),
            health: HealthStatus::default(),
            last_check: None,
            tags: vec![],
        };

        contractor.register_resource(resource);

        let health = contractor.health_check("res-001").await.unwrap();
        assert!(health.responding);
        assert_eq!(health.score, 100);
    }

    #[test]
    fn test_system_health() {
        let contractor = Contractor::new();

        // Empty system should be healthy
        let health = contractor.get_system_health();
        assert_eq!(health.overall_score, 100);

        // Add resources
        contractor.register_resource(Resource {
            id: "r1".to_string(),
            name: "R1".to_string(),
            resource_type: ResourceType::Database,
            status: ResourceStatus::Healthy,
            config: serde_json::json!({}),
            health: HealthStatus::default(),
            last_check: None,
            tags: vec![],
        });

        contractor.register_resource(Resource {
            id: "r2".to_string(),
            name: "R2".to_string(),
            resource_type: ResourceType::Cache,
            status: ResourceStatus::Degraded,
            config: serde_json::json!({}),
            health: HealthStatus::default(),
            last_check: None,
            tags: vec![],
        });

        let health = contractor.get_system_health();
        assert_eq!(health.total_resources, 2);
        assert_eq!(health.healthy_count, 1);
        assert_eq!(health.degraded_count, 1);
    }
}
