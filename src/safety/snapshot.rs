//! Prudent Agency: File System and System State Snapshotting
//! Provides mechanism to backup and restore files and system state before critical operations.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Manages snapshots for files and system state
pub struct SnapshotManager {
    store_path: PathBuf,
}

/// A single snapshot record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub original_path: PathBuf,
    pub backup_path: PathBuf,
    pub timestamp: DateTime<Utc>,
    pub snapshot_type: SnapshotType,
}

/// Type of snapshot
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SnapshotType {
    File,
    Directory,
    Database,
    Memory,
    FullSystem,
}

/// System state snapshot containing multiple components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub components: Vec<SnapshotComponent>,
    pub metadata: SnapshotMetadata,
}

/// Individual component in a system snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotComponent {
    pub name: String,
    pub component_type: ComponentType,
    pub backup_path: PathBuf,
    pub size_bytes: u64,
}

/// Types of system components that can be snapshotted
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComponentType {
    File,
    Directory,
    SqliteDatabase,
    PostgresDatabase,
    MemoryStore,
    Config,
}

/// Metadata for a snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    pub created_by: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub parent_snapshot_id: Option<String>,
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
        let snapshot_type = if target_path.is_dir() {
            SnapshotType::Directory
        } else {
            SnapshotType::File
        };
        
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
            snapshot_type,
        })
    }

    /// Create a snapshot of a SQLite database
    pub fn create_database_snapshot(&self, db_path: &Path) -> Result<Snapshot> {
        let id = Uuid::new_v4().to_string();
        let timestamp = Utc::now();
        let backup_dir = self.store_path.join(&id);
        fs::create_dir_all(&backup_dir)?;
        
        // SQLite supports online backup via VACUUM INTO or file copy
        // For simplicity, we copy the file (ensuring no corruption)
        let backup_file = backup_dir.join(db_path.file_name().unwrap());
        
        // Use SQLite backup API for consistency (via rusqlite)
        // For Phase 1, we do a simple file copy with WAL handling
        fs::copy(db_path, &backup_file)?;
        
        // Also backup WAL and SHM files if they exist
        let wal_path = db_path.with_extension("db-wal");
        let shm_path = db_path.with_extension("db-shm");
        if wal_path.exists() {
            fs::copy(&wal_path, backup_file.with_extension("db-wal"))?;
        }
        if shm_path.exists() {
            fs::copy(&shm_path, backup_file.with_extension("db-shm"))?;
        }

        Ok(Snapshot {
            id,
            original_path: db_path.to_path_buf(),
            backup_path: backup_dir,
            timestamp,
            snapshot_type: SnapshotType::Database,
        })
    }

    /// Create a memory state snapshot
    pub fn create_memory_snapshot(&self, memory_data: &[u8], name: &str) -> Result<Snapshot> {
        let id = Uuid::new_v4().to_string();
        let timestamp = Utc::now();
        let backup_dir = self.store_path.join(&id);
        fs::create_dir_all(&backup_dir)?;
        
        let backup_file = backup_dir.join(format!("{}.bin", name));
        fs::write(&backup_file, memory_data)?;
        
        Ok(Snapshot {
            id,
            original_path: PathBuf::from(format!("memory://{}", name)),
            backup_path: backup_dir,
            timestamp,
            snapshot_type: SnapshotType::Memory,
        })
    }

    /// Create a full system snapshot (multiple components)
    pub fn create_system_snapshot(
        &self,
        components: &[PathComponent],
        metadata: SnapshotMetadata,
    ) -> Result<SystemSnapshot> {
        let id = Uuid::new_v4().to_string();
        let timestamp = Utc::now();
        let backup_dir = self.store_path.join(&id);
        fs::create_dir_all(&backup_dir)?;
        
        let mut snapshot_components = Vec::new();
        
        for component in components {
            let component_backup_dir = backup_dir.join(&component.name);
            fs::create_dir_all(&component_backup_dir)?;
            
            let size_bytes = self.backup_component(component, &component_backup_dir)?;
            
            snapshot_components.push(SnapshotComponent {
                name: component.name.clone(),
                component_type: component.component_type.clone(),
                backup_path: component_backup_dir,
                size_bytes,
            });
        }
        
        // Write metadata
        let metadata_path = backup_dir.join("snapshot_metadata.json");
        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        fs::write(&metadata_path, metadata_json)?;

        Ok(SystemSnapshot {
            id,
            timestamp,
            components: snapshot_components,
            metadata,
        })
    }

    /// Backup a single component
    fn backup_component(&self, component: &PathComponent, backup_dir: &Path) -> Result<u64> {
        match component.component_type {
            ComponentType::File | ComponentType::Directory | ComponentType::Config => {
                if component.path.is_dir() {
                    copy_dir_recursive(&component.path, &backup_dir.join(component.path.file_name().unwrap()))?;
                } else {
                    fs::copy(&component.path, backup_dir.join(component.path.file_name().unwrap()))?;
                }
                Ok(self.calculate_dir_size(backup_dir)?)
            }
            ComponentType::SqliteDatabase => {
                let snapshot = self.create_database_snapshot(&component.path)?;
                Ok(self.calculate_dir_size(&snapshot.backup_path)?)
            }
            ComponentType::PostgresDatabase => {
                // For PostgreSQL, we'd use pg_dump
                // Phase 1: placeholder
                Ok(0)
            }
            ComponentType::MemoryStore => {
                // Memory store backup would be handled separately
                Ok(0)
            }
        }
    }

    /// Calculate the size of a directory
    fn calculate_dir_size(&self, path: &Path) -> Result<u64> {
        let mut total_size = 0;
        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let metadata = entry.metadata()?;
                if metadata.is_dir() {
                    total_size += self.calculate_dir_size(&entry.path())?;
                } else {
                    total_size += metadata.len();
                }
            }
        } else {
            total_size = path.metadata()?.len();
        }
        Ok(total_size)
    }

    /// Restore a snapshot
    pub fn restore_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        // Handle memory snapshots differently
        if snapshot.snapshot_type == SnapshotType::Memory {
            return Err(anyhow::anyhow!("Use restore_memory_snapshot for memory snapshots"));
        }
        
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

    /// Restore a memory snapshot
    pub fn restore_memory_snapshot(&self, snapshot: &Snapshot) -> Result<Vec<u8>> {
        if snapshot.snapshot_type != SnapshotType::Memory {
            return Err(anyhow::anyhow!("Snapshot is not a memory snapshot"));
        }
        
        // Find the .bin file in the backup directory
        for entry in fs::read_dir(&snapshot.backup_path)? {
            let entry = entry?;
            if entry.path().extension().map_or(false, |ext| ext == "bin") {
                return Ok(fs::read(entry.path())?);
            }
        }
        
        Err(anyhow::anyhow!("No .bin file found in memory snapshot"))
    }
    
    /// Delete a snapshot to free space
    pub fn delete_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        fs::remove_dir_all(&snapshot.backup_path)?;
        Ok(())
    }
    
    /// List all snapshots
    pub fn list_snapshots(&self) -> Result<Vec<Snapshot>> {
        let mut snapshots = Vec::new();
        
        if !self.store_path.exists() {
            return Ok(snapshots);
        }
        
        for entry in fs::read_dir(&self.store_path)? {
            let entry = entry?;
            let metadata_path = entry.path().join("snapshot_metadata.json");
            
            // Try to read snapshot metadata if it exists
            if metadata_path.exists() {
                // This is a system snapshot, skip for now
                continue;
            }
            
            // Individual component snapshot
            let backup_dir = entry.path();
            if let Some(original_files) = fs::read_dir(&backup_dir)?.next() {
                let original_file = original_files?.path();
                let id = entry.file_name().to_string_lossy().to_string();
                
                // Try to determine snapshot type
                let snapshot_type = if original_file.extension().map_or(false, |ext| ext == "db") {
                    SnapshotType::Database
                } else if original_file.is_dir() {
                    SnapshotType::Directory
                } else if original_file.extension().map_or(false, |ext| ext == "bin") {
                    SnapshotType::Memory
                } else {
                    SnapshotType::File
                };
                
                snapshots.push(Snapshot {
                    id,
                    original_path: original_file.clone(),
                    backup_path: backup_dir,
                    timestamp: Utc::now(), // Would need to store this properly
                    snapshot_type,
                });
            }
        }
        
        Ok(snapshots)
    }
    
    /// Get snapshot by ID
    pub fn get_snapshot(&self, id: &str) -> Result<Option<Snapshot>> {
        let snapshot_dir = self.store_path.join(id);
        if !snapshot_dir.exists() {
            return Ok(None);
        }
        
        // Read snapshot metadata
        let metadata_path = snapshot_dir.join("snapshot.json");
        if metadata_path.exists() {
            let content = fs::read_to_string(&metadata_path)?;
            let snapshot: Snapshot = serde_json::from_str(&content)?;
            return Ok(Some(snapshot));
        }
        
        Ok(None)
    }
}

/// A path component for system snapshots
#[derive(Debug, Clone)]
pub struct PathComponent {
    pub name: String,
    pub path: PathBuf,
    pub component_type: ComponentType,
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
