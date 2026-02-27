use std::path::{Path, PathBuf};
use tempfile::TempDir;
use std::fs;

pub struct TestEnv {
    pub root: TempDir,
    pub data_dir: PathBuf,
    pub log_dir: PathBuf,
}

impl TestEnv {
    pub fn new() -> Self {
        let root = tempfile::tempdir().expect("Failed to create temp dir");
        let data_dir = root.path().join("data");
        let log_dir = root.path().join("logs");

        fs::create_dir_all(&data_dir).expect("Failed to create data dir");
        fs::create_dir_all(&log_dir).expect("Failed to create log dir");

        Self {
            root,
            data_dir,
            log_dir,
        }
    }

    pub fn create_file(&self, name: &str, content: &str) -> PathBuf {
        let path = self.data_dir.join(name);
        fs::write(&path, content).expect("Failed to create test file");
        path
    }

    pub fn read_file(&self, name: &str) -> String {
        let path = self.data_dir.join(name);
        fs::read_to_string(path).expect("Failed to read test file")
    }

    pub fn list_snapshots(&self) -> Vec<PathBuf> {
        // Mock implementation for snapshot listing
        fs::read_dir(&self.data_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.to_string_lossy().contains(".snapshot"))
            .collect()
    }
}
