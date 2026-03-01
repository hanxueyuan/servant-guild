//! End-to-End Test for ServantGuild Phase 1 Safety Core
//!
//! This test verifies the complete flow:
//! 1. Audit logging with tamper-evident hash chain
//! 2. Snapshot management for file and system state
//! 3. Rollback mechanism with atomic transactions
//! 4. Integration with the Safety Core

use std::path::PathBuf;
use std::sync::Arc;
use tempfile::tempdir;

// Import the safety modules
use zeroclaw::config::AuditConfig;
use zeroclaw::safety::audit::{AuditEvent, AuditEventType, AuditLogger, ChainVerificationResult};
use zeroclaw::safety::rollback::{
    FileDeleteOp, FileWriteOp, RecoveryPolicy, ReversibleOp, RollbackData, TransactionBuilder,
    TransactionManager,
};
use zeroclaw::safety::snapshot::{
    ComponentType, PathComponent, SnapshotManager, SnapshotMetadata, SnapshotType,
};

// -----------------------------------------------------------------------------
// Test 1: Audit System with Hash Chain
// -----------------------------------------------------------------------------
#[test]
fn test_audit_hash_chain_integrity() {
    let dir = tempdir().expect("Failed to create temp dir");
    let config = AuditConfig::default();
    let logger =
        AuditLogger::new(config, dir.path().to_path_buf()).expect("Failed to create audit logger");

    // Log multiple events
    let events = vec![
        AuditEvent::new(AuditEventType::CommandExecution)
            .with_actor("test".to_string(), Some("user1".to_string()), None)
            .with_action("ls -la".to_string(), "low".to_string(), true, true),
        AuditEvent::new(AuditEventType::FileAccess)
            .with_actor("test".to_string(), Some("user1".to_string()), None)
            .with_resource_action("read".to_string(), "/etc/passwd".to_string()),
        AuditEvent::new(AuditEventType::ConfigChange)
            .with_actor("test".to_string(), None, Some("admin".to_string()))
            .with_action(
                "update_settings".to_string(),
                "medium".to_string(),
                true,
                true,
            ),
    ];

    for event in &events {
        logger.log(event).expect("Failed to log event");
    }

    // Verify chain integrity
    let result = logger.verify_chain().expect("Failed to verify chain");
    assert!(result.valid, "Audit chain should be valid");
    assert_eq!(result.total_events, 3);
    assert!(
        result.errors.is_empty(),
        "Should have no errors: {:?}",
        result.errors
    );
}

#[test]
fn test_audit_tamper_detection() {
    let dir = tempdir().expect("Failed to create temp dir");
    let config = AuditConfig::default();
    let logger =
        AuditLogger::new(config, dir.path().to_path_buf()).expect("Failed to create audit logger");

    // Log some events
    for i in 0..3 {
        let event = AuditEvent::new(AuditEventType::CommandExecution).with_action(
            format!("command_{}", i),
            "low".to_string(),
            true,
            true,
        );
        logger.log(&event).expect("Failed to log event");
    }

    // Verify initial chain
    let result = logger.verify_chain().expect("Failed to verify chain");
    assert!(result.valid);

    // Tamper with the log file
    let log_path = dir.path().join(&AuditConfig::default().log_path);
    let content = std::fs::read_to_string(&log_path).expect("Failed to read log");
    let tampered = content.replace("command_0", "TAMPERED");
    std::fs::write(&log_path, tampered).expect("Failed to write tampered log");

    // Verify should now fail
    let result = logger.verify_chain().expect("Failed to verify chain");
    assert!(!result.valid, "Should detect tampering");
    assert!(!result.errors.is_empty());
}

// -----------------------------------------------------------------------------
// Test 2: Snapshot Manager
// -----------------------------------------------------------------------------
#[test]
fn test_file_snapshot_and_restore() {
    let dir = tempdir().expect("Failed to create temp dir");
    let store_path = dir.path().join("snapshots");
    let manager = SnapshotManager::new(store_path).expect("Failed to create snapshot manager");

    // Create a test file
    let test_file = dir.path().join("test.txt");
    std::fs::write(&test_file, b"Original content").expect("Failed to write test file");

    // Create snapshot
    let snapshot = manager
        .create_snapshot(&test_file)
        .expect("Failed to create snapshot");
    assert_eq!(snapshot.snapshot_type, SnapshotType::File);
    assert!(snapshot.backup_path.exists());

    // Modify the file
    std::fs::write(&test_file, b"Modified content").expect("Failed to modify file");

    // Restore snapshot
    manager
        .restore_snapshot(&snapshot)
        .expect("Failed to restore snapshot");

    // Verify content is restored
    let content = std::fs::read_to_string(&test_file).expect("Failed to read file");
    assert_eq!(content, "Original content");
}

#[test]
fn test_database_snapshot() {
    let dir = tempdir().expect("Failed to create temp dir");
    let store_path = dir.path().join("snapshots");
    let manager = SnapshotManager::new(store_path).expect("Failed to create snapshot manager");

    // Create a simple SQLite database
    let db_path = dir.path().join("test.db");
    // Create a minimal SQLite file
    let db_content = b"SQLite format 3\x00"; // SQLite header
    std::fs::write(&db_path, db_content).expect("Failed to write db file");

    // Create database snapshot
    let snapshot = manager
        .create_database_snapshot(&db_path)
        .expect("Failed to create database snapshot");

    assert_eq!(snapshot.snapshot_type, SnapshotType::Database);
    assert!(snapshot.backup_path.exists());
}

#[test]
fn test_memory_snapshot() {
    let dir = tempdir().expect("Failed to create temp dir");
    let store_path = dir.path().join("snapshots");
    let manager = SnapshotManager::new(store_path).expect("Failed to create snapshot manager");

    // Create memory snapshot
    let memory_data = b"Serialized memory state".to_vec();
    let snapshot = manager
        .create_memory_snapshot(&memory_data, "session_state")
        .expect("Failed to create memory snapshot");

    assert_eq!(snapshot.snapshot_type, SnapshotType::Memory);

    // Restore memory snapshot
    let restored = manager
        .restore_memory_snapshot(&snapshot)
        .expect("Failed to restore memory snapshot");
    assert_eq!(restored, memory_data);
}

#[test]
fn test_system_snapshot() {
    let dir = tempdir().expect("Failed to create temp dir");
    let store_path = dir.path().join("snapshots");
    let manager = SnapshotManager::new(store_path).expect("Failed to create snapshot manager");

    // Create test files
    let config_file = dir.path().join("config.toml");
    std::fs::write(&config_file, b"version = 1.0").expect("Failed to write config");

    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&data_dir).expect("Failed to create data dir");
    std::fs::write(data_dir.join("file.txt"), b"data").expect("Failed to write data file");

    // Create system snapshot
    let components = vec![
        PathComponent {
            name: "config".to_string(),
            path: config_file.clone(),
            component_type: ComponentType::Config,
        },
        PathComponent {
            name: "data".to_string(),
            path: data_dir.clone(),
            component_type: ComponentType::Directory,
        },
    ];

    let metadata = SnapshotMetadata {
        created_by: Some("test".to_string()),
        description: Some("Test system snapshot".to_string()),
        tags: vec!["test".to_string()],
        parent_snapshot_id: None,
    };

    let snapshot = manager
        .create_system_snapshot(&components, metadata)
        .expect("Failed to create system snapshot");

    assert!(!snapshot.components.is_empty());
    assert_eq!(snapshot.components.len(), 2);
}

// -----------------------------------------------------------------------------
// Test 3: Rollback Mechanism
// -----------------------------------------------------------------------------
#[test]
fn test_transaction_commit() {
    let dir = tempdir().expect("Failed to create temp dir");
    let store_path = dir.path().join("snapshots");
    let snapshot_manager =
        Arc::new(SnapshotManager::new(store_path).expect("Failed to create manager"));

    let test_file = dir.path().join("test.txt");
    let content = b"Hello, World!".to_vec();

    let tx = TransactionBuilder::new(snapshot_manager)
        .write_file(test_file.clone(), content.clone())
        .build();

    tx.execute_atomic().expect("Transaction failed");

    assert!(test_file.exists());
    assert_eq!(
        std::fs::read(&test_file).expect("Failed to read file"),
        content
    );
}

#[test]
fn test_transaction_rollback_on_failure() {
    let dir = tempdir().expect("Failed to create temp dir");
    let store_path = dir.path().join("snapshots");
    let snapshot_manager =
        Arc::new(SnapshotManager::new(store_path).expect("Failed to create manager"));

    let test_file = dir.path().join("existing.txt");
    let original_content = b"Original content";
    std::fs::write(&test_file, original_content).expect("Failed to write original");

    let new_content = b"New content".to_vec();

    // Create transaction that will write new content
    let tx = TransactionBuilder::new(snapshot_manager.clone())
        .write_file(test_file.clone(), new_content.clone())
        .build();

    // Execute transaction
    tx.prepare().expect("Prepare failed");
    tx.execute().expect("Execute failed");

    // Verify new content
    assert_eq!(
        std::fs::read(&test_file).expect("Failed to read"),
        new_content
    );

    // Manually trigger rollback (simulating failure recovery)
    tx.rollback().expect("Rollback failed");

    // Verify content is restored
    assert_eq!(
        std::fs::read(&test_file).expect("Failed to read file"),
        original_content
    );
}

#[test]
fn test_transaction_delete_and_rollback() {
    let dir = tempdir().expect("Failed to create temp dir");
    let store_path = dir.path().join("snapshots");
    let snapshot_manager =
        Arc::new(SnapshotManager::new(store_path).expect("Failed to create manager"));

    let test_file = dir.path().join("delete_me.txt");
    let content = b"This file will be deleted";
    std::fs::write(&test_file, content).expect("Failed to write file");

    let tx = TransactionBuilder::new(snapshot_manager.clone())
        .delete_file(test_file.clone())
        .build();

    // Execute
    tx.prepare().expect("Prepare failed");
    tx.execute().expect("Execute failed");

    // File should be deleted
    assert!(!test_file.exists());

    // Rollback
    tx.rollback().expect("Rollback failed");

    // File should be restored
    assert!(test_file.exists());
    assert_eq!(std::fs::read(&test_file).expect("Failed to read"), content);
}

#[test]
fn test_recovery_policy() {
    let dir = tempdir().expect("Failed to create temp dir");
    let store_path = dir.path().join("snapshots");
    let snapshot_manager =
        Arc::new(SnapshotManager::new(store_path).expect("Failed to create manager"));

    // Test with automatic rollback policy
    let tx = TransactionManager::with_policy(
        snapshot_manager.clone(),
        RecoveryPolicy::AutomaticRollback,
    );

    assert_eq!(
        tx.status(),
        zeroclaw::safety::rollback::TransactionStatus::Pending
    );
}

// -----------------------------------------------------------------------------
// Test 4: Full Safety Flow (E2E)
// -----------------------------------------------------------------------------
#[test]
fn test_full_safety_flow() {
    // This test simulates a complete agent operation with safety:
    // 1. Audit the intent
    // 2. Create snapshot
    // 3. Execute operation via transaction
    // 4. Verify result (or rollback on failure)

    let dir = tempdir().expect("Failed to create temp dir");

    // Setup safety components
    let audit_config = AuditConfig::default();
    let audit_logger = AuditLogger::new(audit_config, dir.path().join("logs"))
        .expect("Failed to create audit logger");

    let snapshot_manager = Arc::new(
        SnapshotManager::new(dir.path().join("snapshots"))
            .expect("Failed to create snapshot manager"),
    );

    // Create a target file
    let target_file = dir.path().join("important_data.txt");
    std::fs::write(&target_file, b"Important data: 42").expect("Failed to write target");

    // 1. Audit the operation intent
    let audit_event = AuditEvent::new(AuditEventType::FileAccess)
        .with_actor("agent_worker".to_string(), None, None)
        .with_agent("worker_001".to_string())
        .with_resource_action(
            "write".to_string(),
            target_file.to_string_lossy().to_string(),
        );

    audit_logger.log(&audit_event).expect("Failed to log audit");

    // 2. Create snapshot before modification
    let snapshot = snapshot_manager
        .create_snapshot(&target_file)
        .expect("Failed to create snapshot");

    // 3. Execute operation via transaction
    let tx = TransactionBuilder::new(snapshot_manager.clone())
        .write_file(target_file.clone(), b"Important data: 84".to_vec())
        .build();

    // Simulate: Agent decides to commit
    tx.prepare().expect("Prepare failed");
    tx.execute().expect("Execute failed");

    // 4. Verify result
    let content = std::fs::read_to_string(&target_file).expect("Failed to read file");
    assert_eq!(content, "Important data: 84");

    // 5. Audit the result
    let result_event = AuditEvent::new(AuditEventType::CommandExecution)
        .with_actor("agent_worker".to_string(), None, None)
        .with_action("file_write".to_string(), "low".to_string(), true, true)
        .with_result(true, None, 50, None);

    audit_logger
        .log(&result_event)
        .expect("Failed to log result");

    // 6. Verify audit chain
    let verification = audit_logger.verify_chain().expect("Failed to verify chain");
    assert!(
        verification.valid,
        "Audit chain should be valid after operations"
    );
    assert_eq!(verification.total_events, 2);

    // 7. Cleanup (delete old snapshot)
    snapshot_manager
        .delete_snapshot(&snapshot)
        .expect("Failed to cleanup snapshot");
    assert!(!snapshot.backup_path.exists());
}

#[test]
fn test_safety_flow_with_rollback() {
    // Test the full safety flow when operation needs to be rolled back

    let dir = tempdir().expect("Failed to create temp dir");

    let audit_config = AuditConfig::default();
    let audit_logger = AuditLogger::new(audit_config, dir.path().join("logs"))
        .expect("Failed to create audit logger");

    let snapshot_manager = Arc::new(
        SnapshotManager::new(dir.path().join("snapshots"))
            .expect("Failed to create snapshot manager"),
    );

    let target_file = dir.path().join("config.json");
    let original_content = r#"{"version": 1, "enabled": true}"#;
    std::fs::write(&target_file, original_content).expect("Failed to write config");

    // 1. Audit operation start
    let start_event = AuditEvent::new(AuditEventType::ConfigChange)
        .with_agent("warden_001".to_string())
        .with_resource_action(
            "update".to_string(),
            target_file.to_string_lossy().to_string(),
        );
    audit_logger.log(&start_event).expect("Failed to log");

    // 2. Execute with transaction
    let tx = TransactionBuilder::new(snapshot_manager.clone())
        .write_file(
            target_file.clone(),
            br#"{"version": 2, "enabled": false}"#.to_vec(),
        )
        .build();

    tx.prepare().expect("Prepare failed");
    tx.execute().expect("Execute failed");

    // 3. Simulate: Warden rejects the change (policy violation)
    // Trigger rollback
    tx.rollback().expect("Rollback failed");

    // 4. Verify original content restored
    let content = std::fs::read_to_string(&target_file).expect("Failed to read file");
    assert_eq!(content, original_content);

    // 5. Audit the rollback
    let rollback_event = AuditEvent::new(AuditEventType::PolicyViolation)
        .with_agent("warden_001".to_string())
        .with_action(
            "config_update_rollback".to_string(),
            "high".to_string(),
            false,
            false,
        )
        .with_result(
            true,
            None,
            25,
            Some("Policy violation: unsafe config change".to_string()),
        );
    audit_logger
        .log(&rollback_event)
        .expect("Failed to log rollback");

    // 6. Verify audit chain integrity
    let verification = audit_logger.verify_chain().expect("Failed to verify chain");
    assert!(verification.valid);
    assert_eq!(verification.total_events, 2);
}
