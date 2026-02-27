//! Prudent Agency: File System Snapshotting
//! Provides mechanism to backup and restore files before critical operations.

use anyhow::{Context, Result};
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct SnapshotManager {
    store_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub id: String,
    pub original_path: PathBuf,
    pub backup_path: PathBuf,
    pub timestamp: chrono::DateTime<Utc>,
}

impl SnapshotManager {
    /// Create a new snapshot manager
    pub fn new(store_path: PathBuf) -> Result<Self> {
        if !store_path.exists() {
            fs::create_dir_all(&store_path)?;
        }
        Ok(Self { store_path })
    }

    /// Create a snapshot of a file or directory
    pub fn create_snapshot(&self, target_path: &Path) -> Result<Snapshot> {
        let id = Uuid::new_v4().to_string();
        let timestamp = Utc::now();
        let backup_dir = self.store_path.join(&id);
        
        fs::create_dir_all(&backup_dir)?;
        
        // Simple copy strategy for MVP. 
        // Production should use Copy-on-Write (reflink) if supported by FS.
        if target_path.is_dir() {
            copy_dir_recursive(target_path, &backup_dir.join(target_path.file_name().unwrap()))?;
        } else {
            fs::copy(target_path, backup_dir.join(target_path.file_name().unwrap()))?;
        }

        Ok(Snapshot {
            id,
            original_path: target_path.to_path_buf(),
            backup_path: backup_dir,
            timestamp,
        })
    }

    /// Restore a snapshot
    pub fn restore_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        if snapshot.original_path.exists() {
            if snapshot.original_path.is_dir() {
                fs::remove_dir_all(&snapshot.original_path)?;
            } else {
                fs::remove_file(&snapshot.original_path)?;
            }
        }

        let source = snapshot.backup_path.join(snapshot.original_path.file_name().unwrap());
        
        if source.is_dir() {
            copy_dir_recursive(&source, &snapshot.original_path)?;
        } else {
            fs::copy(&source, &snapshot.original_path)?;
        }

        Ok(())
    }
    
    /// Delete a snapshot to free space
    pub fn delete_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        fs::remove_dir_all(&snapshot.backup_path)?;
        Ok(())
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
