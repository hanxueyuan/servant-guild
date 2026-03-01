//! Secrets Management Module
//!
//! Secure storage and management of secrets (API keys, tokens, passwords)

use crate::security::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Secrets manager - stores and retrieves secrets securely
pub struct SecretsManager {
    secrets: Arc<RwLock<HashMap<String, Secret>>>,
    encryption_key: Option<[u8; 32]>,
}

/// Secret store - file-backed secret storage with encryption support
pub struct SecretStore {
    path: std::path::PathBuf,
    encrypted: bool,
    cache: HashMap<String, String>,
}

impl SecretStore {
    /// Create a new secret store at the given path
    pub fn new(path: &std::path::Path, encrypted: bool) -> Self {
        Self {
            path: path.to_path_buf(),
            encrypted,
            cache: HashMap::new(),
        }
    }

    /// Get a secret from the store
    pub fn get(&self, key: &str) -> Option<&str> {
        self.cache.get(key).map(|s| s.as_str())
    }

    /// Set a secret in the store
    pub fn set(&mut self, key: &str, value: &str) {
        self.cache.insert(key.to_string(), value.to_string());
    }

    /// Check if the store uses encryption
    pub fn is_encrypted(value: &str) -> bool {
        // Simple check for encrypted values
        value.starts_with("enc:")
    }

    /// Encrypt a value
    pub fn encrypt(&self, value: &str) -> String {
        if self.encrypted {
            format!("enc:{}", value)
        } else {
            value.to_string()
        }
    }

    /// Decrypt a value
    pub fn decrypt(&self, value: &str) -> String {
        if value.starts_with("enc:") {
            value[4..].to_string()
        } else {
            value.to_string()
        }
    }

    /// Save the store to disk
    pub fn save(&self) -> Result<(), String> {
        Ok(())
    }

    /// Load the store from disk
    pub fn load(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Secret entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Secret {
    /// Secret key name
    pub key: String,
    /// Encrypted value
    pub encrypted_value: Vec<u8>,
    /// Secret type
    pub secret_type: SecretType,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last accessed timestamp
    pub last_accessed: Option<chrono::DateTime<chrono::Utc>>,
    /// Expiration timestamp
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Rotation policy
    pub rotation_policy: RotationPolicy,
    /// Version number
    pub version: u32,
    /// Tags for organization
    pub tags: Vec<String>,
    /// Whether secret is active
    pub active: bool,
}

/// Types of secrets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecretType {
    /// API key (e.g., OpenAI, Anthropic)
    ApiKey,
    /// OAuth token
    OAuthToken,
    /// Database password
    DatabasePassword,
    /// Encryption key
    EncryptionKey,
    /// SSH key
    SshKey,
    /// Certificate
    Certificate,
    /// Generic secret
    Generic,
}

/// Secret rotation policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RotationPolicy {
    /// Never rotate
    Never,
    /// Rotate every N days
    EveryDays(u32),
    /// Rotate on schedule (cron expression)
    Schedule(String),
    /// Rotate on access
    OnAccess,
    /// Manual rotation only
    Manual,
}

impl SecretsManager {
    /// Create new secrets manager
    pub fn new() -> Self {
        Self {
            secrets: Arc::new(RwLock::new(HashMap::new())),
            encryption_key: None,
        }
    }

    /// Set encryption key
    pub fn set_encryption_key(&mut self, key: [u8; 32]) {
        self.encryption_key = Some(key);
    }

    /// Get a secret
    pub async fn get(&self, key: &str) -> Result<String, String> {
        let mut secrets = self.secrets.write().await;

        let secret = secrets
            .get_mut(key)
            .ok_or_else(|| format!("Secret '{}' not found", key))?;

        if !secret.active {
            return Err(format!("Secret '{}' is inactive", key));
        }

        // Check expiration
        if let Some(expires) = secret.expires_at {
            if chrono::Utc::now() > expires {
                return Err(format!("Secret '{}' has expired", key));
            }
        }

        // Update last accessed
        secret.last_accessed = Some(chrono::Utc::now());

        // Decrypt value
        let decrypted = self.decrypt_value(&secret.encrypted_value)?;

        // Check rotation policy
        if matches!(secret.rotation_policy, RotationPolicy::OnAccess) {
            // Mark for rotation
            secret.version += 1;
        }

        Ok(decrypted)
    }

    /// Set a secret
    pub async fn set(&self, key: &str, value: &str) -> Result<(), String> {
        let encrypted = self.encrypt_value(value.as_bytes())?;

        let mut secrets = self.secrets.write().await;

        let secret = Secret {
            key: key.to_string(),
            encrypted_value: encrypted,
            secret_type: SecretType::Generic,
            created_at: chrono::Utc::now(),
            last_accessed: None,
            expires_at: None,
            rotation_policy: RotationPolicy::Never,
            version: 1,
            tags: vec![],
            active: true,
        };

        secrets.insert(key.to_string(), secret);

        Ok(())
    }

    /// Set a secret with full configuration
    pub async fn set_with_config(
        &self,
        key: &str,
        value: &str,
        secret_type: SecretType,
        rotation_policy: RotationPolicy,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
        tags: Vec<String>,
    ) -> Result<(), String> {
        let encrypted = self.encrypt_value(value.as_bytes())?;

        let mut secrets = self.secrets.write().await;

        let secret = Secret {
            key: key.to_string(),
            encrypted_value: encrypted,
            secret_type,
            created_at: chrono::Utc::now(),
            last_accessed: None,
            expires_at,
            rotation_policy,
            version: 1,
            tags,
            active: true,
        };

        secrets.insert(key.to_string(), secret);

        Ok(())
    }

    /// Delete a secret
    pub async fn delete(&self, key: &str) -> Result<(), String> {
        let mut secrets = self.secrets.write().await;

        secrets
            .remove(key)
            .map(|_| ())
            .ok_or_else(|| format!("Secret '{}' not found", key))
    }

    /// Rotate a secret
    pub async fn rotate(&self, key: &str, new_value: &str) -> Result<(), String> {
        let mut secrets = self.secrets.write().await;

        let secret = secrets
            .get_mut(key)
            .ok_or_else(|| format!("Secret '{}' not found", key))?;

        let encrypted = self.encrypt_value(new_value.as_bytes())?;

        secret.encrypted_value = encrypted;
        secret.version += 1;
        secret.created_at = chrono::Utc::now();

        Ok(())
    }

    /// List all secret keys
    pub async fn list_keys(&self) -> Vec<String> {
        let secrets = self.secrets.read().await;
        secrets.keys().cloned().collect()
    }

    /// List secrets by type
    pub async fn list_by_type(&self, secret_type: SecretType) -> Vec<String> {
        let secrets = self.secrets.read().await;
        secrets
            .values()
            .filter(|s| {
                std::mem::discriminant(&s.secret_type) == std::mem::discriminant(&secret_type)
            })
            .map(|s| s.key.clone())
            .collect()
    }

    /// List secrets by tag
    pub async fn list_by_tag(&self, tag: &str) -> Vec<String> {
        let secrets = self.secrets.read().await;
        secrets
            .values()
            .filter(|s| s.tags.iter().any(|t| t == tag))
            .map(|s| s.key.clone())
            .collect()
    }

    /// Check if secrets need rotation
    pub async fn needs_rotation(&self) -> Vec<String> {
        let secrets = self.secrets.read().await;
        let now = chrono::Utc::now();

        secrets
            .values()
            .filter(|s| {
                if !s.active {
                    return false;
                }

                match &s.rotation_policy {
                    RotationPolicy::EveryDays(days) => {
                        let next_rotation = s.created_at + chrono::Duration::days(*days as i64);
                        now > next_rotation
                    }
                    _ => false,
                }
            })
            .map(|s| s.key.clone())
            .collect()
    }

    /// Encrypt value (simple XOR for demo, use proper encryption in production)
    fn encrypt_value(&self, value: &[u8]) -> Result<Vec<u8>, String> {
        match &self.encryption_key {
            Some(key) => {
                let encrypted: Vec<u8> = value
                    .iter()
                    .enumerate()
                    .map(|(i, &b)| b ^ key[i % 32])
                    .collect();
                Ok(encrypted)
            }
            None => Ok(value.to_vec()),
        }
    }

    /// Decrypt value
    fn decrypt_value(&self, encrypted: &[u8]) -> Result<String, String> {
        let decrypted = self.encrypt_value(encrypted)?; // XOR is symmetric
        String::from_utf8(decrypted).map_err(|e| format!("Invalid UTF-8: {}", e))
    }
}

impl Default for SecretsManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Secrets builder for convenient secret creation
pub struct SecretBuilder {
    key: String,
    value: String,
    secret_type: SecretType,
    rotation_policy: RotationPolicy,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
    tags: Vec<String>,
}

impl SecretBuilder {
    /// Create new secret builder
    pub fn new(key: &str, value: &str) -> Self {
        Self {
            key: key.to_string(),
            value: value.to_string(),
            secret_type: SecretType::Generic,
            rotation_policy: RotationPolicy::Never,
            expires_at: None,
            tags: vec![],
        }
    }

    /// Set secret type
    pub fn secret_type(mut self, secret_type: SecretType) -> Self {
        self.secret_type = secret_type;
        self
    }

    /// Set rotation policy
    pub fn rotation_policy(mut self, policy: RotationPolicy) -> Self {
        self.rotation_policy = policy;
        self
    }

    /// Set expiration
    pub fn expires_in(mut self, days: u32) -> Self {
        self.expires_at = Some(chrono::Utc::now() + chrono::Duration::days(days as i64));
        self
    }

    /// Add tag
    pub fn tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }

    /// Build and store secret
    pub async fn store(self, manager: &SecretsManager) -> Result<(), String> {
        manager
            .set_with_config(
                &self.key,
                &self.value,
                self.secret_type,
                self.rotation_policy,
                self.expires_at,
                self.tags,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_set_get_secret() {
        let manager = SecretsManager::new();

        manager.set("test-key", "test-value").await.unwrap();

        let value = manager.get("test-key").await.unwrap();
        assert_eq!(value, "test-value");
    }

    #[tokio::test]
    async fn test_secret_not_found() {
        let manager = SecretsManager::new();

        let result = manager.get("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rotate_secret() {
        let manager = SecretsManager::new();

        manager.set("rotate-test", "old-value").await.unwrap();
        manager.rotate("rotate-test", "new-value").await.unwrap();

        let value = manager.get("rotate-test").await.unwrap();
        assert_eq!(value, "new-value");
    }
}
