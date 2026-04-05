use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, warn};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::self_vault::SelfVault;
use crate::secrets::self_vault::audit_trail::AuditTrail;

/// Dynamic credential that automatically expires
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicCredential {
    pub username: String,
    pub password: String,
    pub created_at: chrono::DateTime<Utc>,
    pub expires_at: chrono::DateTime<Utc>,
    pub credential_type: String,
}

impl DynamicCredential {
    /// Check if the credential is still valid
    pub fn is_valid(&self) -> bool {
        Utc::now() < self.expires_at
    }

    /// Get remaining time until expiry
    pub fn time_until_expiry(&self) -> Duration {
        let now = Utc::now();
        if now >= self.expires_at {
            Duration::from_secs(0)
        } else {
            let seconds = (self.expires_at - now).num_seconds();
            Duration::from_secs(seconds as u64)
        }
    }

    /// Check if credential should be renewed (within renewal threshold)
    pub fn should_renew(&self, threshold_seconds: u64) -> bool {
        self.time_until_expiry() <= Duration::from_secs(threshold_seconds)
    }
}

/// Manager for dynamic credentials with automatic rotation
pub struct DynamicCredentialsManager {
    vault: Arc<SelfVault>,
    credentials: RwLock<Vec<DynamicCredential>>,
    default_ttl_seconds: u64,
    renewal_threshold_seconds: u64,
    audit_trail: AuditTrail,
}

impl DynamicCredentialsManager {
    /// Create a new dynamic credentials manager
    pub fn new(
        vault: Arc<SelfVault>,
        default_ttl_seconds: u64,
        renewal_threshold_seconds: u64,
    ) -> Self {
        info!(
            "🗄️  Initializing Dynamic Credentials Manager (TTL: {}s, Renewal: {}s)",
            default_ttl_seconds, renewal_threshold_seconds
        );

        Self {
            vault: vault.clone(),
            credentials: RwLock::new(Vec::new()),
            default_ttl_seconds,
            renewal_threshold_seconds,
            audit_trail: vault.audit_trail().clone(),
        }
    }

    /// Generate a new dynamic credential and store it in the vault
    pub async fn generate_credential(
        &self,
        path: &str,
        credential_type: &str,
        user: &str,
    ) -> Result<DynamicCredential, Box<dyn std::error::Error>> {
        info!("🔄 Generating new dynamic credential at path: {}", path);

        // Generate random credentials
        let username = format!("dyn_{}_{}", credential_type, Self::random_string(8));
        let password = Self::random_string(32);

        let now = Utc::now();
        let expires_at = now + chrono::Duration::seconds(self.default_ttl_seconds as i64);

        let credential = DynamicCredential {
            username: username.clone(),
            password: password.clone(),
            created_at: now,
            expires_at,
            credential_type: credential_type.to_string(),
        };

        // Store credentials in vault as JSON
        let cred_json = serde_json::to_string(&credential)?;
        self.vault.put_secret(path, &cred_json, Some(self.default_ttl_seconds), user).await?;

        // Add to local cache
        let mut creds = self.credentials.write().await;
        creds.push(credential.clone());

        self.audit_trail.log_credential_operation(user, "GENERATE", path, &username).await;
        info!(
            "✅ Dynamic credential generated (expires in {}s)",
            self.default_ttl_seconds
        );

        Ok(credential)
    }

    /// Get a valid credential, automatically generating a new one if needed
    pub async fn get_credential(
        &self,
        path: &str,
        credential_type: &str,
        user: &str,
    ) -> Result<DynamicCredential, Box<dyn std::error::Error>> {
        // Try to get from cache first
        let cached = {
            let creds = self.credentials.read().await;
            creds.iter()
                .find(|c| c.credential_type == credential_type && c.is_valid())
                .cloned()
        };

        if let Some(cred) = cached {
            if !cred.should_renew(self.renewal_threshold_seconds) {
                info!("✓ Using cached credential (valid for {:?})", cred.time_until_expiry());
                return Ok(cred);
            }
        }

        // Need to fetch or regenerate
        info!("🔄 Fetching/regenerating credential from vault...");
        
        // Try to retrieve from vault
        match self.vault.get_secret(path, user).await? {
            Some(cred_json) => {
                let credential: DynamicCredential = serde_json::from_str(&cred_json)?;
                
                if credential.is_valid() && !credential.should_renew(self.renewal_threshold_seconds) {
                    // Update cache
                    let mut creds = self.credentials.write().await;
                    creds.retain(|c| c.credential_type != credential_type);
                    creds.push(credential.clone());
                    
                    info!("✓ Retrieved valid credential from vault");
                    return Ok(credential);
                }
            }
            None => {
                info!("No existing credential found, generating new one");
            }
        }

        // Generate new credential
        self.generate_credential(path, credential_type, user).await
    }

    /// Revoke a specific credential
    pub async fn revoke_credential(
        &self,
        path: &str,
        user: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("🚫 Revoking credential at path: {}", path);

        // Remove from vault
        self.vault.delete_secret(path, user).await?;

        // Remove from cache
        let mut creds = self.credentials.write().await;
        let removed_count = creds.len();
        creds.clear();

        self.audit_trail.log_credential_operation(user, "REVOKE", path, "*").await;
        info!("🗑️  Credential revoked (removed {} from cache)", removed_count);

        Ok(())
    }

    /// Start background task to monitor and rotate expiring credentials
    pub async fn start_auto_rotation(self: std::sync::Arc<Self>) {
        info!("🔄 Starting automatic credential rotation monitor...");

        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;

            let creds_to_rotate = {
                let creds = self.credentials.read().await;
                creds.iter()
                    .filter(|c| c.should_renew(self.renewal_threshold_seconds))
                    .map(|c| c.credential_type.clone())
                    .collect::<Vec<_>>()
            };

            for cred_type in creds_to_rotate {
                info!("⏰ Credential type '{}' approaching expiry, rotating...", cred_type);
                // Note: In production, you'd need to track paths per credential type
                // This is simplified for demonstration
            }
        }
    }

    /// Get count of active credentials
    pub async fn active_credential_count(&self) -> usize {
        let creds = self.credentials.read().await;
        creds.iter().filter(|c| c.is_valid()).count()
    }

    /// Generate a random string for credentials
    fn random_string(length: usize) -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::thread_rng();
        (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::self_vault::SelfVault;

    #[tokio::test]
    async fn test_generate_and_retrieve_credential() {
        let key = SelfVault::generate_master_key();
        let vault = Arc::new(SelfVault::new(&key));
        let manager = DynamicCredentialsManager::new(vault, 3600, 300);

        let cred = manager.generate_credential("db/creds/app", "database", "admin")
            .await
            .unwrap();

        assert!(cred.is_valid());
        assert_eq!(cred.credential_type, "database");
        assert!(cred.username.starts_with("dyn_database_"));
        assert_eq!(cred.password.len(), 32);
    }

    #[tokio::test]
    async fn test_credential_expiry() {
        let key = SelfVault::generate_master_key();
        let vault = Arc::new(SelfVault::new(&key));
        let manager = DynamicCredentialsManager::new(vault, 2, 1); // 2 second TTL

        let cred = manager.generate_credential("db/creds/temp", "temp", "admin")
            .await
            .unwrap();

        assert!(cred.is_valid());

        // Wait for expiry
        tokio::time::sleep(Duration::from_secs(3)).await;

        assert!(!cred.is_valid());
    }
}
