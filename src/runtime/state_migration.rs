//! State Migration for Hot-Swap - Transfer Memory State Between Module Versions
//!
//! This module provides state migration capabilities for hot-swapping,
//! allowing memory state to be transferred between different module versions.
//!
//! Migration Strategies:
//! - Direct: State schema unchanged, direct copy
//! - Transform: Apply transformation functions
//! - Progressive: Migrate through intermediate versions
//! - Snapshot: Save and restore via snapshot

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Migration strategy type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationStrategy {
    /// Direct copy (no transformation)
    Direct,
    /// Transform via migration function
    Transform(TransformConfig),
    /// Progressive migration through versions
    Progressive(Vec<String>),
    /// Snapshot-based migration
    Snapshot,
}

/// Transform configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformConfig {
    /// Transform function name
    pub function: String,
    /// Transform parameters
    pub params: HashMap<String, serde_json::Value>,
    /// Whether the transform is reversible
    pub reversible: bool,
}

/// State schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSchema {
    /// Schema version
    pub version: String,
    /// Field definitions
    pub fields: Vec<FieldDef>,
    /// Invariants (constraints that must be preserved)
    pub invariants: Vec<String>,
}

/// Field definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    /// Field name
    pub name: String,
    /// Field type
    pub field_type: FieldType,
    /// Whether the field is required
    pub required: bool,
    /// Default value if not present
    pub default: Option<serde_json::Value>,
    /// Migration hint
    pub migration_hint: Option<String>,
}

/// Field type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldType {
    /// Primitive integer
    Integer { bits: u8, signed: bool },
    /// Primitive float
    Float { bits: u8 },
    /// Boolean
    Boolean,
    /// String
    String,
    /// Bytes
    Bytes,
    /// Array of type
    Array { element: Box<FieldType> },
    /// Map from key to value
    Map {
        key: Box<FieldType>,
        value: Box<FieldType>,
    },
    /// Optional of type
    Optional { inner: Box<FieldType> },
    /// Custom type name
    Custom { name: String },
}

/// State snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// Snapshot ID
    pub id: String,
    /// Module ID
    pub module_id: String,
    /// Module version
    pub module_version: String,
    /// Schema version
    pub schema_version: String,
    /// Snapshot timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// State data
    pub data: serde_json::Value,
    /// State metadata
    pub metadata: HashMap<String, String>,
    /// Checksum for integrity
    pub checksum: String,
}

/// Migration result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResult {
    /// Success flag
    pub success: bool,
    /// Source version
    pub from_version: String,
    /// Target version
    pub to_version: String,
    /// Migrated state
    pub state: Option<serde_json::Value>,
    /// Migration warnings
    pub warnings: Vec<String>,
    /// Fields migrated
    pub fields_migrated: usize,
    /// Fields skipped
    pub fields_skipped: usize,
    /// Fields added
    pub fields_added: usize,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

/// Migration plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    /// Plan ID
    pub id: String,
    /// Source schema
    pub source_schema: StateSchema,
    /// Target schema
    pub target_schema: StateSchema,
    /// Migration steps
    pub steps: Vec<MigrationStep>,
    /// Estimated duration
    pub estimated_duration_ms: u64,
    /// Risk level (low/medium/high)
    pub risk_level: String,
}

/// Migration step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationStep {
    /// Step ID
    pub id: String,
    /// Step description
    pub description: String,
    /// Field to migrate
    pub field: String,
    /// Transform to apply
    pub transform: Option<TransformConfig>,
    /// Dependencies on other steps
    pub dependencies: Vec<String>,
    /// Whether this step is reversible
    pub reversible: bool,
}

/// State migrator
pub struct StateMigrator {
    /// Registered schemas
    schemas: Arc<RwLock<HashMap<String, StateSchema>>>,
    /// Migration functions
    transforms: Arc<RwLock<HashMap<String, TransformFn>>>,
    /// Migration history
    history: Arc<RwLock<Vec<MigrationRecord>>>,
}

/// Transform function type
type TransformFn = Box<
    dyn Fn(&serde_json::Value, &HashMap<String, serde_json::Value>) -> Result<serde_json::Value>
        + Send
        + Sync,
>;

/// Migration record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRecord {
    /// Record ID
    pub id: String,
    /// Module ID
    pub module_id: String,
    /// From version
    pub from_version: String,
    /// To version
    pub to_version: String,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Result
    pub success: bool,
    /// Duration
    pub duration_ms: u64,
}

impl StateMigrator {
    /// Create a new state migrator
    pub fn new() -> Self {
        Self {
            schemas: Arc::new(RwLock::new(HashMap::new())),
            transforms: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a schema
    pub async fn register_schema(&self, version: &str, schema: StateSchema) {
        self.schemas
            .write()
            .await
            .insert(version.to_string(), schema);
        info!("Registered schema version: {}", version);
    }

    /// Register a transform function
    pub async fn register_transform(&self, name: &str, transform: TransformFn) {
        self.transforms
            .write()
            .await
            .insert(name.to_string(), transform);
        debug!("Registered transform function: {}", name);
    }

    /// Create a state snapshot
    pub async fn create_snapshot(
        &self,
        module_id: &str,
        module_version: &str,
        schema_version: &str,
        data: serde_json::Value,
    ) -> Result<StateSnapshot> {
        let checksum = self.compute_checksum(&data);

        Ok(StateSnapshot {
            id: format!("snap-{}", uuid::Uuid::new_v4()),
            module_id: module_id.to_string(),
            module_version: module_version.to_string(),
            schema_version: schema_version.to_string(),
            timestamp: chrono::Utc::now(),
            data,
            metadata: HashMap::new(),
            checksum,
        })
    }

    /// Compute checksum for state data
    fn compute_checksum(&self, data: &serde_json::Value) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        data.to_string().hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Verify snapshot integrity
    pub fn verify_snapshot(&self, snapshot: &StateSnapshot) -> Result<bool> {
        let expected = self.compute_checksum(&snapshot.data);
        Ok(expected == snapshot.checksum)
    }

    /// Plan a migration
    pub async fn plan_migration(
        &self,
        from_version: &str,
        to_version: &str,
        source_schema: StateSchema,
        target_schema: StateSchema,
    ) -> Result<MigrationPlan> {
        let mut steps = Vec::new();
        let mut risk_level = "low".to_string();

        // Compare schemas and generate steps
        let source_fields_map: HashMap<&str, &FieldDef> = source_schema
            .fields
            .iter()
            .map(|f| (f.name.as_str(), f))
            .collect();

        for target_field in &target_schema.fields {
            if let Some(source_field) = source_fields_map.get(target_field.name.as_str()) {
                // Field exists in both - check if transformation needed
                if self.needs_transform(source_field, target_field) {
                    steps.push(MigrationStep {
                        id: format!("step-{}", uuid::Uuid::new_v4()),
                        description: format!("Transform field: {}", target_field.name),
                        field: target_field.name.clone(),
                        transform: target_field.migration_hint.as_ref().map(|hint| {
                            TransformConfig {
                                function: hint.clone(),
                                params: HashMap::new(),
                                reversible: true,
                            }
                        }),
                        dependencies: Vec::new(),
                        reversible: true,
                    });

                    risk_level = "medium".to_string();
                }
            } else {
                // New field - add with default
                if target_field.required {
                    steps.push(MigrationStep {
                        id: format!("step-{}", uuid::Uuid::new_v4()),
                        description: format!("Add required field: {}", target_field.name),
                        field: target_field.name.clone(),
                        transform: None,
                        dependencies: Vec::new(),
                        reversible: true,
                    });

                    risk_level = "medium".to_string();
                }
            }
        }

        // Check for removed fields
        for source_field in &source_schema.fields {
            if !target_schema
                .fields
                .iter()
                .any(|f| f.name == source_field.name)
            {
                steps.push(MigrationStep {
                    id: format!("step-{}", uuid::Uuid::new_v4()),
                    description: format!("Remove field: {}", source_field.name),
                    field: source_field.name.clone(),
                    transform: None,
                    dependencies: Vec::new(),
                    reversible: false, // Data loss
                });

                risk_level = "high".to_string();
            }
        }

        let estimated_duration = steps.len() as u64 * 10; // 10ms per step

        Ok(MigrationPlan {
            id: format!("plan-{}", uuid::Uuid::new_v4()),
            source_schema,
            target_schema,
            steps,
            estimated_duration_ms: estimated_duration,
            risk_level,
        })
    }

    /// Check if transformation is needed
    fn needs_transform(&self, source: &FieldDef, target: &FieldDef) -> bool {
        // Check type change
        if std::mem::discriminant(&source.field_type) != std::mem::discriminant(&target.field_type)
        {
            return true;
        }

        // Check required change
        if source.required != target.required {
            return true;
        }

        false
    }

    /// Execute migration
    pub async fn migrate(
        &self,
        snapshot: &StateSnapshot,
        plan: &MigrationPlan,
    ) -> Result<MigrationResult> {
        let start_time = std::time::Instant::now();

        info!(
            "Migrating state from version {} to {}",
            snapshot.schema_version, plan.target_schema.version
        );

        let mut state = snapshot.data.clone();
        let mut warnings = Vec::new();
        let mut fields_migrated = 0;
        let mut fields_skipped = 0;
        let mut fields_added = 0;

        // Build field map
        let source_fields: HashMap<&str, &FieldDef> = plan
            .source_schema
            .fields
            .iter()
            .map(|f| (f.name.as_str(), f))
            .collect();

        let target_fields: HashMap<&str, &FieldDef> = plan
            .target_schema
            .fields
            .iter()
            .map(|f| (f.name.as_str(), f))
            .collect();

        // Apply migration steps
        for step in &plan.steps {
            match self
                .apply_migration_step(&mut state, step, &source_fields, &target_fields)
                .await
            {
                Ok(applied) => {
                    if applied {
                        fields_migrated += 1;
                    } else {
                        fields_skipped += 1;
                    }
                }
                Err(e) => {
                    warnings.push(format!("Step {} failed: {}", step.id, e));
                }
            }
        }

        // Ensure all required fields are present
        if let serde_json::Value::Object(ref mut map) = state {
            for field in &plan.target_schema.fields {
                if field.required && !map.contains_key(&field.name) {
                    if let Some(ref default) = field.default {
                        map.insert(field.name.clone(), default.clone());
                        fields_added += 1;
                    } else {
                        warnings.push(format!("Missing required field: {}", field.name));
                    }
                }
            }
        }

        let duration = start_time.elapsed();

        // Record migration
        let record = MigrationRecord {
            id: format!("rec-{}", uuid::Uuid::new_v4()),
            module_id: snapshot.module_id.clone(),
            from_version: snapshot.schema_version.clone(),
            to_version: plan.target_schema.version.clone(),
            timestamp: chrono::Utc::now(),
            success: warnings.is_empty(),
            duration_ms: duration.as_millis() as u64,
        };

        self.history.write().await.push(record);

        Ok(MigrationResult {
            success: warnings.is_empty(),
            from_version: snapshot.schema_version.clone(),
            to_version: plan.target_schema.version.clone(),
            state: Some(state),
            warnings,
            fields_migrated,
            fields_skipped,
            fields_added,
            duration_ms: duration.as_millis() as u64,
        })
    }

    /// Apply a single migration step
    async fn apply_migration_step(
        &self,
        state: &mut serde_json::Value,
        step: &MigrationStep,
        source_fields: &HashMap<&str, &FieldDef>,
        target_fields: &HashMap<&str, &FieldDef>,
    ) -> Result<bool> {
        if let serde_json::Value::Object(ref mut map) = state {
            // Check if field exists
            if let Some(value) = map.get(&step.field).cloned() {
                // Apply transform if specified
                if let Some(ref transform) = step.transform {
                    let transforms = self.transforms.read().await;

                    if let Some(transform_fn) = transforms.get(&transform.function) {
                        let new_value = transform_fn(&value, &transform.params)?;
                        map.insert(step.field.clone(), new_value);
                        return Ok(true);
                    } else {
                        // No transform function registered - use default transforms
                        let new_value = self.apply_default_transform(
                            &value,
                            &step.field,
                            source_fields,
                            target_fields,
                        )?;
                        map.insert(step.field.clone(), new_value);
                        return Ok(true);
                    }
                }

                // No transform needed
                return Ok(false);
            } else {
                // Field doesn't exist - add default if required
                if let Some(field_def) = target_fields.get(step.field.as_str()) {
                    if let Some(ref default) = field_def.default {
                        map.insert(step.field.clone(), default.clone());
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    /// Apply default transform based on type change
    fn apply_default_transform(
        &self,
        value: &serde_json::Value,
        field_name: &str,
        source_fields: &HashMap<&str, &FieldDef>,
        target_fields: &HashMap<&str, &FieldDef>,
    ) -> Result<serde_json::Value> {
        let source_field = source_fields.get(field_name);
        let target_field = target_fields.get(field_name);

        if source_field.is_none() || target_field.is_none() {
            return Ok(value.clone());
        }

        let source = source_field.unwrap();
        let target = target_field.unwrap();

        // Handle common type conversions
        match (&source.field_type, &target.field_type) {
            // Integer to String
            (FieldType::Integer { .. }, FieldType::String) => match value {
                serde_json::Value::Number(n) => Ok(serde_json::Value::String(n.to_string())),
                _ => Ok(value.clone()),
            },

            // String to Integer
            (FieldType::String, FieldType::Integer { .. }) => match value {
                serde_json::Value::String(s) => {
                    if let Ok(n) = s.parse::<i64>() {
                        Ok(serde_json::Value::Number(n.into()))
                    } else {
                        bail!("Cannot convert '{}' to integer", s);
                    }
                }
                _ => Ok(value.clone()),
            },

            // Array to Map (if elements have key)
            (FieldType::Array { .. }, FieldType::Map { .. }) => match value {
                serde_json::Value::Array(arr) => {
                    let mut map = serde_json::Map::new();
                    for item in arr {
                        if let serde_json::Value::Object(obj) = item {
                            if let Some(key) = obj.get("key").and_then(|k| k.as_str()) {
                                map.insert(key.to_string(), item.clone());
                            }
                        }
                    }
                    Ok(serde_json::Value::Object(map))
                }
                _ => Ok(value.clone()),
            },

            // Default: keep as is
            _ => Ok(value.clone()),
        }
    }

    /// Get migration history
    pub async fn get_history(&self) -> Vec<MigrationRecord> {
        self.history.read().await.clone()
    }

    /// Get migration statistics
    pub async fn get_stats(&self) -> MigrationStats {
        let history = self.history.read().await;

        let total = history.len();
        let successful = history.iter().filter(|r| r.success).count();
        let total_duration = history.iter().map(|r| r.duration_ms).sum();

        MigrationStats {
            total_migrations: total,
            successful,
            failed: total - successful,
            total_duration_ms: total_duration,
            average_duration_ms: if total > 0 {
                total_duration / total as u64
            } else {
                0
            },
        }
    }
}

/// Migration statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationStats {
    pub total_migrations: usize,
    pub successful: usize,
    pub failed: usize,
    pub total_duration_ms: u64,
    pub average_duration_ms: u64,
}

impl Default for StateMigrator {
    fn default() -> Self {
        Self::new()
    }
}

/// Common transform functions
pub mod transforms {
    use super::*;
    use std::collections::HashMap;

    /// Identity transform (no-op)
    pub fn identity() -> TransformFn {
        Box::new(|value, _| Ok(value.clone()))
    }

    /// Integer to string transform
    pub fn int_to_string() -> TransformFn {
        Box::new(|value, _| match value {
            serde_json::Value::Number(n) => Ok(serde_json::Value::String(n.to_string())),
            _ => Ok(value.clone()),
        })
    }

    /// String to integer transform
    pub fn string_to_int() -> TransformFn {
        Box::new(|value, _| match value {
            serde_json::Value::String(s) => {
                let n: i64 = s.parse().context("Failed to parse integer")?;
                Ok(serde_json::Value::Number(n.into()))
            }
            _ => Ok(value.clone()),
        })
    }

    /// Flatten nested object
    pub fn flatten(separator: &str) -> TransformFn {
        let sep = separator.to_string();
        Box::new(move |value, _| match value {
            serde_json::Value::Object(obj) => {
                let mut flat = serde_json::Map::new();
                flatten_object(&mut flat, "", obj, &sep);
                Ok(serde_json::Value::Object(flat))
            }
            _ => Ok(value.clone()),
        })
    }

    fn flatten_object(
        flat: &mut serde_json::Map<String, serde_json::Value>,
        prefix: &str,
        obj: &serde_json::Map<String, serde_json::Value>,
        sep: &str,
    ) {
        for (key, value) in obj {
            let new_key = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{}{}{}", prefix, sep, key)
            };

            match value {
                serde_json::Value::Object(nested) => {
                    flatten_object(flat, &new_key, nested, sep);
                }
                _ => {
                    flat.insert(new_key, value.clone());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_snapshot() {
        let migrator = StateMigrator::new();

        let data = serde_json::json!({
            "counter": 42,
            "name": "test",
            "enabled": true
        });

        let snapshot = migrator
            .create_snapshot("module-1", "1.0.0", "1.0.0", data.clone())
            .await
            .unwrap();

        assert!(snapshot.id.starts_with("snap-"));
        assert!(migrator.verify_snapshot(&snapshot).unwrap());
    }

    #[tokio::test]
    async fn test_plan_migration() {
        let migrator = StateMigrator::new();

        let source_schema = StateSchema {
            version: "1.0.0".to_string(),
            fields: vec![FieldDef {
                name: "counter".to_string(),
                field_type: FieldType::Integer {
                    bits: 32,
                    signed: true,
                },
                required: true,
                default: None,
                migration_hint: None,
            }],
            invariants: Vec::new(),
        };

        let target_schema = StateSchema {
            version: "1.1.0".to_string(),
            fields: vec![
                FieldDef {
                    name: "counter".to_string(),
                    field_type: FieldType::Integer {
                        bits: 64,
                        signed: true,
                    },
                    required: true,
                    default: None,
                    migration_hint: None,
                },
                FieldDef {
                    name: "name".to_string(),
                    field_type: FieldType::String,
                    required: false,
                    default: Some(serde_json::Value::String("unnamed".to_string())),
                    migration_hint: None,
                },
            ],
            invariants: Vec::new(),
        };

        let plan = migrator
            .plan_migration("1.0.0", "1.1.0", source_schema, target_schema)
            .await
            .unwrap();

        assert!(plan.steps.iter().any(|s| s.field == "name"));
    }

    #[tokio::test]
    async fn test_transforms() {
        let transform = transforms::int_to_string();

        let value = serde_json::json!(42);
        let result = transform(&value, &HashMap::new()).unwrap();

        assert_eq!(result, serde_json::json!("42"));
    }
}
