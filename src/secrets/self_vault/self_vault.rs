use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce, Key,
};
use rand::RngCore;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use zeroize::Zeroize;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::secrets::self_vault::audit_trail::AuditTrail;
use crate::secrets::self_vault::access_control::AccessControl;

/// A secret entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretEntry {
    pub encrypted_value: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: u64,
    pub ttl_seconds: Option<u64>,
    pub path: String,
}

impl SecretEntry {
    /// Check if the secret has expired based on TTL
    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl_seconds {
            let age = Utc::now().signed_duration_since(self.updated_at);
            age.num_seconds() >= ttl as i64
        } else {
            false
        }
    }
}

/// Central SelfVault implementation providing enterprise-grade secrets management
#[derive(Clone)]
pub struct SelfVault {
    /// Encrypted storage for secrets
    storage: Arc<RwLock<HashMap<String, SecretEntry>>>,
    
    /// AES-256-GCM cipher for encryption/decryption
    cipher: Aes256Gcm,
    
    /// Encryption key (will be zeroized on drop)
    encryption_key: Vec<u8>,
    
    /// Audit trail for tracking all operations
    audit_trail: AuditTrail,
    
    /// Access control system
    access_control: AccessControl,
    
    /// Whether the vault is sealed
    sealed: Arc<RwLock<bool>>,
}

impl SelfVault {
    /// Create a new SelfVault instance with a 256-bit encryption key
    pub fn new(encryption_key: &[u8; 32]) -> Self {
        info!("🔐 Initializing SelfVault with AES-256-GCM encryption");
        
        let key = Key::<Aes256Gcm>::from_slice(encryption_key);
        let cipher = Aes256Gcm::new(key);
        
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
            cipher,
            encryption_key: encryption_key.to_vec(),
            audit_trail: AuditTrail::new(),
            access_control: AccessControl::new(),
            sealed: Arc::new(RwLock::new(false)),
        }
    }

    /// Generate a random encryption key (for initialization)
    pub fn generate_master_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut key);
        info!("🔑 Generated new master encryption key");
        key
    }

    /// Seal the vault (prevent access to secrets)
    pub async fn seal(&self) {
        let mut sealed = self.sealed.write().await;
        *sealed = true;
        self.audit_trail.log_system_event("VAULT_SEALED", "System").await;
        info!("🔒 Vault sealed");
    }

    /// Unseal the vault (allow access to secrets)
    pub async fn unseal(&self) {
        let mut sealed = self.sealed.write().await;
        *sealed = false;
        self.audit_trail.log_system_event("VAULT_UNSEALED", "System").await;
        info!("🔓 Vault unsealed");
    }

    /// Check if vault is sealed
    pub async fn is_sealed(&self) -> bool {
        *self.sealed.read().await
    }

    /// Store a secret at the given path
    pub async fn put_secret(
        &self,
        path: &str,
        value: &str,
        ttl_seconds: Option<u64>,
        user: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Check if vault is sealed
        if self.is_sealed().await {
            return Err("Vault is sealed".into());
        }

        // Check access control
        if !self.access_control.check_write_access(user, path).await {
            self.audit_trail.log_access_denied(user, "WRITE", path).await;
            return Err(format!("Access denied for user '{}' to path '{}'", user, path).into());
        }

        // Encrypt the value
        let encrypted = self.encrypt_value(value)?;

        let now = Utc::now();
        let entry = SecretEntry {
            encrypted_value: encrypted,
            created_at: now,
            updated_at: now,
            version: 1,
            ttl_seconds,
            path: path.to_string(),
        };

        // Store in vault
        let mut storage = self.storage.write().await;
        storage.insert(path.to_string(), entry);

        // Log the operation
        self.audit_trail.log_secret_operation(user, "PUT", path).await;
        info!("✓ Secret stored at path: {}", path);

        Ok(())
    }

    /// Retrieve a secret from the given path
    pub async fn get_secret(
        &self,
        path: &str,
        user: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        // Check if vault is sealed
        if self.is_sealed().await {
            return Err("Vault is sealed".into());
        }

        // Check access control
        if !self.access_control.check_read_access(user, path).await {
            self.audit_trail.log_access_denied(user, "READ", path).await;
            return Err(format!("Access denied for user '{}' to path '{}'", user, path).into());
        }

        let storage = self.storage.read().await;
        
        if let Some(entry) = storage.get(path) {
            // Check if expired
            if entry.is_expired() {
                drop(storage);
                self.audit_trail.log_secret_operation(user, "EXPIRED_READ", path).await;
                warn!("⏰ Secret at path '{}' has expired", path);
                return Ok(None);
            }

            // Decrypt the value
            match self.decrypt_value(&entry.encrypted_value) {
                Ok(value) => {
                    self.audit_trail.log_secret_operation(user, "GET", path).await;
                    info!("✓ Secret retrieved from path: {}", path);
                    Ok(Some(value))
                }
                Err(e) => {
                    error!("Failed to decrypt secret at {}: {}", path, e);
                    self.audit_trail.log_error(user, "DECRYPT_ERROR", path, &e.to_string()).await;
                    Err(e)
                }
            }
        } else {
            self.audit_trail.log_secret_operation(user, "NOT_FOUND", path).await;
            Ok(None)
        }
    }

    /// Delete a secret at the given path
    pub async fn delete_secret(
        &self,
        path: &str,
        user: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // Check if vault is sealed
        if self.is_sealed().await {
            return Err("Vault is sealed".into());
        }

        // Check access control
        if !self.access_control.check_delete_access(user, path).await {
            self.audit_trail.log_access_denied(user, "DELETE", path).await;
            return Err(format!("Access denied for user '{}' to path '{}'", user, path).into());
        }

        let mut storage = self.storage.write().await;
        let deleted = storage.remove(path).is_some();

        if deleted {
            self.audit_trail.log_secret_operation(user, "DELETE", path).await;
            info!("🗑️  Secret deleted from path: {}", path);
        }

        Ok(deleted)
    }

    /// List all secret paths (not values)
    pub async fn list_secrets(&self, user: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        // Check if vault is sealed
        if self.is_sealed().await {
            return Err("Vault is sealed".into());
        }

        // Check access control
        if !self.access_control.check_list_access(user).await {
            self.audit_trail.log_access_denied(user, "LIST", "*").await;
            return Err(format!("Access denied for user '{}' to list secrets", user).into());
        }

        let storage = self.storage.read().await;
        let paths: Vec<String> = storage.keys().cloned().collect();

        self.audit_trail.log_secret_operation(user, "LIST", "*").await;
        Ok(paths)
    }

    /// Encrypt a value using AES-256-GCM
    fn encrypt_value(&self, value: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt the value
        let ciphertext = self.cipher.encrypt(nonce, value.as_bytes())
            .map_err(|e| format!("Encryption failed: {}", e))?;

        // Prepend nonce to ciphertext
        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    /// Decrypt a value using AES-256-GCM
    fn decrypt_value(&self, encrypted_data: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
        if encrypted_data.len() < 12 {
            return Err("Invalid encrypted data: too short".into());
        }

        // Extract nonce and ciphertext
        let nonce_bytes = &encrypted_data[..12];
        let ciphertext = &encrypted_data[12..];
        
        let nonce = Nonce::from_slice(nonce_bytes);

        // Decrypt
        let plaintext = self.cipher.decrypt(nonce, ciphertext)
            .map_err(|e| format!("Decryption failed: {}", e))?;

        let value = String::from_utf8(plaintext)?;
        Ok(value)
    }

    /// Get reference to audit trail
    pub fn audit_trail(&self) -> &AuditTrail {
        &self.audit_trail
    }

    /// Get reference to access control
    pub fn access_control(&self) -> &AccessControl {
        &self.access_control
    }

    /// Get number of secrets stored
    pub async fn secret_count(&self) -> usize {
        let storage = self.storage.read().await;
        storage.len()
    }
}

impl Drop for SelfVault {
    fn drop(&mut self) {
        // Zero out the encryption key from memory
        self.encryption_key.zeroize();
        info!("🔒 SelfVault dropped, encryption key wiped from memory");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let key = SelfVault::generate_master_key();
        let vault = SelfVault::new(&key);

        vault.put_secret("secret/test", "my_secret_value", None, "admin")
            .await
            .unwrap();

        let retrieved = vault.get_secret("secret/test", "admin")
            .await
            .unwrap();

        assert_eq!(retrieved, Some("my_secret_value".to_string()));
    }

    #[tokio::test]
    async fn test_ttl_expiry() {
        let key = SelfVault::generate_master_key();
        let vault = SelfVault::new(&key);

        // Store with 1 second TTL
        vault.put_secret("secret/expiring", "temp_value", Some(1), "admin")
            .await
            .unwrap();

        // Should be available immediately
        let retrieved = vault.get_secret("secret/expiring", "admin")
            .await
            .unwrap();
        assert_eq!(retrieved, Some("temp_value".to_string()));

        // Wait for expiry
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Should be expired now
        let retrieved = vault.get_secret("secret/expiring", "admin")
            .await
            .unwrap();
        assert_eq!(retrieved, None);
    }

    #[tokio::test]
    async fn test_seal_unseal() {
        let key = SelfVault::generate_master_key();
        let vault = SelfVault::new(&key);

        vault.put_secret("secret/test", "value", None, "admin")
            .await
            .unwrap();

        // Seal the vault
        vault.seal().await;
        assert!(vault.is_sealed().await);

        // Should not be able to access secrets
        let result = vault.get_secret("secret/test", "admin").await;
        assert!(result.is_err());

        // Unseal and verify access restored
        vault.unseal().await;
        assert!(!vault.is_sealed().await);

        let retrieved = vault.get_secret("secret/test", "admin")
            .await
            .unwrap();
        assert_eq!(retrieved, Some("value".to_string()));
    }
}
