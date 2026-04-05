use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce, Key,
};
use rand::RngCore;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, warn};
use zeroize::Zeroize;

/// Encrypted in-memory cache for secrets using AES-256-GCM
pub struct EncryptedSecretCache {
    cipher: Aes256Gcm,
    cache: RwLock<HashMap<String, Vec<u8>>>, // key -> encrypted value
    encryption_key: Vec<u8>, // Will be zeroized on drop
}

impl EncryptedSecretCache {
    /// Create a new encrypted cache with a 256-bit key
    pub fn new(encryption_key: &[u8; 32]) -> Self {
        info!("🔐 Initializing AES-256-GCM encrypted secret cache");
        
        let key = Key::<Aes256Gcm>::from_slice(encryption_key);
        let cipher = Aes256Gcm::new(key);
        
        Self {
            cipher,
            cache: RwLock::new(HashMap::new()),
            encryption_key: encryption_key.to_vec(),
        }
    }

    /// Generate a random encryption key (for testing/development)
    pub fn generate_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut key);
        key
    }

    /// Store a secret in the encrypted cache
    pub async fn store(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt the value
        let ciphertext = self.cipher.encrypt(nonce, value.as_bytes())
            .map_err(|e| format!("Encryption failed: {}", e))?;

        // Prepend nonce to ciphertext for storage
        let mut stored_data = nonce_bytes.to_vec();
        stored_data.extend_from_slice(&ciphertext);

        // Store in cache
        let mut cache = self.cache.write().await;
        cache.insert(key.to_string(), stored_data);
        
        info!("✓ Secret stored in encrypted cache: {}", key);
        Ok(())
    }

    /// Retrieve and decrypt a secret from the cache
    pub async fn retrieve(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let cache = self.cache.read().await;
        
        if let Some(stored_data) = cache.get(key) {
            if stored_data.len() < 12 {
                warn!("⚠️  Invalid encrypted data for key: {}", key);
                return Ok(None);
            }

            // Extract nonce and ciphertext
            let nonce_bytes = &stored_data[..12];
            let ciphertext = &stored_data[12..];
            
            let nonce = Nonce::from_slice(nonce_bytes);

            // Decrypt
            match self.cipher.decrypt(nonce, ciphertext.as_ref()) {
                Ok(plaintext) => {
                    let value = String::from_utf8(plaintext)?;
                    info!("✓ Secret retrieved from encrypted cache: {}", key);
                    Ok(Some(value))
                }
                Err(e) => {
                    warn!("⚠️  Failed to decrypt secret {}: {}", key, e);
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    /// Remove a secret from the cache
    pub async fn remove(&self, key: &str) -> Option<Vec<u8>> {
        let mut cache = self.cache.write().await;
        cache.remove(key)
    }

    /// Clear all cached secrets
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        info!("🗑️  Encrypted cache cleared");
    }

    /// Get number of cached secrets
    pub async fn len(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// Check if cache is empty
    pub async fn is_empty(&self) -> bool {
        let cache = self.cache.read().await;
        cache.is_empty()
    }
}

impl Drop for EncryptedSecretCache {
    fn drop(&mut self) {
        // Zero out the encryption key from memory
        self.encryption_key.zeroize();
        info!("🔒 Encrypted cache dropped, encryption key wiped from memory");
    }
}

/// Cache entry with TTL (time-to-live)
pub struct CachedSecret {
    pub encrypted_value: Vec<u8>,
    pub created_at: std::time::Instant,
    pub ttl: std::time::Duration,
}

impl CachedSecret {
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}

/// Enhanced cache with TTL support
pub struct TtlEncryptedCache {
    base_cache: EncryptedSecretCache,
    ttl_entries: RwLock<HashMap<String, CachedSecret>>,
    default_ttl: std::time::Duration,
}

impl TtlEncryptedCache {
    pub fn new(encryption_key: &[u8; 32], default_ttl_secs: u64) -> Self {
        Self {
            base_cache: EncryptedSecretCache::new(encryption_key),
            ttl_entries: RwLock::new(HashMap::new()),
            default_ttl: std::time::Duration::from_secs(default_ttl_secs),
        }
    }

    pub async fn store_with_ttl(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Store in base cache
        self.base_cache.store(key, value).await?;
        
        // Track TTL
        let mut entries = self.ttl_entries.write().await;
        entries.insert(key.to_string(), CachedSecret {
            encrypted_value: vec![], // Placeholder - would need to get from base cache
            created_at: std::time::Instant::now(),
            ttl: self.default_ttl,
        });
        
        Ok(())
    }

    pub async fn retrieve(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        // Check TTL first
        let entries = self.ttl_entries.read().await;
        if let Some(entry) = entries.get(key) {
            if entry.is_expired() {
                drop(entries);
                info!("⏰ TTL expired for key: {}, removing", key);
                self.base_cache.remove(key).await;
                return Ok(None);
            }
        }
        drop(entries);

        // Retrieve from base cache
        self.base_cache.retrieve(key).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_encrypt_decrypt() {
        let key = EncryptedSecretCache::generate_key();
        let cache = EncryptedSecretCache::new(&key);
        
        cache.store("test_secret", "super_secret_value").await.unwrap();
        let retrieved = cache.retrieve("test_secret").await.unwrap();
        
        assert_eq!(retrieved, Some("super_secret_value".to_string()));
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let key = EncryptedSecretCache::generate_key();
        let cache = EncryptedSecretCache::new(&key);
        
        cache.store("secret1", "value1").await.unwrap();
        cache.store("secret2", "value2").await.unwrap();
        
        assert_eq!(cache.len().await, 2);
        
        cache.clear().await;
        assert_eq!(cache.len().await, 0);
    }
}
