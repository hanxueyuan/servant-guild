//! Audit Logging Module
//!
//! Provides comprehensive audit logging for compliance and security

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Security level for operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityLevel {
    Normal,
    Elevated,
    Critical,
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique entry ID
    pub id: uuid::Uuid,
    /// Operation type
    pub operation: AuditOperation,
    /// Agent that performed the operation
    pub agent: String,
    /// Target of the operation (if applicable)
    pub target: Option<String>,
    /// Security level of the operation
    pub security_level: SecurityLevel,
    /// Operation result
    pub result: OperationResult,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Source IP address
    pub source_ip: Option<String>,
    /// Session ID
    pub session_id: Option<uuid::Uuid>,
    /// Additional details
    pub details: std::collections::HashMap<String, String>,
    /// Duration in milliseconds
    pub duration_ms: Option<u64>,
}

/// Types of auditable operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditOperation {
    // Authentication
    Login,
    Logout,
    TokenRefresh,

    // Agent operations
    AgentSpawn,
    AgentTerminate,
    AgentEvolve,

    // Task operations
    TaskCreate,
    TaskExecute,
    TaskComplete,
    TaskFail,

    // Consensus operations
    ConsensusPropose,
    ConsensusVote,
    ConsensusFinalize,

    // Evolution operations
    EvolutionPropose,
    EvolutionApprove,
    EvolutionReject,
    EvolutionExecute,

    // Security operations
    SecretAccess,
    SecretRotate,
    EncryptionKeyRotate,
    PolicyChange,

    // Administrative
    ConfigurationChange,
    SystemRestart,
    BackupCreate,
    RestoreFromBackup,

    // Data operations
    DataRead,
    DataWrite,
    DataDelete,
    DataExport,
}

/// Result of an operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationResult {
    Success,
    Failure { reason: String },
    Unauthorized,
    Forbidden,
    Timeout,
    Cancelled,
}

/// Audit log
pub struct AuditLog {
    retention_days: u32,
    entries: Arc<RwLock<VecDeque<AuditEntry>>>,
    max_entries: usize,
}

impl AuditLog {
    /// Create new audit log
    pub fn new(retention_days: u32) -> Self {
        Self {
            retention_days,
            entries: Arc::new(RwLock::new(VecDeque::with_capacity(10000))),
            max_entries: 100000,
        }
    }

    /// Log an audit entry
    pub async fn log(&self, entry: AuditEntry) {
        let mut entries = self.entries.write().await;

        // Add entry
        entries.push_back(entry);

        // Prune old entries
        if entries.len() > self.max_entries {
            let cutoff = Utc::now() - Duration::days(self.retention_days as i64);
            entries.retain(|e| e.timestamp > cutoff);
        }
    }

    /// Get entries matching filter
    pub async fn query(&self, filter: &AuditFilter) -> Vec<AuditEntry> {
        let entries = self.entries.read().await;

        entries
            .iter()
            .filter(|e| filter.matches(e))
            .cloned()
            .collect()
    }

    /// Get entries for an agent
    pub async fn get_agent_entries(&self, agent: &str, limit: usize) -> Vec<AuditEntry> {
        let entries = self.entries.read().await;

        entries
            .iter()
            .filter(|e| e.agent == agent)
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get recent entries
    pub async fn recent(&self, limit: usize) -> Vec<AuditEntry> {
        let entries = self.entries.read().await;

        entries.iter().rev().take(limit).cloned().collect()
    }

    /// Get failed operations
    pub async fn failed_operations(&self, limit: usize) -> Vec<AuditEntry> {
        let entries = self.entries.read().await;

        entries
            .iter()
            .rev()
            .filter(|e| !matches!(e.result, OperationResult::Success))
            .take(limit)
            .cloned()
            .collect()
    }

    /// Export entries for compliance
    pub async fn export(&self, format: ExportFormat) -> Vec<u8> {
        let entries = self.entries.read().await;

        match format {
            ExportFormat::Json => serde_json::to_vec_pretty(&*entries).unwrap_or_default(),
            ExportFormat::Csv => {
                let mut csv = String::from("id,operation,agent,timestamp,result,details\n");
                for entry in entries.iter() {
                    csv.push_str(&format!(
                        "{},{},{},{},{:?},{}\n",
                        entry.id,
                        serde_json::to_string(&entry.operation).unwrap_or_default(),
                        entry.agent,
                        entry.timestamp.to_rfc3339(),
                        entry.result,
                        serde_json::to_string(&entry.details).unwrap_or_default(),
                    ));
                }
                csv.into_bytes()
            }
        }
    }

    /// Get statistics
    pub async fn stats(&self) -> AuditStats {
        let entries = self.entries.read().await;

        let total = entries.len();
        let mut success_count = 0;
        let mut failure_count = 0;
        let mut by_operation = std::collections::HashMap::new();
        let mut by_agent = std::collections::HashMap::new();

        for entry in entries.iter() {
            match entry.result {
                OperationResult::Success => success_count += 1,
                _ => failure_count += 1,
            }

            *by_operation
                .entry(format!("{:?}", entry.operation))
                .or_insert(0) += 1;
            *by_agent.entry(entry.agent.clone()).or_insert(0) += 1;
        }

        AuditStats {
            total_entries: total,
            success_count,
            failure_count,
            by_operation,
            by_agent,
        }
    }
}

/// Filter for audit queries
#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    pub agent: Option<String>,
    pub operation: Option<AuditOperation>,
    pub result: Option<OperationResult>,
    pub security_level: Option<SecurityLevel>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

impl AuditFilter {
    /// Check if entry matches filter
    pub fn matches(&self, entry: &AuditEntry) -> bool {
        if let Some(ref agent) = self.agent {
            if entry.agent != *agent {
                return false;
            }
        }

        if let Some(ref operation) = self.operation {
            if std::mem::discriminant(&entry.operation) != std::mem::discriminant(operation) {
                return false;
            }
        }

        if let Some(ref result) = self.result {
            if std::mem::discriminant(&entry.result) != std::mem::discriminant(result) {
                return false;
            }
        }

        if let Some(ref level) = self.security_level {
            if entry.security_level != *level {
                return false;
            }
        }

        if let Some(start) = self.start_time {
            if entry.timestamp < start {
                return false;
            }
        }

        if let Some(end) = self.end_time {
            if entry.timestamp > end {
                return false;
            }
        }

        true
    }
}

/// Export format for audit logs
#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    Json,
    Csv,
}

/// Audit statistics
#[derive(Debug, Clone, Serialize)]
pub struct AuditStats {
    pub total_entries: usize,
    pub success_count: u64,
    pub failure_count: u64,
    pub by_operation: std::collections::HashMap<String, u64>,
    pub by_agent: std::collections::HashMap<String, u64>,
}

impl AuditEntry {
    /// Create new audit entry
    pub fn new(operation: AuditOperation, agent: String, security_level: SecurityLevel) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            operation,
            agent,
            target: None,
            security_level,
            result: OperationResult::Success,
            timestamp: Utc::now(),
            source_ip: None,
            session_id: None,
            details: std::collections::HashMap::new(),
            duration_ms: None,
        }
    }

    /// Set target
    pub fn with_target(mut self, target: String) -> Self {
        self.target = Some(target);
        self
    }

    /// Set result
    pub fn with_result(mut self, result: OperationResult) -> Self {
        self.result = result;
        self
    }

    /// Set source IP
    pub fn with_source_ip(mut self, ip: String) -> Self {
        self.source_ip = Some(ip);
        self
    }

    /// Add detail
    pub fn add_detail(mut self, key: &str, value: &str) -> Self {
        self.details.insert(key.to_string(), value.to_string());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audit_log() {
        let log = AuditLog::new(90);

        let entry = AuditEntry::new(
            AuditOperation::TaskExecute,
            "worker".to_string(),
            SecurityLevel::Normal,
        );

        log.log(entry).await;

        let stats = log.stats().await;
        assert_eq!(stats.total_entries, 1);
    }

    #[tokio::test]
    async fn test_audit_filter() {
        let log = AuditLog::new(90);

        let entry = AuditEntry::new(
            AuditOperation::TaskExecute,
            "worker".to_string(),
            SecurityLevel::Normal,
        );

        log.log(entry).await;

        let filter = AuditFilter {
            agent: Some("worker".to_string()),
            ..Default::default()
        };

        let results = log.query(&filter).await;
        assert_eq!(results.len(), 1);
    }
}
