//! Device Pairing Module
//!
//! Provides secure device pairing capabilities for ServantGuild.
//! Handles device authentication, trust establishment, and key exchange.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Constant-time comparison for security-sensitive data
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    
    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

/// Check if a bind address is public (not localhost/private)
pub fn is_public_bind(addr: &str) -> bool {
    !addr.starts_with("127.") 
        && !addr.starts_with("::1") 
        && !addr.starts_with("localhost")
        && !addr.starts_with("0.0.0.0")
        && !addr.starts_with("[::]")
}

/// Pairing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingConfig {
    /// Enable pairing mode
    pub enabled: bool,
    /// Pairing timeout in seconds
    pub timeout_secs: u64,
    /// Maximum pairing attempts
    pub max_attempts: u32,
    /// Require confirmation
    pub require_confirmation: bool,
}

impl Default for PairingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout_secs: 300,
            max_attempts: 3,
            require_confirmation: true,
        }
    }
}

/// Pairing status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PairingStatus {
    /// Waiting for pairing request
    Waiting,
    /// Pairing in progress
    InProgress,
    /// Paired successfully
    Paired,
    /// Pairing rejected
    Rejected,
    /// Pairing expired
    Expired,
    /// Pairing failed
    Failed,
}

/// Paired device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairedDevice {
    /// Device ID
    pub device_id: String,
    /// Device name
    pub name: String,
    /// Device type
    pub device_type: String,
    /// Public key fingerprint
    pub key_fingerprint: String,
    /// Paired at
    pub paired_at: DateTime<Utc>,
    /// Last seen
    pub last_seen: DateTime<Utc>,
    /// Trust level
    pub trust_level: TrustLevel,
}

/// Trust level for paired devices
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustLevel {
    /// Untrusted - limited access
    Untrusted,
    /// Trusted - standard access
    Trusted,
    /// Admin - full access
    Admin,
    /// Owner - complete control
    Owner,
}

/// Pairing guard for access control
pub struct PairingGuard {
    /// Whether pairing is required
    require_pairing: bool,
    /// Trusted device IDs
    trusted_devices: Vec<String>,
    /// Pairing code
    pairing_code: Option<String>,
    /// Whether currently paired
    is_paired: bool,
}

impl PairingGuard {
    /// Create a new pairing guard
    pub fn new(require_pairing: bool) -> Self {
        Self {
            require_pairing,
            trusted_devices: Vec::new(),
            pairing_code: None,
            is_paired: false,
        }
    }

    /// Add a trusted device
    pub fn add_trusted(&mut self, device_id: String) {
        if !self.trusted_devices.contains(&device_id) {
            self.trusted_devices.push(device_id);
        }
    }

    /// Check if access is allowed
    pub fn check_access(&self, device_id: Option<&str>) -> bool {
        if !self.require_pairing {
            return true;
        }
        
        match device_id {
            Some(id) => self.trusted_devices.contains(&id.to_string()),
            None => false,
        }
    }

    /// Get the pairing code
    pub fn pairing_code(&self) -> Option<&str> {
        self.pairing_code.as_deref()
    }

    /// Check if pairing is required
    pub fn require_pairing(&self) -> bool {
        self.require_pairing
    }

    /// Check if currently paired
    pub fn is_paired(&self) -> bool {
        self.is_paired
    }

    /// Check if authenticated
    pub fn is_authenticated(&self) -> bool {
        !self.require_pairing || self.is_paired
    }

    /// Try to pair with a code
    pub async fn try_pair(&self, code: &str, _context: &str) -> Result<(), String> {
        if !self.require_pairing {
            return Ok(());
        }
        
        match &self.pairing_code {
            Some(expected) if expected == code => Ok(()),
            Some(_) => Err("Invalid pairing code".to_string()),
            None => Err("No pairing in progress".to_string()),
        }
    }
}

/// Device pairing manager
pub struct DevicePairing {
    config: PairingConfig,
    paired_devices: Arc<RwLock<HashMap<String, PairedDevice>>>,
    pending_pairings: Arc<RwLock<HashMap<String, PendingPairing>>>,
}

/// Pending pairing request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingPairing {
    /// Request ID
    pub id: String,
    /// Device info
    pub device_info: String,
    /// Status
    pub status: PairingStatus,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Attempts
    pub attempts: u32,
}

impl DevicePairing {
    /// Create a new device pairing manager
    pub fn new(config: PairingConfig) -> Self {
        Self {
            config,
            paired_devices: Arc::new(RwLock::new(HashMap::new())),
            pending_pairings: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initiate a pairing request
    pub async fn initiate_pairing(&self, device_info: &str) -> Result<String, String> {
        if !self.config.enabled {
            return Err("Pairing is disabled".to_string());
        }

        let id = uuid::Uuid::new_v4().to_string();
        let pairing = PendingPairing {
            id: id.clone(),
            device_info: device_info.to_string(),
            status: PairingStatus::InProgress,
            created_at: Utc::now(),
            attempts: 0,
        };

        let mut pending = self.pending_pairings.write().await;
        pending.insert(id.clone(), pairing);

        Ok(id)
    }

    /// Complete a pairing request
    pub async fn complete_pairing(&self, pairing_id: &str, device: PairedDevice) -> Result<(), String> {
        let mut pending = self.pending_pairings.write().await;
        
        if let Some(pairing) = pending.remove(pairing_id) {
            if pairing.status != PairingStatus::InProgress {
                return Err("Pairing is not in progress".to_string());
            }

            let mut devices = self.paired_devices.write().await;
            devices.insert(device.device_id.clone(), device);
            
            Ok(())
        } else {
            Err("Pairing request not found".to_string())
        }
    }

    /// Reject a pairing request
    pub async fn reject_pairing(&self, pairing_id: &str) -> Result<(), String> {
        let mut pending = self.pending_pairings.write().await;
        
        if let Some(pairing) = pending.get_mut(pairing_id) {
            pairing.status = PairingStatus::Rejected;
            Ok(())
        } else {
            Err("Pairing request not found".to_string())
        }
    }

    /// Get all paired devices
    pub async fn get_paired_devices(&self) -> Vec<PairedDevice> {
        let devices = self.paired_devices.read().await;
        devices.values().cloned().collect()
    }

    /// Remove a paired device
    pub async fn unpair(&self, device_id: &str) -> Result<(), String> {
        let mut devices = self.paired_devices.write().await;
        
        if devices.remove(device_id).is_some() {
            Ok(())
        } else {
            Err("Device not found".to_string())
        }
    }

    /// Check if a device is paired
    pub async fn is_paired(&self, device_id: &str) -> bool {
        let devices = self.paired_devices.read().await;
        devices.contains_key(device_id)
    }

    /// Get device trust level
    pub async fn get_trust_level(&self, device_id: &str) -> Option<TrustLevel> {
        let devices = self.paired_devices.read().await;
        devices.get(device_id).map(|d| d.trust_level)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_initiate_pairing() {
        let config = PairingConfig::default();
        let pairing = DevicePairing::new(config);

        let id = pairing.initiate_pairing("test-device").await.unwrap();
        assert!(!id.is_empty());
    }

    #[tokio::test]
    async fn test_complete_pairing() {
        let config = PairingConfig::default();
        let pairing = DevicePairing::new(config);

        let id = pairing.initiate_pairing("test-device").await.unwrap();
        
        let device = PairedDevice {
            device_id: "test-device-1".to_string(),
            name: "Test Device".to_string(),
            device_type: "cli".to_string(),
            key_fingerprint: "abc123".to_string(),
            paired_at: Utc::now(),
            last_seen: Utc::now(),
            trust_level: TrustLevel::Trusted,
        };

        pairing.complete_pairing(&id, device).await.unwrap();
        assert!(pairing.is_paired("test-device-1").await);
    }
}
