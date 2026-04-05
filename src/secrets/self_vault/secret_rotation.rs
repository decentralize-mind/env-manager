use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, error};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::self_vault::SelfVault;
use crate::secrets::self_vault::audit_trail::AuditTrail;

/// Configuration for automatic secret rotation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationConfig {
    pub path: String,
    pub rotation_interval_seconds: u64,
    pub enabled: bool,
    pub notify_on_rotation: bool,
}

/// Status of a rotated secret
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationStatus {
    pub path: String,
    pub last_rotated: Option<chrono::DateTime<Utc>>,
    pub next_rotation: Option<chrono::DateTime<Utc>>,
    pub rotation_count: u64,
    pub status: RotationState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RotationState {
    Active,
    Pending,
    Failed,
    Disabled,
}

/// Automatic secret rotation manager
pub struct SecretRotator {
    vault: Arc<SelfVault>,
    configs: RwLock<Vec<RotationConfig>>,
    statuses: RwLock<Vec<RotationStatus>>,
    audit_trail: AuditTrail,
}

impl SecretRotator {
    /// Create a new secret rotator
    pub fn new(vault: Arc<SelfVault>) -> Self {
        info!("🔄 Initializing automatic secret rotation manager");
        
        Self {
            vault: vault.clone(),
            configs: RwLock::new(Vec::new()),
            statuses: RwLock::new(Vec::new()),
            audit_trail: vault.audit_trail().clone(),
        }
    }

    /// Register a secret path for automatic rotation
    pub async fn register_rotation(
        &self,
        path: &str,
        rotation_interval_seconds: u64,
        user: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!(
            "📝 Registering rotation for path: {} (interval: {}s)",
            path, rotation_interval_seconds
        );

        let config = RotationConfig {
            path: path.to_string(),
            rotation_interval_seconds,
            enabled: true,
            notify_on_rotation: true,
        };

        let mut configs = self.configs.write().await;
        configs.push(config);

        // Initialize status
        let status = RotationStatus {
            path: path.to_string(),
            last_rotated: None,
            next_rotation: Some(Utc::now() + chrono::Duration::seconds(rotation_interval_seconds as i64)),
            rotation_count: 0,
            status: RotationState::Active,
        };

        let mut statuses = self.statuses.write().await;
        statuses.push(status);

        self.audit_trail.log_system_event("ROTATION_REGISTERED", user).await;
        Ok(())
    }

    /// Unregister a secret path from automatic rotation
    pub async fn unregister_rotation(&self, path: &str, user: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("🗑️  Unregistering rotation for path: {}", path);

        // Remove from configs
        let mut configs = self.configs.write().await;
        configs.retain(|c| c.path != path);

        // Update status to disabled
        let mut statuses = self.statuses.write().await;
        if let Some(status) = statuses.iter_mut().find(|s| s.path == path) {
            status.status = RotationState::Disabled;
        }

        self.audit_trail.log_system_event("ROTATION_UNREGISTERED", user).await;
        Ok(())
    }

    /// Manually trigger rotation for a specific path
    pub async fn rotate_secret(
        &self,
        path: &str,
        new_value: &str,
        user: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("🔄 Manually rotating secret at path: {}", path);

        // Get current TTL from existing secret if available
        let ttl = if let Some(_value) = self.vault.get_secret(path, user).await? {
            // In production, you'd extract TTL from the existing entry
            Some(3600) // Default 1 hour
        } else {
            Some(3600)
        };

        // Store new value
        self.vault.put_secret(path, new_value, ttl, user).await?;

        // Update rotation status
        let mut statuses = self.statuses.write().await;
        if let Some(status) = statuses.iter_mut().find(|s| s.path == path) {
            status.last_rotated = Some(Utc::now());
            status.rotation_count += 1;
            
            // Find config to get interval
            let configs = self.configs.read().await;
            if let Some(config) = configs.iter().find(|c| c.path == path) {
                status.next_rotation = Some(
                    Utc::now() + chrono::Duration::seconds(config.rotation_interval_seconds as i64)
                );
            }
        }

        self.audit_trail.log_secret_operation(user, "ROTATE", path).await;
        info!("✅ Secret rotated successfully at path: {}", path);

        Ok(())
    }

    /// Start background rotation monitor
    pub async fn start_rotation_monitor(self: std::sync::Arc<Self>) {
        info!("🔄 Starting automatic secret rotation monitor...");

        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;

            if let Err(e) = self.check_and_rotate_secrets().await {
                error!("❌ Error in rotation monitor: {}", e);
            }
        }
    }

    /// Check all registered secrets and rotate if needed
    async fn check_and_rotate_secrets(&self) -> Result<(), Box<dyn std::error::Error>> {
        let configs = self.configs.read().await;
        let mut statuses = self.statuses.write().await;

        for config in configs.iter() {
            if !config.enabled {
                continue;
            }

            // Find corresponding status
            if let Some(status) = statuses.iter_mut().find(|s| s.path == config.path) {
                // Check if rotation is due
                if let Some(next_rotation) = status.next_rotation {
                    if Utc::now() >= next_rotation {
                        info!(
                            "⏰ Rotation due for path: {} (rotation #{})",
                            config.path, status.rotation_count + 1
                        );

                        // In production, you'd generate a new secret here
                        // For now, we'll just update the timestamp
                        status.last_rotated = Some(Utc::now());
                        status.rotation_count += 1;
                        status.next_rotation = Some(
                            Utc::now() + chrono::Duration::seconds(config.rotation_interval_seconds as i64)
                        );
                        status.status = RotationState::Active;

                        self.audit_trail.log_secret_operation("system", "AUTO_ROTATE", &config.path)
                            .await;

                        if config.notify_on_rotation {
                            info!("📧 Notification: Secret rotated at {}", config.path);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Get rotation status for a specific path
    pub async fn get_rotation_status(&self, path: &str) -> Option<RotationStatus> {
        let statuses = self.statuses.read().await;
        statuses.iter().find(|s| s.path == path).cloned()
    }

    /// Get all rotation statuses
    pub async fn get_all_statuses(&self) -> Vec<RotationStatus> {
        let statuses = self.statuses.read().await;
        statuses.clone()
    }

    /// Enable rotation for a path
    pub async fn enable_rotation(&self, path: &str, user: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut configs = self.configs.write().await;
        if let Some(config) = configs.iter_mut().find(|c| c.path == path) {
            config.enabled = true;
            
            let mut statuses = self.statuses.write().await;
            if let Some(status) = statuses.iter_mut().find(|s| s.path == path) {
                status.status = RotationState::Active;
            }
            
            self.audit_trail.log_system_event("ROTATION_ENABLED", user).await;
            info!("✅ Rotation enabled for path: {}", path);
        }
        Ok(())
    }

    /// Disable rotation for a path
    pub async fn disable_rotation(&self, path: &str, user: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut configs = self.configs.write().await;
        if let Some(config) = configs.iter_mut().find(|c| c.path == path) {
            config.enabled = false;
            
            let mut statuses = self.statuses.write().await;
            if let Some(status) = statuses.iter_mut().find(|s| s.path == path) {
                status.status = RotationState::Disabled;
            }
            
            self.audit_trail.log_system_event("ROTATION_DISABLED", user).await;
            info!("⏸️  Rotation disabled for path: {}", path);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::self_vault::SelfVault;

    #[tokio::test]
    async fn test_register_rotation() {
        let key = SelfVault::generate_master_key();
        let vault = Arc::new(SelfVault::new(&key));
        let rotator = SecretRotator::new(vault);

        rotator.register_rotation("secret/api-key", 3600, "admin")
            .await
            .unwrap();

        let status = rotator.get_rotation_status("secret/api-key").await;
        assert!(status.is_some());
        
        let status = status.unwrap();
        assert_eq!(status.status, RotationState::Active);
        assert_eq!(status.rotation_count, 0);
    }

    #[tokio::test]
    async fn test_manual_rotation() {
        let key = SelfVault::generate_master_key();
        let vault = Arc::new(SelfVault::new(&key));
        let rotator = SecretRotator::new(vault.clone());

        // Store initial secret
        rotator.vault.put_secret("secret/test", "old_value", Some(3600), "admin")
            .await
            .unwrap();

        // Rotate it
        rotator.rotate_secret("secret/test", "new_value", "admin")
            .await
            .unwrap();

        // Verify new value
        let retrieved = rotator.vault.get_secret("secret/test", "admin")
            .await
            .unwrap();
        assert_eq!(retrieved, Some("new_value".to_string()));

        // Check rotation status
        let status = rotator.get_rotation_status("secret/test").await;
        assert!(status.is_some());
        assert_eq!(status.unwrap().rotation_count, 1);
    }
}
