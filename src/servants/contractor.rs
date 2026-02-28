//! Contractor Servant - Resource Management and Configuration
//!
//! The Contractor is the "builder" of the guild, responsible for:
//! - Managing resources and configurations
//! - Handling deployments and scaling
//! - Managing external service integrations
//! - Maintaining system health
//! - Environment and secrets management

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::{
    Servant, ServantId, ServantRole, ServantStatus, ServantTask, ServantResult, ServantError,
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
        }
    }
    
    /// Set the consensus engine
    pub fn with_consensus(mut self, consensus: Arc<ConsensusEngine>) -> Self {
        self.consensus = Some(consensus);
        self
    }
    
    /// Register a resource
    pub fn register_resource(&self, resource: Resource) {
        self.resources.write().insert(resource.id.clone(), resource);
    }
    
    /// Unregister a resource
    pub fn unregister_resource(&self, resource_id: &str) {
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
    pub fn update_resource_status(&self, resource_id: &str, status: ResourceStatus) -> Result<(), ServantError> {
        let mut resources = self.resources.write();
        let resource = resources
            .get_mut(resource_id)
            .ok_or_else(|| ServantError::InvalidTask(format!("Resource {} not found", resource_id)))?;
        
        resource.status = status;
        resource.last_check = Some(Utc::now());
        Ok(())
    }
    
    /// Perform health check on a resource
    pub async fn health_check(&self, resource_id: &str) -> Result<HealthStatus, ServantError> {
        let mut resources = self.resources.write();
        let resource = resources
            .get_mut(resource_id)
            .ok_or_else(|| ServantError::InvalidTask(format!("Resource {} not found", resource_id)))?;
        
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
    
    /// Set a configuration value
    pub fn set_config(
        &self,
        key: String,
        value: serde_json::Value,
        is_secret: bool,
        updated_by: String,
    ) -> Result<(), ServantError> {
        // If this is a secret, it requires approval
        if is_secret {
            // TODO: Check with consensus for approval
        }
        
        let mut store = self.config_store.write();
        
        let version = store
            .get(&key)
            .map(|e| e.version + 1)
            .unwrap_or(1);
        
        let entry = ConfigEntry {
            key: key.clone(),
            value,
            is_secret,
            updated_at: Utc::now(),
            updated_by,
            version,
        };
        
        store.insert(key, entry);
        
        Ok(())
    }
    
    /// Get a configuration value
    pub fn get_config(&self, key: &str) -> Option<ConfigEntry> {
        self.config_store.read().get(key).cloned()
    }
    
    /// Get all configuration keys (not values, for security)
    pub fn get_config_keys(&self) -> Vec<String> {
        self.config_store.read().keys().cloned().collect()
    }
    
    /// Delete a configuration value
    pub fn delete_config(&self, key: &str) -> Result<(), ServantError> {
        self.config_store.write().remove(key);
        Ok(())
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
                return Err(ServantError::Internal("Scaling requires approval".to_string()));
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
    pub async fn deploy(
        &self,
        resource: Resource,
    ) -> Result<String, ServantError> {
        // This requires consensus
        if let Some(consensus) = &self.consensus {
            if consensus.requires_vote(&DecisionType::SystemUpdate) {
                // TODO: Create proposal and wait for approval
                return Err(ServantError::Internal("Deployment requires approval".to_string()));
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
        let healthy = resources.values().filter(|r| r.status == ResourceStatus::Healthy).count();
        let degraded = resources.values().filter(|r| r.status == ResourceStatus::Degraded).count();
        let unhealthy = resources.values().filter(|r| r.status == ResourceStatus::Unhealthy).count();
        
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
            consensus.cast_vote(proposal_id, self.id.as_str().to_string(), vote, reason)
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
        
        contractor.set_config(
            "app.port".to_string(),
            serde_json::json!(8080),
            false,
            "coordinator".to_string(),
        ).unwrap();
        
        let config = contractor.get_config("app.port").unwrap();
        assert_eq!(config.value, serde_json::json!(8080));
        assert!(!config.is_secret);
        
        // Update the config
        contractor.set_config(
            "app.port".to_string(),
            serde_json::json!(9090),
            false,
            "warden".to_string(),
        ).unwrap();
        
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
