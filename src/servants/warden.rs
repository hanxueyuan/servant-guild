//! Warden Servant - Security Auditing and Safety Enforcement
//!
//! The Warden is the "guardian" of the guild, responsible for:
//! - Auditing all operations for safety
//! - Enforcing security policies
//! - Managing snapshots and rollback points
//! - Validating tool execution requests
//! - Monitoring for suspicious activity

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use super::{
    Servant, ServantError, ServantId, ServantResult, ServantRole, ServantStatus, ServantTask,
};
use crate::consensus::{ConsensusEngine, Vote};
use crate::safety::{AuditLogger, Snapshot, TransactionManager};

// Type aliases for consistency
type AuditLog = AuditLogger;
type RollbackManager = Mutex<TransactionManager>;

/// Security policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Maximum risk level allowed without approval (1-10)
    pub max_auto_risk_level: u8,
    /// Whether to require snapshots before changes
    pub require_snapshots: bool,
    /// Whether to enforce audit logging
    pub enforce_audit: bool,
    /// Maximum operations per minute
    pub rate_limit: u32,
    /// Whether to block external network access
    pub block_network: bool,
    /// List of allowed domains (if network allowed)
    pub allowed_domains: Vec<String>,
    /// List of blocked file patterns
    pub blocked_patterns: Vec<String>,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            max_auto_risk_level: 5,
            require_snapshots: true,
            enforce_audit: true,
            rate_limit: 60,
            block_network: false,
            allowed_domains: Vec::new(),
            blocked_patterns: vec![
                "**/.env".to_string(),
                "**/secrets.*".to_string(),
                "**/credentials.*".to_string(),
            ],
        }
    }
}

/// Result of a security check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityCheckResult {
    /// Whether the operation is allowed
    pub allowed: bool,
    /// Reason for decision
    pub reason: String,
    /// Risk level of the operation (1-10)
    pub risk_level: u8,
    /// Whether approval is required
    pub requires_approval: bool,
    /// Any warnings
    pub warnings: Vec<String>,
}

/// Record of a security event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    /// Event ID
    pub id: String,
    /// Type of event
    pub event_type: SecurityEventType,
    /// Description
    pub description: String,
    /// Servant that triggered the event
    pub source: String,
    /// Risk level
    pub risk_level: u8,
    /// When it occurred
    pub timestamp: DateTime<Utc>,
    /// Whether it was blocked
    pub blocked: bool,
}

/// Types of security events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEventType {
    /// Tool execution attempt
    ToolExecution,
    /// File access attempt
    FileAccess,
    /// Network request attempt
    NetworkRequest,
    /// Policy violation
    PolicyViolation,
    /// Suspicious activity detected
    SuspiciousActivity,
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Security scan result
    SecurityScan,
}

/// The Warden servant
pub struct Warden {
    /// Unique ID
    id: ServantId,
    /// Current status
    status: RwLock<ServantStatus>,
    /// Consensus engine reference
    consensus: Option<Arc<ConsensusEngine>>,
    /// Security policy
    policy: RwLock<SecurityPolicy>,
    /// Audit log reference
    audit_log: Option<Arc<AuditLog>>,
    /// Rollback manager reference
    rollback_manager: Option<Arc<RollbackManager>>,
    /// Security events log
    events: RwLock<Vec<SecurityEvent>>,
    /// Operation rate tracker
    rate_tracker: RwLock<HashMap<String, Vec<DateTime<Utc>>>>,
}

impl Warden {
    /// Create a new Warden
    pub fn new() -> Self {
        Self {
            id: ServantId::new(ServantRole::Warden.default_id()),
            status: RwLock::new(ServantStatus::Starting),
            consensus: None,
            policy: RwLock::new(SecurityPolicy::default()),
            audit_log: None,
            rollback_manager: None,
            events: RwLock::new(Vec::new()),
            rate_tracker: RwLock::new(HashMap::new()),
        }
    }

    /// Set the consensus engine
    pub fn with_consensus(mut self, consensus: Arc<ConsensusEngine>) -> Self {
        self.consensus = Some(consensus);
        self
    }

    /// Set the audit log
    pub fn with_audit_log(mut self, audit_log: Arc<AuditLog>) -> Self {
        self.audit_log = Some(audit_log);
        self
    }

    /// Set the rollback manager
    pub fn with_rollback_manager(mut self, manager: Arc<RollbackManager>) -> Self {
        self.rollback_manager = Some(manager);
        self
    }

    /// Update security policy
    pub fn set_policy(&self, policy: SecurityPolicy) {
        *self.policy.write() = policy;
    }

    /// Get current security policy
    pub fn get_policy(&self) -> SecurityPolicy {
        self.policy.read().clone()
    }

    /// Check if an operation is allowed
    pub fn check_operation(
        &self,
        operation_type: &str,
        params: &serde_json::Value,
        source: &str,
    ) -> SecurityCheckResult {
        let policy = self.policy.read();
        let mut warnings = Vec::new();

        // Determine risk level based on operation type
        let risk_level = Self::calculate_risk_level(operation_type, params);

        // Check rate limit
        if !self.check_rate_limit(source, policy.rate_limit) {
            self.log_event(SecurityEvent {
                id: uuid::Uuid::new_v4().to_string(),
                event_type: SecurityEventType::RateLimitExceeded,
                description: format!("Rate limit exceeded for {}", source),
                source: source.to_string(),
                risk_level,
                timestamp: Utc::now(),
                blocked: true,
            });

            return SecurityCheckResult {
                allowed: false,
                reason: "Rate limit exceeded".to_string(),
                risk_level,
                requires_approval: false,
                warnings: vec!["Wait before making more requests".to_string()],
            };
        }

        // Check risk level
        let requires_approval = risk_level > policy.max_auto_risk_level;

        // Check for blocked patterns
        if let Some(path) = params.get("path").and_then(|p| p.as_str()) {
            for pattern in &policy.blocked_patterns {
                if Self::matches_pattern(path, pattern) {
                    self.log_event(SecurityEvent {
                        id: uuid::Uuid::new_v4().to_string(),
                        event_type: SecurityEventType::FileAccess,
                        description: format!("Blocked access to {}", path),
                        source: source.to_string(),
                        risk_level,
                        timestamp: Utc::now(),
                        blocked: true,
                    });

                    return SecurityCheckResult {
                        allowed: false,
                        reason: format!("Access to {} is blocked by policy", path),
                        risk_level,
                        requires_approval: false,
                        warnings,
                    };
                }
            }
        }

        // Check network access
        if policy.block_network && operation_type == "http_request" {
            warnings.push("Network access is currently blocked".to_string());

            return SecurityCheckResult {
                allowed: false,
                reason: "Network access is blocked by policy".to_string(),
                risk_level,
                requires_approval: false,
                warnings,
            };
        }

        // Log the check
        if policy.enforce_audit {
            self.log_event(SecurityEvent {
                id: uuid::Uuid::new_v4().to_string(),
                event_type: SecurityEventType::ToolExecution,
                description: format!("Operation {} checked", operation_type),
                source: source.to_string(),
                risk_level,
                timestamp: Utc::now(),
                blocked: false,
            });
        }

        SecurityCheckResult {
            allowed: true,
            reason: if requires_approval {
                "Allowed but requires approval due to high risk level".to_string()
            } else {
                "Operation allowed".to_string()
            },
            risk_level,
            requires_approval,
            warnings,
        }
    }

    /// Calculate risk level for an operation
    fn calculate_risk_level(operation_type: &str, params: &serde_json::Value) -> u8 {
        match operation_type {
            "read_file" | "analyze_code" => 1,
            "http_get" => 3,
            "http_request" => 4,
            "write_file" => 5,
            "run_command" => 7,
            "delete_file" => 8,
            "modify_system" => 9,
            _ => 5, // Default medium risk
        }
    }

    /// Check if a path matches a glob pattern
    fn matches_pattern(path: &str, pattern: &str) -> bool {
        // Simple glob matching for common patterns
        if pattern.starts_with("**/") {
            let suffix = &pattern[3..];
            return path.ends_with(suffix) || path.contains(&format!("/{}", suffix));
        }
        path == pattern
    }

    /// Check rate limit
    fn check_rate_limit(&self, source: &str, limit: u32) -> bool {
        let mut tracker = self.rate_tracker.write();
        let now = Utc::now();
        let minute_ago = now - chrono::Duration::seconds(60);

        // Get or create entry for this source
        let timestamps = tracker.entry(source.to_string()).or_insert_with(Vec::new);

        // Remove old entries
        timestamps.retain(|&t| t > minute_ago);

        // Check if under limit
        if timestamps.len() >= limit as usize {
            return false;
        }

        // Record this operation
        timestamps.push(now);
        true
    }

    /// Log a security event
    fn log_event(&self, event: SecurityEvent) {
        self.events.write().push(event);
    }

    /// Get security events
    pub fn get_events(&self) -> Vec<SecurityEvent> {
        self.events.read().clone()
    }

    /// Create a safety snapshot before a risky operation
    pub async fn create_snapshot(&self, operation_id: &str) -> Result<String, ServantError> {
        if let Some(manager) = &self.rollback_manager {
            manager
                .lock()
                .begin()
                .map_err(|e| ServantError::Internal(e.to_string()))?;
            Ok(operation_id.to_string())
        } else {
            Err(ServantError::Internal(
                "Rollback manager not configured".to_string(),
            ))
        }
    }

    /// Rollback to a snapshot
    pub async fn rollback(&self, _snapshot_id: &str) -> Result<(), ServantError> {
        if let Some(manager) = &self.rollback_manager {
            manager
                .lock()
                .rollback()
                .map_err(|e| ServantError::Internal(e.to_string()))?;
            Ok(())
        } else {
            Err(ServantError::Internal(
                "Rollback manager not configured".to_string(),
            ))
        }
    }

    /// Audit an operation
    pub async fn audit_operation(
        &self,
        operation: &str,
        details: serde_json::Value,
    ) -> Result<(), ServantError> {
        if let Some(audit_log) = &self.audit_log {
            use crate::safety::audit::AuditEvent;
            let event =
                AuditEvent::new(crate::safety::AuditEventType::Custom(operation.to_string()))
                    .with_result(true, None, 0, Some(details.to_string()));
            audit_log
                .log(&event)
                .map_err(|e| ServantError::Internal(e.to_string()))?;
        }
        Ok(())
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

impl Default for Warden {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Servant for Warden {
    fn id(&self) -> &ServantId {
        &self.id
    }

    fn role(&self) -> ServantRole {
        ServantRole::Warden
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
            "security_check".to_string(),
            "audit".to_string(),
            "snapshot".to_string(),
            "rollback".to_string(),
            "policy_enforcement".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_warden_creation() {
        let warden = Warden::new();
        assert_eq!(warden.role(), ServantRole::Warden);
        assert_eq!(warden.status(), ServantStatus::Starting);
    }

    #[tokio::test]
    async fn test_warden_start_stop() {
        let mut warden = Warden::new();

        warden.start().await.unwrap();
        assert_eq!(warden.status(), ServantStatus::Ready);

        warden.stop().await.unwrap();
        assert_eq!(warden.status(), ServantStatus::Paused);
    }

    #[test]
    fn test_check_safe_operation() {
        let mut warden = Warden::new();
        *warden.status.write() = ServantStatus::Ready;

        let result = warden.check_operation(
            "read_file",
            &serde_json::json!({"path": "/safe/path.txt"}),
            "worker",
        );

        assert!(result.allowed);
        assert!(!result.requires_approval);
    }

    #[test]
    fn test_check_risky_operation() {
        let mut warden = Warden::new();
        *warden.status.write() = ServantStatus::Ready;

        let result = warden.check_operation(
            "delete_file",
            &serde_json::json!({"path": "/important/file.txt"}),
            "worker",
        );

        assert!(result.allowed); // Still allowed
        assert!(result.requires_approval); // But needs approval
        assert_eq!(result.risk_level, 8);
    }

    #[test]
    fn test_blocked_pattern() {
        let mut warden = Warden::new();
        *warden.status.write() = ServantStatus::Ready;

        let result = warden.check_operation(
            "read_file",
            &serde_json::json!({"path": "/project/.env"}),
            "worker",
        );

        assert!(!result.allowed);
        assert!(result.reason.contains("blocked"));
    }

    #[test]
    fn test_rate_limit() {
        let mut warden = Warden::new();
        *warden.status.write() = ServantStatus::Ready;

        // Set a very low rate limit
        let mut policy = SecurityPolicy::default();
        policy.rate_limit = 2;
        warden.set_policy(policy);

        // First two should succeed
        assert!(
            warden
                .check_operation("read_file", &serde_json::json!({}), "worker1")
                .allowed
        );
        assert!(
            warden
                .check_operation("read_file", &serde_json::json!({}), "worker1")
                .allowed
        );

        // Third should be rate limited
        let result = warden.check_operation("read_file", &serde_json::json!({}), "worker1");
        assert!(!result.allowed);
        assert!(result.reason.contains("Rate limit"));

        // Different source should still work
        assert!(
            warden
                .check_operation("read_file", &serde_json::json!({}), "worker2")
                .allowed
        );
    }
}
