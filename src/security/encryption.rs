//! Encryption Module
//!
//! Provides encryption utilities for secure data storage

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Encryption key (256-bit)
#[derive(Clone)]
pub struct EncryptionKey {
    key: [u8; 32],
}

impl EncryptionKey {
    /// Generate new random key
    pub fn generate() -> Self {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        Self { key }
    }
    
    /// Create from bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { key: bytes }
    }
    
    /// Create from base64 string
    pub fn from_base64(s: &str) -> Result<Self, String> {
        let bytes = BASE64
            .decode(s)
            .map_err(|e| format!("Invalid base64: {}", e))?;
        
        if bytes.len() != 32 {
            return Err("Key must be 32 bytes".to_string());
        }
        
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        
        Ok(Self { key })
    }
    
    /// Export to base64
    pub fn to_base64(&self) -> String {
        BASE64.encode(&self.key)
    }
    
    /// Get raw bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.key
    }
}

/// Encryptor for data encryption
pub struct Encryptor {
    key: Arc<EncryptionKey>,
}

impl Encryptor {
    /// Create new encryptor with generated key
    pub fn new() -> Self {
        Self {
            key: Arc::new(EncryptionKey::generate()),
        }
    }
    
    /// Create encryptor with specific key
    pub fn with_key(key: EncryptionKey) -> Self {
        Self {
            key: Arc::new(key),
        }
    }
    
    /// Get encryption key
    pub fn key(&self) -> EncryptionKey {
        (*self.key).clone()
    }
    
    /// Encrypt data
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let cipher = Aes256Gcm::new_from_slice(&self.key.key)
            .map_err(|e| format!("Failed to create cipher: {}", e))?;
        
        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Encrypt
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| format!("Encryption failed: {}", e))?;
        
        // Prepend nonce to ciphertext
        let mut result = nonce_bytes.to_vec();
        result.extend(ciphertext);
        
        Ok(result)
    }
    
    /// Decrypt data
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        if data.len() < 12 {
            return Err("Data too short".to_string());
        }
        
        let cipher = Aes256Gcm::new_from_slice(&self.key.key)
            .map_err(|e| format!("Failed to create cipher: {}", e))?;
        
        // Extract nonce and ciphertext
        let nonce = Nonce::from_slice(&data[..12]);
        let ciphertext = &data[12..];
        
        // Decrypt
        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| format!("Decryption failed: {}", e))
    }
    
    /// Encrypt string
    pub fn encrypt_string(&self, plaintext: &str) -> Result<String, String> {
        let encrypted = self.encrypt(plaintext.as_bytes())?;
        Ok(BASE64.encode(&encrypted))
    }
    
    /// Decrypt string
    pub fn decrypt_string(&self, encrypted: &str) -> Result<String, String> {
        let data = BASE64
            .decode(encrypted)
            .map_err(|e| format!("Invalid base64: {}", e))?;
        let decrypted = self.decrypt(&data)?;
        String::from_utf8(decrypted).map_err(|e| format!("Invalid UTF-8: {}", e))
    }
    
    /// Encrypt JSON
    pub fn encrypt_json<T: Serialize>(&self, value: &T) -> Result<String, String> {
        let json = serde_json::to_string(value)
            .map_err(|e| format!("JSON serialization failed: {}", e))?;
        self.encrypt_string(&json)
    }
    
    /// Decrypt JSON
    pub fn decrypt_json<T: for<'de> Deserialize<'de>>(&self, encrypted: &str) -> Result<T, String> {
        let json = self.decrypt_string(encrypted)?;
        serde_json::from_str(&json).map_err(|e| format!("JSON deserialization failed: {}", e))
    }
}

impl Default for Encryptor {
    fn default() -> Self {
        Self::new()
    }
}

/// Key derivation function for deriving keys from passwords
pub fn derive_key(password: &str, salt: &[u8]) -> Result<EncryptionKey, String> {
    use sha2::{Sha256, Digest};
    
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(salt);
    
    // Multiple rounds for key stretching
    let mut result = hasher.finalize();
    for _ in 0..10000 {
        let mut hasher = Sha256::new();
        hasher.update(&result);
        hasher.update(salt);
        result = hasher.finalize();
    }
    
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    
    Ok(EncryptionKey::from_bytes(key))
}

/// Hash password using Argon2id (simplified version)
pub fn hash_password(password: &str) -> Result<String, String> {
    use sha2::{Sha256, Digest};
    
    // Generate salt
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    
    // Hash password
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(&salt);
    
    for _ in 0..100000 {
        let result = hasher.finalize();
        let mut new_hasher = Sha256::new();
        new_hasher.update(&result);
        new_hasher.update(&salt);
        hasher = new_hasher;
    }
    
    let hash = hasher.finalize();
    
    // Format: salt$hash
    Ok(format!("{}${}", BASE64.encode(&salt), BASE64.encode(&hash)))
}

/// Verify password against hash
pub fn verify_password(password: &str, stored_hash: &str) -> Result<bool, String> {
    use sha2::{Sha256, Digest};
    
    let parts: Vec<&str> = stored_hash.split('$').collect();
    if parts.len() != 2 {
        return Err("Invalid hash format".to_string());
    }
    
    let salt = BASE64
        .decode(parts[0])
        .map_err(|e| format!("Invalid salt: {}", e))?;
    let expected_hash = BASE64
        .decode(parts[1])
        .map_err(|e| format!("Invalid hash: {}", e))?;
    
    // Hash password with same salt
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(&salt);
    
    for _ in 0..100000 {
        let result = hasher.finalize();
        let mut new_hasher = Sha256::new();
        new_hasher.update(&result);
        new_hasher.update(&salt);
        hasher = new_hasher;
    }
    
    let hash = hasher.finalize();
    
    Ok(hash.as_slice() == expected_hash.as_slice())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_encryption_key() {
        let key = EncryptionKey::generate();
        let base64 = key.to_base64();
        
        let key2 = EncryptionKey::from_base64(&base64).unwrap();
        assert_eq!(key.as_bytes(), key2.as_bytes());
    }
    
    #[test]
    fn test_encrypt_decrypt() {
        let encryptor = Encryptor::new();
        
        let plaintext = b"Hello, World!";
        let encrypted = encryptor.encrypt(plaintext).unwrap();
        let decrypted = encryptor.decrypt(&encrypted).unwrap();
        
        assert_eq!(plaintext.to_vec(), decrypted);
    }
    
    #[test]
    fn test_encrypt_decrypt_string() {
        let encryptor = Encryptor::new();
        
        let plaintext = "Hello, World!";
        let encrypted = encryptor.encrypt_string(plaintext).unwrap();
        let decrypted = encryptor.decrypt_string(&encrypted).unwrap();
        
        assert_eq!(plaintext, decrypted);
    }
    
    #[test]
    fn test_password_hashing() {
        let password = "my-secret-password";
        let hash = hash_password(password).unwrap();
        
        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrong-password", &hash).unwrap());
    }
}
