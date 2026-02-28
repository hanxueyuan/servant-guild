//! Audit logging for security events with tamper-evident hashing.
//! Migrated from src/security/audit.rs for ServantGuild Phase 1
//!
//! # Tamper-Evident Audit Log
//!
//! Each audit event contains a cryptographic hash of the previous event,
//! forming an immutable chain similar to blockchain. This makes it possible
//! to detect any tampering with historical audit records.
//!
//! ```text
//! Event 0: hash = SHA256("GENESIS" + event_data)
//! Event 1: hash = SHA256(Event 0 hash + event_data)
//! Event 2: hash = SHA256(Event 1 hash + event_data)
//! ...
//! ```

use crate::config::AuditConfig;
use anyhow::Result;
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use uuid::Uuid;

/// Genesis hash for the first event in the chain
const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Audit event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    CommandExecution,
    FileAccess,
    ConfigChange,
    AuthSuccess,
    AuthFailure,
    PolicyViolation,
    SecurityEvent,
    ServantAction, // New for ServantGuild
    Custom(String), // For custom event types
}

/// Actor information (who performed the action)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    pub channel: String,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub agent_id: Option<String>, // New for ServantGuild
}

/// Action information (what was done)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub command: Option<String>,
    pub risk_level: Option<String>,
    pub approved: bool,
    pub allowed: bool,
    pub resource: Option<String>, // New for ServantGuild
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
}

/// Security context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityContext {
    pub policy_violation: bool,
    pub rate_limit_remaining: Option<u32>,
    pub sandbox_backend: Option<String>,
}

/// Complete audit event with tamper-evident hashing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub timestamp: DateTime<Utc>,
    pub event_id: String,
    pub event_type: AuditEventType,
    pub actor: Option<Actor>,
    pub action: Option<Action>,
    pub result: Option<ExecutionResult>,
    pub security: SecurityContext,
    /// Hash of the previous event in the chain (for tamper detection)
    #[serde(default)]
    pub prev_hash: String,
    /// Hash of this event (computed during logging)
    #[serde(default)]
    pub hash: String,
}

impl AuditEvent {
    /// Create a new audit event (hash will be computed during logging)
    pub fn new(event_type: AuditEventType) -> Self {
        Self {
            timestamp: Utc::now(),
            event_id: Uuid::new_v4().to_string(),
            event_type,
            actor: None,
            action: None,
            result: None,
            security: SecurityContext {
                policy_violation: false,
                rate_limit_remaining: None,
                sandbox_backend: None,
            },
            prev_hash: GENESIS_HASH.to_string(),
            hash: String::new(),
        }
    }

    /// Compute the hash of this event based on its content and previous hash
    pub fn compute_hash(&self) -> String {
        let mut hasher = Sha256::new();
        
        // Include previous hash for chain integrity
        hasher.update(self.prev_hash.as_bytes());
        
        // Include all event data for integrity
        hasher.update(self.timestamp.to_rfc3339().as_bytes());
        hasher.update(self.event_id.as_bytes());
        hasher.update(serde_json::to_string(&self.event_type).unwrap_or_default().as_bytes());
        
        if let Some(ref actor) = self.actor {
            hasher.update(&actor.channel);
            if let Some(ref user_id) = actor.user_id {
                hasher.update(user_id.as_bytes());
            }
            if let Some(ref agent_id) = actor.agent_id {
                hasher.update(agent_id.as_bytes());
            }
        }
        
        if let Some(ref action) = self.action {
            if let Some(ref cmd) = action.command {
                hasher.update(cmd.as_bytes());
            }
            if let Some(ref resource) = action.resource {
                hasher.update(resource.as_bytes());
            }
        }
        
        format!("{:x}", hasher.finalize())
    }

    /// Set the previous hash for chain continuity
    pub fn with_prev_hash(mut self, prev_hash: String) -> Self {
        self.prev_hash = prev_hash;
        self
    }

    /// Finalize the event by computing its hash
    pub fn finalize(mut self) -> Self {
        self.hash = self.compute_hash();
        self
    }

    /// Set the actor
    pub fn with_actor(
        mut self,
        channel: String,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Self {
        self.actor = Some(Actor {
            channel,
            user_id,
            username,
            agent_id: None,
        });
        self
    }
    
    /// Set the agent actor
    pub fn with_agent(mut self, agent_id: String) -> Self {
        self.actor = Some(Actor {
            channel: "servant_guild".to_string(),
            user_id: None,
            username: None,
            agent_id: Some(agent_id),
        });
        self
    }

    /// Set the action
    pub fn with_action(
        mut self,
        command: String,
        risk_level: String,
        approved: bool,
        allowed: bool,
    ) -> Self {
        self.action = Some(Action {
            command: Some(command),
            risk_level: Some(risk_level),
            approved,
            allowed,
            resource: None,
        });
        self
    }

    /// Set the resource action
    pub fn with_resource_action(mut self, action: String, resource: String) -> Self {
        self.action = Some(Action {
            command: Some(action),
            risk_level: None,
            approved: false,
            allowed: true,
            resource: Some(resource),
        });
        self
    }

    /// Set the result
    pub fn with_result(
        mut self,
        success: bool,
        exit_code: Option<i32>,
        duration_ms: u64,
        error: Option<String>,
    ) -> Self {
        self.result = Some(ExecutionResult {
            success,
            exit_code,
            duration_ms: Some(duration_ms),
            error,
        });
        self
    }

    /// Set security context
    pub fn with_security(mut self, sandbox_backend: Option<String>) -> Self {
        self.security.sandbox_backend = sandbox_backend;
        self
    }
}

/// Audit logger with tamper-evident hash chain support
pub struct AuditLogger {
    log_path: PathBuf,
    config: AuditConfig,
    buffer: Mutex<Vec<AuditEvent>>,
    /// Hash of the last logged event (for chain continuity)
    last_hash: Mutex<String>,
}

/// Structured command execution details for audit logging.
#[derive(Debug, Clone)]
pub struct CommandExecutionLog<'a> {
    pub channel: &'a str,
    pub command: &'a str,
    pub risk_level: &'a str,
    pub approved: bool,
    pub allowed: bool,
    pub success: bool,
    pub duration_ms: u64,
}

impl AuditLogger {
    /// Create a new audit logger and initialize hash chain
    pub fn new(config: AuditConfig, zeroclaw_dir: PathBuf) -> Result<Self> {
        let log_path = zeroclaw_dir.join(&config.log_path);
        
        // Ensure log directory exists
        if let Some(parent) = log_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }
        
        // Load last hash from existing log if available
        let last_hash = Self::load_last_hash(&log_path).unwrap_or_else(|_| GENESIS_HASH.to_string());
        
        Ok(Self {
            log_path,
            config,
            buffer: Mutex::new(Vec::new()),
            last_hash: Mutex::new(last_hash),
        })
    }

    /// Load the hash of the last event from the log file
    fn load_last_hash(log_path: &PathBuf) -> Result<String> {
        if !log_path.exists() {
            return Ok(GENESIS_HASH.to_string());
        }
        
        let file = File::open(log_path)?;
        let reader = BufReader::new(file);
        
        // Read the last line
        let last_line = reader.lines().last().transpose()?.unwrap_or_default();
        if last_line.is_empty() {
            return Ok(GENESIS_HASH.to_string());
        }
        
        let event: AuditEvent = serde_json::from_str(&last_line)?;
        Ok(event.hash)
    }

    /// Log an event with automatic hash chain maintenance
    pub fn log(&self, event: &AuditEvent) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Check log size and rotate if needed
        self.rotate_if_needed()?;

        // Get previous hash and compute this event's hash
        let prev_hash = self.last_hash.lock().clone();
        let event_with_hash = event.clone()
            .with_prev_hash(prev_hash)
            .finalize();

        // Serialize and write
        let line = serde_json::to_string(&event_with_hash)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        writeln!(file, "{}", line)?;
        file.sync_all()?;

        // Update last hash
        *self.last_hash.lock() = event_with_hash.hash;

        Ok(())
    }

    /// Log a command execution event.
    pub fn log_command_event(&self, entry: CommandExecutionLog<'_>) -> Result<()> {
        let event = AuditEvent::new(AuditEventType::CommandExecution)
            .with_actor(entry.channel.to_string(), None, None)
            .with_action(
                entry.command.to_string(),
                entry.risk_level.to_string(),
                entry.approved,
                entry.allowed,
            )
            .with_result(entry.success, None, entry.duration_ms, None);

        self.log(&event)
    }

    /// Backward-compatible helper to log a command execution event.
    #[allow(clippy::too_many_arguments)]
    pub fn log_command(
        &self,
        channel: &str,
        command: &str,
        risk_level: &str,
        approved: bool,
        allowed: bool,
        success: bool,
        duration_ms: u64,
    ) -> Result<()> {
        self.log_command_event(CommandExecutionLog {
            channel,
            command,
            risk_level,
            approved,
            allowed,
            success,
            duration_ms,
        })
    }

    /// Rotate log if it exceeds max size
    fn rotate_if_needed(&self) -> Result<()> {
        if let Ok(metadata) = std::fs::metadata(&self.log_path) {
            let current_size_mb = metadata.len() / (1024 * 1024);
            if current_size_mb >= u64::from(self.config.max_size_mb) {
                self.rotate()?;
            }
        }
        Ok(())
    }

    /// Rotate the log file
    fn rotate(&self) -> Result<()> {
        for i in (1..10).rev() {
            let old_name = format!("{}.{}.log", self.log_path.display(), i);
            let new_name = format!("{}.{}.log", self.log_path.display(), i + 1);
            let _ = std::fs::rename(&old_name, &new_name);
        }

        let rotated = format!("{}.1.log", self.log_path.display());
        std::fs::rename(&self.log_path, &rotated)?;
        
        // Reset hash chain for new log file
        *self.last_hash.lock() = GENESIS_HASH.to_string();
        
        Ok(())
    }
    
    /// Verify the integrity of the audit log chain
    pub fn verify_chain(&self) -> Result<ChainVerificationResult> {
        if !self.log_path.exists() {
            return Ok(ChainVerificationResult {
                valid: true,
                total_events: 0,
                errors: vec![],
            });
        }
        
        let file = File::open(&self.log_path)?;
        let reader = BufReader::new(file);
        
        let mut prev_hash = GENESIS_HASH.to_string();
        let mut total_events = 0;
        let mut errors = vec![];
        
        for (line_num, line_result) in reader.lines().enumerate() {
            let line = line_result?;
            if line.is_empty() {
                continue;
            }
            
            match serde_json::from_str::<AuditEvent>(&line) {
                Ok(event) => {
                    // Verify previous hash matches
                    if event.prev_hash != prev_hash {
                        errors.push(format!(
                            "Line {}: Hash chain broken. Expected prev_hash={}, got {}",
                            line_num + 1, prev_hash, event.prev_hash
                        ));
                    }
                    
                    // Verify event hash is correct
                    let computed_hash = event.compute_hash();
                    if event.hash != computed_hash {
                        errors.push(format!(
                            "Line {}: Event hash mismatch. Expected {}, got {}",
                            line_num + 1, computed_hash, event.hash
                        ));
                    }
                    
                    prev_hash = event.hash.clone();
                    total_events += 1;
                }
                Err(e) => {
                    errors.push(format!("Line {}: Failed to parse event: {}", line_num + 1, e));
                }
            }
        }
        
        Ok(ChainVerificationResult {
            valid: errors.is_empty(),
            total_events,
            errors,
        })
    }
}

/// Result of audit log chain verification
#[derive(Debug, Clone)]
pub struct ChainVerificationResult {
    /// Whether the chain is valid (no errors)
    pub valid: bool,
    /// Total number of events in the chain
    pub total_events: usize,
    /// List of errors found during verification
    pub errors: Vec<String>,
}

impl std::fmt::Display for ChainVerificationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.valid {
            write!(f, "Audit log chain valid: {} events verified", self.total_events)
        } else {
            write!(
                f,
                "Audit log chain INVALID: {} errors in {} events",
                self.errors.len(),
                self.total_events
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_audit_event_hash_computation() {
        let event = AuditEvent::new(AuditEventType::CommandExecution)
            .with_actor("test".to_string(), None, None)
            .with_prev_hash(GENESIS_HASH.to_string())
            .finalize();
        
        assert!(!event.hash.is_empty());
        assert_eq!(event.prev_hash, GENESIS_HASH);
    }

    #[test]
    fn test_audit_log_chain_integrity() {
        let dir = tempdir().unwrap();
        let config = AuditConfig::default();
        let logger = AuditLogger::new(config, dir.path().to_path_buf()).unwrap();
        
        // Log multiple events
        for i in 0..5 {
            let event = AuditEvent::new(AuditEventType::CommandExecution)
                .with_action(
                    format!("test_command_{}", i),
                    "low".to_string(),
                    true,
                    true,
                );
            logger.log(&event).unwrap();
        }
        
        // Verify chain
        let result = logger.verify_chain().unwrap();
        assert!(result.valid);
        assert_eq!(result.total_events, 5);
    }

    #[test]
    fn test_genesis_hash_detection() {
        let event = AuditEvent::new(AuditEventType::SecurityEvent);
        assert_eq!(event.prev_hash, GENESIS_HASH);
    }
}
