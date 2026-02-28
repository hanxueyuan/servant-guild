//! Security Hardening Module
//!
//! Provides security primitives for:
//! - Network isolation and segmentation
//! - Secrets management and encryption
//! - Input validation and sanitization
//! - Audit logging and compliance

pub mod audit;
pub mod encryption;
pub mod network;
pub mod secrets;
pub mod validation;

use serde::{Deserialize, Serialize};

pub use audit::{AuditLog, AuditEntry};
pub use encryption::{EncryptionKey, Encryptor};
pub use network::{NetworkPolicy, NetworkIsolation};
pub use secrets::{SecretsManager, Secret};
pub use validation::{InputValidator, ValidationError};

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable network isolation
    pub enable_network_isolation: bool,
    /// Enable secrets encryption
    pub enable_encryption: bool,
    /// Enable audit logging
    pub enable_audit_logging: bool,
    /// Audit log retention days
    pub audit_retention_days: u32,
    /// Enable input validation
    pub enable_input_validation: bool,
    /// Maximum request size in bytes
    pub max_request_size: usize,
    /// Enable rate limiting
    pub enable_rate_limiting: bool,
    /// Rate limit requests per minute
    pub rate_limit_per_minute: u32,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_network_isolation: true,
            enable_encryption: true,
            enable_audit_logging: true,
            audit_retention_days: 90,
            enable_input_validation: true,
            max_request_size: 10 * 1024 * 1024, // 10MB
            enable_rate_limiting: true,
            rate_limit_per_minute: 100,
        }
    }
}

/// Security level for operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityLevel {
    /// Standard operations
    Normal,
    /// Elevated privileges required
    Elevated,
    /// Critical operations requiring multi-party authorization
    Critical,
}

/// Security context for operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityContext {
    /// Operation ID
    pub operation_id: uuid::Uuid,
    /// Agent performing the operation
    pub agent: String,
    /// Security level
    pub level: SecurityLevel,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Source IP (if applicable)
    pub source_ip: Option<String>,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl SecurityContext {
    /// Create new security context
    pub fn new(agent: String, level: SecurityLevel) -> Self {
        Self {
            operation_id: uuid::Uuid::new_v4(),
            agent,
            level,
            timestamp: chrono::Utc::now(),
            source_ip: None,
            metadata: std::collections::HashMap::new(),
        }
    }
}

/// Security manager
pub struct SecurityManager {
    config: SecurityConfig,
    audit_log: AuditLog,
    secrets_manager: SecretsManager,
    encryptor: Encryptor,
    validator: InputValidator,
}

impl SecurityManager {
    /// Create new security manager
    pub fn new(config: SecurityConfig) -> Self {
        let audit_log = AuditLog::new(config.audit_retention_days);
        let secrets_manager = SecretsManager::new();
        let encryptor = Encryptor::new();
        let validator = InputValidator::new(config.max_request_size);
        
        Self {
            config,
            audit_log,
            secrets_manager,
            encryptor,
            validator,
        }
    }
    
    /// Create security context for an operation
    pub fn create_context(&self, agent: &str, level: SecurityLevel) -> SecurityContext {
        SecurityContext::new(agent.to_string(), level)
    }
    
    /// Log an audit entry
    pub async fn log_audit(&self, entry: AuditEntry) {
        if self.config.enable_audit_logging {
            self.audit_log.log(entry).await;
        }
    }
    
    /// Get a secret
    pub async fn get_secret(&self, key: &str) -> Result<String, String> {
        self.secrets_manager.get(key).await
    }
    
    /// Set a secret
    pub async fn set_secret(&self, key: &str, value: &str) -> Result<(), String> {
        self.secrets_manager.set(key, value).await
    }
    
    /// Encrypt data
    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        if self.config.enable_encryption {
            self.encryptor.encrypt(data)
        } else {
            Ok(data.to_vec())
        }
    }
    
    /// Decrypt data
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        if self.config.enable_encryption {
            self.encryptor.decrypt(data)
        } else {
            Ok(data.to_vec())
        }
    }
    
    /// Validate input
    pub fn validate_input(&self, input: &str) -> Result<(), ValidationError> {
        if self.config.enable_input_validation {
            self.validator.validate(input)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        
        assert!(config.enable_network_isolation);
        assert!(config.enable_encryption);
        assert!(config.enable_audit_logging);
        assert_eq!(config.audit_retention_days, 90);
    }
    
    #[test]
    fn test_security_context() {
        let ctx = SecurityContext::new("coordinator".to_string(), SecurityLevel::Normal);
        
        assert_eq!(ctx.agent, "coordinator");
        assert_eq!(ctx.level, SecurityLevel::Normal);
    }
}
