//! Rollback mechanisms for file system and state changes.
//!
//! Provides transactional safety for operations that modify persistent state.
//! If an operation fails or is rejected by policy, these mechanisms attempt to
//! restore the previous state.
//!
//! # Atomic Transaction Model
//!
//! Each operation follows this lifecycle:
//! 1. **Prepare**: Create a snapshot of current state
//! 2. **Execute**: Perform the actual operation
//! 3. **Commit**: Mark as successful (snapshot can be cleaned up later)
//! 4. **Rollback**: If execution fails, restore from snapshot
//!
//! ```text
//! [State A] --prepare--> [Snapshot A] --execute--> [State B] --commit--> [Done]
//!                      ^                                  |
//!                      |________rollback_(on failure)____|
//! ```

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::safety::snapshot::{SnapshotManager, Snapshot, SnapshotType};

/// Recovery policy defining how to handle failures
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RecoveryPolicy {
    /// Automatically rollback on any failure
    AutomaticRollback,
    /// Prompt user before rollback
    PromptUser,
    /// Log failure but don't rollback
    LogOnly,
    /// Custom handler will be invoked
    CustomHandler,
}

impl Default for RecoveryPolicy {
    fn default() -> Self {
        RecoveryPolicy::AutomaticRollback
    }
}

/// Status of a transaction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionStatus {
    Pending,
    Prepared,
    Executing,
    Committed,
    RolledBack,
    Failed,
}

/// Represents a reversible operation.
pub trait ReversibleOp: Send + Sync {
    /// Get the operation name for logging
    fn name(&self) -> &str;
    
    /// Prepare for execution (create snapshot, etc.)
    fn prepare(&self) -> Result<RollbackData>;
    
    /// Execute the operation (e.g., write file).
    fn execute(&self) -> Result<()>;
    
    /// Revert the operation using saved data.
    fn rollback(&self, data: &RollbackData) -> Result<()>;
    
    /// Cleanup any temporary resources after commit
    fn cleanup(&self, _data: &RollbackData) -> Result<()> {
        Ok(())
    }
}

/// Data saved during prepare for use in rollback
#[derive(Debug, Clone, Default)]
pub struct RollbackData {
    /// Snapshot ID if a snapshot was created
    pub snapshot_id: Option<String>,
    /// Original file content (for small files)
    pub original_content: Option<Vec<u8>>,
    /// Original path if file was moved
    pub original_path: Option<PathBuf>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Record of a single operation within a transaction
struct OpRecord {
    operation: Box<dyn ReversibleOp>,
    rollback_data: Option<RollbackData>,
    executed: bool,
}

/// Manages a stack of operations for a transaction with atomic guarantees.
pub struct TransactionManager {
    ops: Mutex<Vec<OpRecord>>,
    snapshot_manager: Arc<SnapshotManager>,
    policy: RecoveryPolicy,
    status: Mutex<TransactionStatus>,
    transaction_id: String,
}

impl TransactionManager {
    /// Create a new transaction manager
    pub fn new(snapshot_manager: Arc<SnapshotManager>) -> Self {
        Self {
            ops: Mutex::new(Vec::new()),
            snapshot_manager,
            policy: RecoveryPolicy::default(),
            status: Mutex::new(TransactionStatus::Pending),
            transaction_id: Uuid::new_v4().to_string(),
        }
    }
    
    /// Create a transaction manager with custom recovery policy
    pub fn with_policy(snapshot_manager: Arc<SnapshotManager>, policy: RecoveryPolicy) -> Self {
        Self {
            ops: Mutex::new(Vec::new()),
            snapshot_manager,
            policy,
            status: Mutex::new(TransactionStatus::Pending),
            transaction_id: Uuid::new_v4().to_string(),
        }
    }
    
    /// Get the transaction ID
    pub fn id(&self) -> &str {
        &self.transaction_id
    }
    
    /// Get the current status
    pub fn status(&self) -> TransactionStatus {
        self.status.lock().clone()
    }

    /// Add an operation to the transaction.
    pub fn add_op(&self, op: Box<dyn ReversibleOp>) {
        self.ops.lock().push(OpRecord {
            operation: op,
            rollback_data: None,
            executed: false,
        });
    }

    /// Prepare all operations (create snapshots)
    pub fn prepare(&self) -> Result<()> {
        let mut status = self.status.lock();
        if *status != TransactionStatus::Pending {
            return Err(anyhow::anyhow!("Transaction already prepared"));
        }
        
        let mut ops = self.ops.lock();
        for record in ops.iter_mut() {
            let data = record.operation.prepare()?;
            record.rollback_data = Some(data);
        }
        
        *status = TransactionStatus::Prepared;
        Ok(())
    }
    
    /// Execute all operations. On failure, automatically roll back.
    pub fn execute(&self) -> Result<()> {
        {
            let mut status = self.status.lock();
            if *status != TransactionStatus::Prepared {
                return Err(anyhow::anyhow!("Transaction not prepared"));
            }
            *status = TransactionStatus::Executing;
        }
        
        let mut ops = self.ops.lock();
        
        for (i, record) in ops.iter_mut().enumerate() {
            if let Err(e) = record.operation.execute() {
                // Execution failed - rollback all executed operations
                eprintln!("Operation {} failed: {}", record.operation.name(), e);
                
                // Rollback in reverse order
                for record in ops[..=i].iter().rev() {
                    if record.executed {
                        if let Some(ref data) = record.rollback_data {
                            if let Err(rb_err) = record.operation.rollback(data) {
                                eprintln!("Rollback failed for {}: {}", record.operation.name(), rb_err);
                            }
                        }
                    }
                }
                
                *self.status.lock() = TransactionStatus::Failed;
                return Err(e).context(format!("Transaction failed at operation {}", i));
            }
            record.executed = true;
        }
        
        *self.status.lock() = TransactionStatus::Committed;
        Ok(())
    }
    
    /// Execute as a complete atomic transaction (prepare + execute)
    pub fn execute_atomic(&self) -> Result<()> {
        self.prepare()?;
        self.execute()
    }

    /// Rollback all operations in reverse order.
    pub fn rollback(&self) -> Result<()> {
        let ops = self.ops.lock();
        
        for record in ops.iter().rev() {
            if let Some(ref data) = record.rollback_data {
                if let Err(e) = record.operation.rollback(data) {
                    eprintln!("Failed to rollback {}: {}", record.operation.name(), e);
                    // Continue attempting rollback for other ops
                }
            }
        }
        
        *self.status.lock() = TransactionStatus::RolledBack;
        Ok(())
    }
    
    /// Cleanup resources after successful commit
    pub fn cleanup(&self) -> Result<()> {
        let ops = self.ops.lock();
        
        for record in ops.iter() {
            if let Some(ref data) = record.rollback_data {
                if let Err(e) = record.operation.cleanup(data) {
                    eprintln!("Cleanup warning for {}: {}", record.operation.name(), e);
                }
            }
        }
        
        Ok(())
    }
}

/// A file write operation that backs up the original file first.
pub struct FileWriteOp {
    path: PathBuf,
    snapshot_manager: Arc<SnapshotManager>,
    content: Vec<u8>,
}

impl FileWriteOp {
    pub fn new(path: PathBuf, content: Vec<u8>, snapshot_manager: Arc<SnapshotManager>) -> Self {
        Self {
            path,
            snapshot_manager,
            content,
        }
    }
}

impl ReversibleOp for FileWriteOp {
    fn name(&self) -> &str {
        "file_write"
    }
    
    fn prepare(&self) -> Result<RollbackData> {
        let mut data = RollbackData::default();
        
        if self.path.exists() {
            // Create snapshot of existing file
            let snapshot = self.snapshot_manager.create_snapshot(&self.path)?;
            data.snapshot_id = Some(snapshot.id);
            
            // Also store original content in memory for quick rollback
            data.original_content = Some(std::fs::read(&self.path)?);
        }
        
        Ok(data)
    }
    
    fn execute(&self) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        std::fs::write(&self.path, &self.content)?;
        Ok(())
    }

    fn rollback(&self, data: &RollbackData) -> Result<()> {
        if let Some(ref content) = data.original_content {
            std::fs::write(&self.path, content)?;
        } else {
            // File didn't exist before, delete it
            if self.path.exists() {
                std::fs::remove_file(&self.path)?;
            }
        }
        Ok(())
    }
}

/// A file delete operation
pub struct FileDeleteOp {
    path: PathBuf,
    snapshot_manager: Arc<SnapshotManager>,
}

impl FileDeleteOp {
    pub fn new(path: PathBuf, snapshot_manager: Arc<SnapshotManager>) -> Self {
        Self { path, snapshot_manager }
    }
}

impl ReversibleOp for FileDeleteOp {
    fn name(&self) -> &str {
        "file_delete"
    }
    
    fn prepare(&self) -> Result<RollbackData> {
        let mut data = RollbackData::default();
        
        if self.path.exists() {
            let snapshot = self.snapshot_manager.create_snapshot(&self.path)?;
            data.snapshot_id = Some(snapshot.id);
            data.original_content = Some(std::fs::read(&self.path)?);
        }
        
        Ok(data)
    }
    
    fn execute(&self) -> Result<()> {
        if self.path.is_dir() {
            std::fs::remove_dir_all(&self.path)?;
        } else {
            std::fs::remove_file(&self.path)?;
        }
        Ok(())
    }
    
    fn rollback(&self, data: &RollbackData) -> Result<()> {
        if let Some(ref content) = data.original_content {
            std::fs::write(&self.path, content)?;
        }
        Ok(())
    }
}

/// A directory creation operation
pub struct DirectoryCreateOp {
    path: PathBuf,
}

impl DirectoryCreateOp {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl ReversibleOp for DirectoryCreateOp {
    fn name(&self) -> &str {
        "directory_create"
    }
    
    fn prepare(&self) -> Result<RollbackData> {
        Ok(RollbackData::default())
    }
    
    fn execute(&self) -> Result<()> {
        std::fs::create_dir_all(&self.path)?;
        Ok(())
    }
    
    fn rollback(&self, _data: &RollbackData) -> Result<()> {
        if self.path.exists() {
            std::fs::remove_dir_all(&self.path)?;
        }
        Ok(())
    }
}

/// A command execution operation (for shell commands)
pub struct CommandOp {
    command: String,
    args: Vec<String>,
    rollback_command: Option<String>,
    rollback_args: Vec<String>,
}

impl CommandOp {
    pub fn new(command: String, args: Vec<String>, rollback_command: Option<String>, rollback_args: Vec<String>) -> Self {
        Self {
            command,
            args,
            rollback_command,
            rollback_args,
        }
    }
}

impl ReversibleOp for CommandOp {
    fn name(&self) -> &str {
        "command"
    }
    
    fn prepare(&self) -> Result<RollbackData> {
        Ok(RollbackData::default())
    }
    
    fn execute(&self) -> Result<()> {
        let status = std::process::Command::new(&self.command)
            .args(&self.args)
            .status()?;
        
        if !status.success() {
            return Err(anyhow::anyhow!("Command failed with status: {}", status));
        }
        
        Ok(())
    }
    
    fn rollback(&self, _data: &RollbackData) -> Result<()> {
        if let Some(ref rollback_cmd) = self.rollback_command {
            let status = std::process::Command::new(rollback_cmd)
                .args(&self.rollback_args)
                .status()?;
            
            if !status.success() {
                return Err(anyhow::anyhow!("Rollback command failed: {}", status));
            }
        }
        Ok(())
    }
}

/// A database operation
pub struct DatabaseOp {
    db_path: PathBuf,
    operation: DatabaseOperation,
    snapshot_manager: Arc<SnapshotManager>,
}

#[derive(Debug, Clone)]
pub enum DatabaseOperation {
    Write,
    SchemaChange,
    DataMigration,
}

impl DatabaseOp {
    pub fn new(db_path: PathBuf, operation: DatabaseOperation, snapshot_manager: Arc<SnapshotManager>) -> Self {
        Self {
            db_path,
            operation,
            snapshot_manager,
        }
    }
}

impl ReversibleOp for DatabaseOp {
    fn name(&self) -> &str {
        match self.operation {
            DatabaseOperation::Write => "database_write",
            DatabaseOperation::SchemaChange => "database_schema",
            DatabaseOperation::DataMigration => "database_migration",
        }
    }
    
    fn prepare(&self) -> Result<RollbackData> {
        let mut data = RollbackData::default();
        
        if self.db_path.exists() {
            let snapshot = self.snapshot_manager.create_database_snapshot(&self.db_path)?;
            data.snapshot_id = Some(snapshot.id);
        }
        
        Ok(data)
    }
    
    fn execute(&self) -> Result<()> {
        // The actual database operation would be performed by the caller
        // This is a placeholder for the rollback mechanism
        Ok(())
    }
    
    fn rollback(&self, data: &RollbackData) -> Result<()> {
        if let Some(ref snapshot_id) = data.snapshot_id {
            // Restore from snapshot
            // In a real implementation, we'd load the snapshot and restore
            eprintln!("Rolling back database to snapshot: {}", snapshot_id);
        }
        Ok(())
    }
}

/// A memory state operation
pub struct MemoryOp {
    key: String,
    snapshot_manager: Arc<SnapshotManager>,
}

impl MemoryOp {
    pub fn new(key: String, snapshot_manager: Arc<SnapshotManager>) -> Self {
        Self { key, snapshot_manager }
    }
}

impl ReversibleOp for MemoryOp {
    fn name(&self) -> &str {
        "memory_op"
    }
    
    fn prepare(&self) -> Result<RollbackData> {
        // In a real implementation, we'd serialize current memory state
        let data = RollbackData {
            metadata: vec![("key".to_string(), self.key.clone())].into_iter().collect(),
            ..Default::default()
        };
        Ok(data)
    }
    
    fn execute(&self) -> Result<()> {
        // The actual memory operation would be performed by the caller
        Ok(())
    }
    
    fn rollback(&self, _data: &RollbackData) -> Result<()> {
        // Restore memory state from snapshot
        Ok(())
    }
}

/// Builder for creating transactions
pub struct TransactionBuilder {
    operations: Vec<Box<dyn ReversibleOp>>,
    policy: RecoveryPolicy,
    snapshot_manager: Arc<SnapshotManager>,
}

impl TransactionBuilder {
    pub fn new(snapshot_manager: Arc<SnapshotManager>) -> Self {
        Self {
            operations: Vec::new(),
            policy: RecoveryPolicy::default(),
            snapshot_manager,
        }
    }
    
    pub fn with_policy(mut self, policy: RecoveryPolicy) -> Self {
        self.policy = policy;
        self
    }
    
    pub fn write_file(mut self, path: PathBuf, content: Vec<u8>) -> Self {
        self.operations.push(Box::new(FileWriteOp::new(
            path,
            content,
            self.snapshot_manager.clone(),
        )));
        self
    }
    
    pub fn delete_file(mut self, path: PathBuf) -> Self {
        self.operations.push(Box::new(FileDeleteOp::new(
            path,
            self.snapshot_manager.clone(),
        )));
        self
    }
    
    pub fn create_dir(mut self, path: PathBuf) -> Self {
        self.operations.push(Box::new(DirectoryCreateOp::new(path)));
        self
    }
    
    pub fn run_command(mut self, command: String, args: Vec<String>, rollback_cmd: Option<String>, rollback_args: Vec<String>) -> Self {
        self.operations.push(Box::new(CommandOp::new(command, args, rollback_cmd, rollback_args)));
        self
    }
    
    pub fn database_op(mut self, db_path: PathBuf, operation: DatabaseOperation) -> Self {
        self.operations.push(Box::new(DatabaseOp::new(
            db_path,
            operation,
            self.snapshot_manager.clone(),
        )));
        self
    }
    
    pub fn build(self) -> TransactionManager {
        let manager = TransactionManager::with_policy(self.snapshot_manager, self.policy);
        for op in self.operations {
            manager.add_op(op);
        }
        manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_transaction_write_file() {
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("snapshots");
        let snapshot_manager = Arc::new(SnapshotManager::new(store_path).unwrap());
        
        let test_file = dir.path().join("test.txt");
        let content = b"Hello, World!".to_vec();
        
        let tx = TransactionBuilder::new(snapshot_manager)
            .write_file(test_file.clone(), content.clone())
            .build();
        
        tx.execute_atomic().unwrap();
        
        assert!(test_file.exists());
        assert_eq!(std::fs::read(&test_file).unwrap(), content);
    }
    
    #[test]
    fn test_transaction_rollback() {
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("snapshots");
        let snapshot_manager = Arc::new(SnapshotManager::new(store_path).unwrap());
        
        let test_file = dir.path().join("test.txt");
        let original_content = b"Original content".to_vec();
        std::fs::write(&test_file, &original_content).unwrap();
        
        let new_content = b"New content".to_vec();
        
        let tx = TransactionBuilder::new(snapshot_manager.clone())
            .write_file(test_file.clone(), new_content.clone())
            .build();
        
        tx.prepare().unwrap();
        tx.execute().unwrap();
        
        // Content should be new
        assert_eq!(std::fs::read(&test_file).unwrap(), new_content);
        
        // Now rollback
        tx.rollback().unwrap();
        
        // Content should be original
        assert_eq!(std::fs::read(&test_file).unwrap(), original_content);
    }
    
    #[test]
    fn test_transaction_delete_file() {
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("snapshots");
        let snapshot_manager = Arc::new(SnapshotManager::new(store_path).unwrap());
        
        let test_file = dir.path().join("test.txt");
        std::fs::write(&test_file, b"Delete me").unwrap();
        
        let tx = TransactionBuilder::new(snapshot_manager)
            .delete_file(test_file.clone())
            .build();
        
        tx.execute_atomic().unwrap();
        
        assert!(!test_file.exists());
    }
    
    #[test]
    fn test_recovery_policy_default() {
        let policy = RecoveryPolicy::default();
        assert_eq!(policy, RecoveryPolicy::AutomaticRollback);
    }
}
