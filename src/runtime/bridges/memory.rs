use crate::runtime::state::HostState;
use crate::runtime::bindings::zeroclaw::host::memory::{Host, MemoryResult};
use crate::memory::MemoryCategory;
use wasmtime::Result;

#[async_trait::async_trait]
impl Host for HostState {
    /// Get a memory entry by key
    async fn get(&mut self, key: String) -> Result<Option<String>, String> {
        if let Some(ref memory) = self.memory {
            memory
                .get(&key)
                .await
                .map(|opt| opt.map(|entry| entry.content))
                .map_err(|e| format!("Failed to get memory: {}", e))
        } else {
            Ok(None)
        }
    }

    /// Store a memory entry with a key-value pair
    async fn set(&mut self, key: String, value: String) -> Result<(), String> {
        if let Some(ref memory) = self.memory {
            memory
                .store(
                    &key,
                    &value,
                    MemoryCategory::Core,
                    None, // No session ID for now
                )
                .await
                .map_err(|e| format!("Failed to store memory: {}", e))
        } else {
            Err("Memory backend not initialized".to_string())
        }
    }

    /// Delete a memory entry by key
    async fn delete(&mut self, key: String) -> Result<(), String> {
        if let Some(ref memory) = self.memory {
            memory
                .forget(&key)
                .await
                .map(|_| ())
                .map_err(|e| format!("Failed to delete memory: {}", e))
        } else {
            Err("Memory backend not initialized".to_string())
        }
    }

    /// Search memory entries by query (keyword/semantic search)
    async fn search(&mut self, query: String, limit: u32) -> Result<Vec<MemoryResult>, String> {
        if let Some(ref memory) = self.memory {
            let entries = memory
                .recall(&query, limit as usize, None)
                .await
                .map_err(|e| format!("Failed to search memory: {}", e))?;
            
            let results = entries
                .into_iter()
                .map(|entry| MemoryResult {
                    content: entry.content,
                    score: entry.score.unwrap_or(0.0) as f32,
                    metadata: serde_json::json!({
                        "id": entry.id,
                        "key": entry.key,
                        "category": entry.category,
                        "timestamp": entry.timestamp,
                        "session_id": entry.session_id,
                    })
                    .to_string(),
                })
                .collect();
            
            Ok(results)
        } else {
            Ok(vec![])
        }
    }
}
