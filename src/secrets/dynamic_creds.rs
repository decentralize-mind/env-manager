use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn};
use crate::secrets::vault::VaultClient;

/// Dynamic database credentials that auto-expire
#[derive(Debug, Clone)]
pub struct DbCredentials {
    pub username: String,
    pub password: String,
    pub expires_at: Instant,
}

impl DbCredentials {
    /// Check if credentials are still valid
    pub fn is_valid(&self) -> bool {
        Instant::now() < self.expires_at
    }

    /// Get time until expiry
    pub fn time_until_expiry(&self) -> Duration {
        self.expires_at.saturating_duration_since(Instant::now())
    }
}

/// Manager for dynamic database credentials with automatic rotation
pub struct DynamicDbCredsManager {
    vault_client: VaultClient,
    credentials: RwLock<Option<DbCredentials>>,
    lease_duration: Duration,
    renewal_threshold: Duration, // Renew when this much time is left
}

impl DynamicDbCredsManager {
    pub fn new(vault_client: VaultClient, lease_duration_secs: u64) -> Self {
        info!(
            "🗄️  Initializing dynamic DB credentials manager (lease: {}s)",
            lease_duration_secs
        );
        
        Self {
            vault_client,
            credentials: RwLock::new(None),
            lease_duration: Duration::from_secs(lease_duration_secs),
            renewal_threshold: Duration::from_secs(60), // Renew 60s before expiry
        }
    }

    /// Get current credentials, automatically renewing if needed
    pub async fn get_credentials(&self) -> Result<DbCredentials, Box<dyn std::error::Error>> {
        let creds = self.credentials.read().await;

        // Check if we have valid credentials
        if let Some(ref c) = *creds {
            if c.is_valid() && c.time_until_expiry() > self.renewal_threshold {
                info!("✓ Using cached DB credentials (valid for {:?})", c.time_until_expiry());
                return Ok(c.clone());
            }
        }
        drop(creds);

        // Need to fetch/renew credentials
        info!("🔄 Fetching new dynamic DB credentials from Vault...");
        self.renew_credentials().await
    }

    /// Force renewal of credentials
    async fn renew_credentials(&self) -> Result<DbCredentials, Box<dyn std::error::Error>> {
        // Fetch new credentials from Vault's database secrets engine
        let username = self.vault_client.get_secret("database/creds/app-role", "username").await?;
        let password = self.vault_client.get_secret("database/creds/app-role", "password").await?;

        let creds = DbCredentials {
            username,
            password,
            expires_at: Instant::now() + self.lease_duration,
        };

        info!(
            "✅ New DB credentials obtained (expires in {}s)",
            self.lease_duration.as_secs()
        );

        // Update cached credentials
        let mut write_lock = self.credentials.write().await;
        *write_lock = Some(creds.clone());

        Ok(creds)
    }

    /// Start background task to proactively renew credentials
    pub async fn start_auto_renewal(self: &std::sync::Arc<Self>) {
        info!("🔄 Starting automatic DB credential renewal...");
        
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            
            match self.get_credentials().await {
                Ok(creds) => {
                    info!(
                        "✓ Credentials valid for {:?}",
                        creds.time_until_expiry()
                    );
                }
                Err(e) => {
                    warn!("⚠️  Failed to renew DB credentials: {}", e);
                }
            }
        }
    }
}

/// Connection string builder with dynamic credentials
pub struct DbConnectionString {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub ssl_mode: String,
}

impl DbConnectionString {
    pub fn new(host: &str, port: u16, database: &str, ssl_mode: &str) -> Self {
        Self {
            host: host.to_string(),
            port,
            database: database.to_string(),
            ssl_mode: ssl_mode.to_string(),
        }
    }

    pub async fn build_with_dynamic_creds(
        &self,
        manager: &DynamicDbCredsManager,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let creds = manager.get_credentials().await?;
        
        Ok(format!(
            "postgresql://{}:{}@{}:{}/{}?sslmode={}",
            creds.username, creds.password, self.host, self.port, self.database, self.ssl_mode
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credentials_expiry() {
        let creds = DbCredentials {
            username: "test".to_string(),
            password: "test".to_string(),
            expires_at: Instant::now() + Duration::from_secs(3600),
        };
        
        assert!(creds.is_valid());
        assert!(creds.time_until_expiry() <= Duration::from_secs(3600));
    }
}
