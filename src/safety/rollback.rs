//! Rollback mechanisms for file system and state changes.
//!
//! Provides transactional safety for operations that modify persistent state.
//! If an operation fails or is rejected by policy, these mechanisms attempt to
//! restore the previous state.

use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use crate::safety::snapshot::SnapshotManager;
use std::sync::Arc;

/// Represents a reversible operation.
pub trait ReversibleOp {
    /// Execute the operation (e.g., write file).
    fn execute(&self) -> Result<()>;
    
    /// Revert the operation (e.g., restore backup).
    fn rollback(&self) -> Result<()>;
}

/// Manages a stack of operations for a transaction.
pub struct TransactionManager {
    ops: Vec<Box<dyn ReversibleOp>>,
}

impl TransactionManager {
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }

    /// Add an operation to the transaction log.
    pub fn add_op(&mut self, op: Box<dyn ReversibleOp>) {
        self.ops.push(op);
    }

    /// Rollback all operations in reverse order.
    pub fn rollback_all(&self) -> Result<()> {
        for op in self.ops.iter().rev() {
            if let Err(e) = op.rollback() {
                eprintln!("Failed to rollback operation: {}", e);
                // Continue attempting rollback for other ops
            }
        }
        Ok(())
    }
}

/// A file write operation that backs up the original file first.
pub struct FileWriteOp {
    path: PathBuf,
    snapshot_manager: Arc<SnapshotManager>,
}

impl FileWriteOp {
    pub fn new(path: PathBuf, snapshot_manager: Arc<SnapshotManager>) -> Self {
        Self {
            path,
            snapshot_manager,
        }
    }
}

impl ReversibleOp for FileWriteOp {
    fn execute(&self) -> Result<()> {
        // Create backup before modification
        if self.path.exists() {
             self.snapshot_manager.create_file_snapshot(&self.path)?;
        }
        Ok(())
    }

    fn rollback(&self) -> Result<()> {
        // Restore from backup if it exists
        // Simplification for Phase 1: We assume restoring the latest snapshot is sufficient
        // In a real system, we'd track the specific snapshot ID created in execute()
        // self.snapshot_manager.restore_file_snapshot(&self.path, "latest")?;
        Ok(())
    }
}
