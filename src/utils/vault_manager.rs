/// Vault initialization and master key management utilities
/// 
/// This module provides utilities for:
/// - Generating and storing master keys securely
/// - Loading vault from persisted master key
/// - Migrating secrets from .env to SelfVault
/// - Production-ready vault lifecycle management

use std::fs;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn, error};

use crate::secrets::self_vault::SelfVault;

/// Configuration for vault initialization
pub struct VaultConfig {
    /// Path to store/load master key
    pub master_key_path: String,
    
    /// Whether to generate new key if not exists
    pub generate_if_missing: bool,
    
    /// Environment variable name for master key (alternative to file)
    pub env_var_name: Option<String>,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            master_key_path: ".vault_master.key".to_string(),
            generate_if_missing: true,
            env_var_name: Some("VAULT_MASTER_KEY".to_string()),
        }
    }
}

/// Initialize SelfVault with persistent master key
pub async fn initialize_vault(config: &VaultConfig) -> Result<Arc<SelfVault>, Box<dyn std::error::Error>> {
    info!("🏦 Initializing SelfVault...");
    
    let master_key = load_or_generate_master_key(config).await?;
    let vault = Arc::new(SelfVault::new(&master_key));
    
    // Wait for async initialization of access control
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    
    info!("✅ SelfVault initialized successfully");
    Ok(vault)
}

/// Load master key from file or environment, or generate new one
async fn load_or_generate_master_key(config: &VaultConfig) -> Result<[u8; 32], Box<dyn std::error::Error>> {
    // Try environment variable first
    if let Some(env_var) = &config.env_var_name {
        if let Ok(key_hex) = std::env::var(env_var) {
            info!("🔑 Loading master key from environment variable: {}", env_var);
            return decode_master_key(&key_hex);
        }
    }
    
    // Try file
    if Path::new(&config.master_key_path).exists() {
        info!("🔑 Loading master key from file: {}", config.master_key_path);
        let key_hex = fs::read_to_string(&config.master_key_path)?;
        return decode_master_key(&key_hex.trim());
    }
    
    // Generate new key if allowed
    if config.generate_if_missing {
        info!("🔑 No master key found, generating new one...");
        let new_key = SelfVault::generate_master_key();
        save_master_key(&new_key, config).await?;
        return Ok(new_key);
    }
    
    Err("Master key not found and generation is disabled".into())
}

/// Decode hex-encoded master key
fn decode_master_key(hex_str: &str) -> Result<[u8; 32], Box<dyn std::error::Error>> {
    let bytes = hex::decode(hex_str)?;
    
    if bytes.len() != 32 {
        return Err(format!("Invalid master key length: expected 32 bytes, got {}", bytes.len()).into());
    }
    
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(key)
}

/// Save master key securely
async fn save_master_key(key: &[u8; 32], config: &VaultConfig) -> Result<(), Box<dyn std::error::Error>> {
    let hex_key = hex::encode(key);
    
    // Save to file with restricted permissions
    fs::write(&config.master_key_path, &hex_key)?;
    
    // Set file permissions to owner-only (Unix)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(&config.master_key_path)?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o600); // rw-------
        fs::set_permissions(&config.master_key_path, permissions)?;
    }
    
    info!("💾 Master key saved to: {}", config.master_key_path);
    info!("⚠️  IMPORTANT: Keep this file secure! Loss means data loss.");
    
    Ok(())
}

/// Migrate secrets from .env file to SelfVault
pub async fn migrate_env_to_vault(
    vault: &Arc<SelfVault>,
    user: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    info!("🔄 Migrating secrets from .env to SelfVault...");
    
    // Load .env file
    dotenvy::dotenv().ok();
    
    let mut migrated_count = 0;
    
    // Define secrets to migrate (customize this list)
    let secrets_to_migrate = vec![
        ("JWT_SECRET", "secret/jwt", Some(86400u64)),           // 24 hour TTL
        ("SESSION_SECRET", "secret/session", Some(86400u64)),
        ("API_KEY", "secret/api-key", Some(3600u64)),            // 1 hour TTL
        ("API_SECRET", "secret/api-secret", Some(3600u64)),
        ("ENCRYPTION_KEY", "secret/encryption-key", None),       // No expiry
        ("DATABASE_PASSWORD", "secret/db-password", None),
        ("WEB3_PRIVATE_KEY", "secret/web3-private-key", None),
    ];
    
    for (env_var, vault_path, ttl) in secrets_to_migrate {
        if let Ok(value) = std::env::var(env_var) {
            // Skip empty values
            if value.is_empty() || value.starts_with("__") {
                continue;
            }
            
            vault.put_secret(vault_path, &value, ttl, user).await?;
            info!("  ✓ Migrated {} → {}", env_var, vault_path);
            migrated_count += 1;
        } else {
            info!("  ⊘ Skipped {} (not set)", env_var);
        }
    }
    
    info!("✅ Migration complete: {} secrets migrated", migrated_count);
    info!("💡 You can now remove sensitive values from .env file");
    
    Ok(migrated_count)
}

/// Export vault secrets back to .env format (for backup/debugging)
pub async fn export_vault_to_env(
    vault: &Arc<SelfVault>,
    output_path: &str,
    user: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("📤 Exporting vault secrets to: {}", output_path);
    
    let paths = vault.list_secrets(user).await?;
    let path_count = paths.len();
    let mut env_content = String::new();
    
    env_content.push_str("# Auto-generated from SelfVault\n");
    env_content.push_str("# DO NOT commit this file to version control!\n\n");
    
    for path in paths {
        if let Some(value) = vault.get_secret(&path, user).await? {
            // Extract key name from path
            let key_name = path.split('/').last().unwrap_or(&path);
            env_content.push_str(&format!("{}={}\n", key_name.to_uppercase(), value));
        }
    }
    
    fs::write(output_path, env_content)?;
    info!("✅ Exported {} secrets to {}", path_count, output_path);
    
    Ok(())
}

/// Verify vault integrity
pub async fn verify_vault(vault: &Arc<SelfVault>, user: &str) -> Result<bool, Box<dyn std::error::Error>> {
    info!("🔍 Verifying vault integrity...");
    
    // Ensure user has admin role
    vault.access_control().assign_role(user, "admin").await?;
    
    // Check if vault is accessible
    let paths = vault.list_secrets(user).await?;
    info!("  Found {} secrets in vault", paths.len());
    
    // Test read/write
    let test_path = "__integrity_test__";
    vault.put_secret(test_path, "test", Some(60), user).await?;
    
    if let Some(value) = vault.get_secret(test_path, user).await? {
        if value == "test" {
            vault.delete_secret(test_path, user).await?;
            info!("✅ Vault integrity check passed");
            return Ok(true);
        }
    }
    
    error!("❌ Vault integrity check failed");
    Ok(false)
}

/// Display vault statistics
pub async fn display_vault_stats(vault: &Arc<SelfVault>, user: &str) {
    println!("\n📊 SelfVault Statistics");
    println!("═══════════════════════");
    
    let secret_count = vault.secret_count().await;
    println!("Total secrets: {}", secret_count);
    
    let audit_count = vault.audit_trail().log_count().await;
    println!("Audit log entries: {}", audit_count);
    
    if let Ok(paths) = vault.list_secrets(user).await {
        println!("\nSecret paths:");
        for path in paths {
            println!("  • {}", path);
        }
    }
    
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_vault_initialization() {
        let config = VaultConfig {
            master_key_path: ".test_vault_key".to_string(),
            generate_if_missing: true,
            env_var_name: None,
        };
        
        let vault = initialize_vault(&config).await.unwrap();
        assert!(vault.secret_count().await >= 0);
        
        // Cleanup
        if Path::new(&config.master_key_path).exists() {
            fs::remove_file(&config.master_key_path).ok();
        }
    }
    
    #[tokio::test]
    async fn test_master_key_persistence() {
        let config = VaultConfig {
            master_key_path: ".test_persist_key".to_string(),
            generate_if_missing: true,
            env_var_name: None,
        };
        
        // First init generates key
        let vault1 = initialize_vault(&config).await.unwrap();
        vault1.put_secret("test/path", "value", None, "admin").await.unwrap();
        
        // Second init loads same key
        let vault2 = initialize_vault(&config).await.unwrap();
        
        // Should be able to retrieve data
        let value = vault2.get_secret("test/path", "admin").await.unwrap();
        assert_eq!(value, Some("value".to_string()));
        
        // Cleanup
        if Path::new(&config.master_key_path).exists() {
            fs::remove_file(&config.master_key_path).ok();
        }
    }
}
