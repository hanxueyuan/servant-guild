//! State Migration Module
//!
//! Provides state migration capabilities for ServantGuild,
//! enabling safe state transitions between versions.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// State snapshot for migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// Snapshot ID
    pub id: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Version
    pub version: String,
    /// State data
    pub data: HashMap<String, serde_json::Value>,
    /// File paths included
    pub files: Vec<PathBuf>,
}

impl StateSnapshot {
    /// Create a new state snapshot
    pub fn new(version: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            version,
            data: HashMap::new(),
            files: Vec::new(),
        }
    }

    /// Add data to the snapshot
    pub fn with_data(mut self, key: String, value: serde_json::Value) -> Self {
        self.data.insert(key, value);
        self
    }

    /// Add a file to the snapshot
    pub fn with_file(mut self, path: PathBuf) -> Self {
        self.files.push(path);
        self
    }
}

/// Migration plan for state transitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    /// Plan ID
    pub id: String,
    /// Source version
    pub from_version: String,
    /// Target version
    pub to_version: String,
    /// Migration steps
    pub steps: Vec<MigrationStep>,
    /// Created at
    pub created_at: DateTime<Utc>,
}

/// Migration step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationStep {
    /// Step number
    pub step: u32,
    /// Description
    pub description: String,
    /// Action type
    pub action: MigrationAction,
    /// Whether the step is reversible
    pub reversible: bool,
}

/// Migration action type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationAction {
    /// Transform data
    TransformData { key: String, transform: String },
    /// Rename field
    RenameField { from: String, to: String },
    /// Add field
    AddField { key: String, default: serde_json::Value },
    /// Remove field
    RemoveField { key: String },
    /// Execute script
    ExecuteScript { script: String },
}

/// State migrator
pub struct StateMigrator {
    /// Migration plans
    plans: Vec<MigrationPlan>,
}

impl StateMigrator {
    /// Create a new state migrator
    pub fn new() -> Self {
        Self {
            plans: Vec::new(),
        }
    }

    /// Create a snapshot
    pub async fn create_snapshot(&self, version: &str) -> Result<StateSnapshot> {
        Ok(StateSnapshot::new(version.to_string()))
    }

    /// Restore from a snapshot
    pub async fn restore_snapshot(&self, _snapshot: &StateSnapshot) -> Result<()> {
        // Implementation would restore state from the snapshot
        Ok(())
    }

    /// Create a migration plan
    pub fn create_plan(&mut self, from_version: String, to_version: String) -> MigrationPlan {
        let plan = MigrationPlan {
            id: uuid::Uuid::new_v4().to_string(),
            from_version,
            to_version,
            steps: Vec::new(),
            created_at: Utc::now(),
        };
        self.plans.push(plan.clone());
        plan
    }

    /// Execute a migration plan
    pub async fn execute_plan(&self, _plan: &MigrationPlan, _snapshot: &StateSnapshot) -> Result<StateSnapshot> {
        // Implementation would execute the migration steps
        Ok(StateSnapshot::new("migrated".to_string()))
    }

    /// Get all plans
    pub fn get_plans(&self) -> &[MigrationPlan] {
        &self.plans
    }
}

impl Default for StateMigrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_snapshot_creation() {
        let snapshot = StateSnapshot::new("1.0.0".to_string());
        assert_eq!(snapshot.version, "1.0.0");
    }

    #[tokio::test]
    async fn test_create_snapshot() {
        let migrator = StateMigrator::new();
        let snapshot = migrator.create_snapshot("1.0.0").await.unwrap();
        assert_eq!(snapshot.version, "1.0.0");
    }

    #[test]
    fn test_create_plan() {
        let mut migrator = StateMigrator::new();
        let plan = migrator.create_plan("1.0.0".to_string(), "2.0.0".to_string());
        assert_eq!(plan.from_version, "1.0.0");
        assert_eq!(plan.to_version, "2.0.0");
    }
}
