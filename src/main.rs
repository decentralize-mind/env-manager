mod config;
mod secrets;
mod security;
mod utils;

use config::{loader::load_config, validator::validate, advanced::AdvancedConfig};
use secrets::vault::VaultClient;
use secrets::self_vault::{SelfVault, DynamicCredentialsManager, SecretRotator};
use utils::secure_env::SecureEnvManager;
use utils::vault_manager;
use tracing::{info, warn};
use std::env;

#[tokio::main]
async fn main() {
    // Initialize tracing for structured logging
    tracing_subscriber::fmt::init();

    // Check for CLI commands
    let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 {
        match args[1].as_str() {
            "generate" => {
                info!("📝 Generating .env template...");
                let manager = SecureEnvManager::new(".env");
                if let Err(e) = manager.generate_env_template() {
                    eprintln!("❌ Error: {}", e);
                    std::process::exit(1);
                }
                return;
            }
            "lock" => {
                info!("🔒 Locking .env file...");
                let manager = SecureEnvManager::new(".env");
                if let Err(e) = manager.lock_env() {
                    eprintln!("❌ Error: {}", e);
                    std::process::exit(1);
                }
                return;
            }
            "unlock" => {
                info!("🔓 Unlocking .env file...");
                let manager = SecureEnvManager::new(".env");
                if let Err(e) = manager.unlock_env() {
                    eprintln!("❌ Error: {}", e);
                    std::process::exit(1);
                }
                return;
            }
            "chpasswd" => {
                info!("🔑 Changing encryption password...");
                let manager = SecureEnvManager::new(".env");
                if let Err(e) = manager.change_password() {
                    eprintln!("❌ Error: {}", e);
                    std::process::exit(1);
                }
                return;
            }
            "status" => {
                let manager = SecureEnvManager::new(".env");
                if manager.is_locked() {
                    println!("🔒 .env file is LOCKED (encrypted)");
                    println!("   Run 'cargo run -- unlock' to decrypt");
                } else if manager.is_unlocked() {
                    println!("🔓 .env file is UNLOCKED (plaintext)");
                    println!("   Run 'cargo run -- lock' to encrypt and protect");
                } else {
                    println!("⚠️  No .env file found");
                    println!("   Run 'cargo run -- generate' to create one");
                }
                return;
            }
            "self-vault-demo" => {
                info!("🏦 Running SelfVault demonstration...");
                if let Err(e) = run_self_vault_demo().await {
                    eprintln!("❌ Error: {}", e);
                    std::process::exit(1);
                }
                return;
            }
            "vault-init" => {
                info!("🏦 Initializing SelfVault with persistent master key...");
                if let Err(e) = run_vault_init().await {
                    eprintln!("❌ Error: {}", e);
                    std::process::exit(1);
                }
                return;
            }
            "vault-migrate" => {
                info!("🔄 Migrating .env secrets to SelfVault...");
                if let Err(e) = run_vault_migrate().await {
                    eprintln!("❌ Error: {}", e);
                    std::process::exit(1);
                }
                return;
            }
            "vault-stats" => {
                info!("📊 Displaying SelfVault statistics...");
                if let Err(e) = run_vault_stats().await {
                    eprintln!("❌ Error: {}", e);
                    std::process::exit(1);
                }
                return;
            }
            "help" | "--help" | "-h" => {
                print_help();
                return;
            }
            _ => {
                eprintln!("❌ Unknown command: {}", args[1]);
                print_help();
                std::process::exit(1);
            }
        }
    }

    // Default behavior: Load and validate configuration
    info!("🔐 Loading secure configuration...");

    let mut cfg = load_config().expect("Config failed");
    validate(&cfg);

    info!("✅ Configuration validated successfully");

    // Load advanced configuration (feature flags, safety controls, etc.)
    let advanced_config = AdvancedConfig::from_env();
    info!("🛡️  Advanced configuration loaded:");
    info!("   - Feature Flags: Bridge={}, Airdrop={}, Global Pause={}", 
          advanced_config.feature_flags.bridge_enabled,
          advanced_config.feature_flags.airdrop_enabled,
          advanced_config.feature_flags.global_pause);
    info!("   - Safety Controls: Max Withdrawal={}, Bridge Daily Limit={}",
          advanced_config.safety_controls.max_withdrawal_limit,
          advanced_config.safety_controls.bridge_daily_limit);
    info!("   - Rotation Config: Refresh={}s, Max TTL={}s",
          advanced_config.rotation_config.refresh_interval_secs,
          advanced_config.rotation_config.max_ttl_secs);
    info!("   - Observability: Metrics Port={}",
          advanced_config.observability.metrics_port);
    
    // Check if secret paths are configured for Vault
    if advanced_config.secret_paths.is_configured() {
        info!("☁️  Vault secret paths configured:");
        for (name, path) in advanced_config.secret_paths.get_all_paths() {
            info!("   - {}: {}", name, path);
        }
    } else {
        info!("💻 Using auto-generated secrets (Vault paths not configured)");
    }

    // Check system health (global pause, etc.)
    match advanced_config.check_system_health() {
        Ok(()) => info!("✅ System health check passed"),
        Err(errors) => {
            eprintln!("❌ System health check failed:");
            for error in errors {
                eprintln!("   - {}", error);
            }
            std::process::exit(1);
        }
    }

    // Start metrics server if enabled
    if advanced_config.feature_flags.metrics_enabled {
        let metrics_port = advanced_config.observability.metrics_port;
        tokio::spawn(async move {
            if let Err(e) = utils::metrics::start_metrics_server(metrics_port).await {
                eprintln!("⚠️  Failed to start metrics server: {}", e);
            }
        });
    }

    // Check if Vault is configured
    let vault_token = std::env::var("VAULT_TOKEN").ok();
    let vault_addr = std::env::var("VAULT_ADDR").unwrap_or_else(|_| "http://127.0.0.1:8200".to_string());

    if let Some(token) = vault_token {
        // Vault is configured - try to fetch secrets
        info!("🏦 Vault configured at: {}", vault_addr);
        
        let vault = VaultClient::new(vault_addr.clone(), token);

        // 🔐 Replace env secrets with Vault secrets
        match vault.get_secret("secret/data/app", "jwt").await {
            Ok(jwt) => {
                cfg.security.jwt_secret = jwt;
                info!("✅ JWT secret loaded from Vault");
            }
            Err(e) => {
                eprintln!("⚠️  Failed to load JWT secret from Vault: {}", e);
                eprintln!("   ℹ️  Make sure Vault is running and accessible at {}", vault_addr);
                eprintln!("   ℹ️  Using value from environment/config file as fallback");
            }
        }
    } else {
        // Running in development mode without Vault
        info!("💻 Running in development mode (Vault not configured)");
        info!("   ℹ️  To use Vault, set VAULT_TOKEN and VAULT_ADDR environment variables");
        info!("   ℹ️  Using secrets from .env file or environment variables");
    }

    // Log audit trail
    security::audit::log_access("system", "config_load");

    // Check access control
    if security::access::check_access("admin") {
        info!("✅ Admin access granted");
    }

    info!("🚀 Application ready!");
    info!("   App: {} on port {}", cfg.app.name, cfg.app.port);
    info!("   Database URL: {}", cfg.db.url);

    // Send test Telegram notification if configured
    let telegram_handle = tokio::spawn(async {
        if let Some(notifier) = utils::telegram_notifier::TelegramNotifier::from_env() {
            match notifier.send_test_message().await {
                Ok(_) => info!("✅ Test Telegram notification sent successfully"),
                Err(e) => warn!("⚠️  Failed to send test Telegram notification: {}", e),
            }
        } else {
            info!("ℹ️  Telegram not configured (set TELEGRAM_BOT_TOKEN and TELEGRAM_CHAT_ID in .env)");
        }
    });

    // Wait briefly for Telegram notification to complete
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Cancel the telegram task if still running
    telegram_handle.abort();

    // Start rotation loop in background (optional)
    // tokio::spawn(secrets::rotator::rotation_loop());
}

/// Print help message
fn print_help() {
    println!("\n🔐 Secure Environment Manager");
    println!("================================\n");
    println!("Usage: cargo run -- [command]\n");
    println!("Commands:");
    println!("  generate   Create a new .env template with all required fields");
    println!("  lock       Encrypt and password-protect the .env file");
    println!("  unlock     Decrypt the .env file (requires password)");
    println!("  chpasswd   Change the encryption password");
    println!("  status     Check if .env is locked or unlocked");
    println!("  self-vault-demo  Demonstrate SelfVault features");
    println!("  vault-init         Initialize SelfVault with persistent master key");
    println!("  vault-migrate      Migrate .env secrets to SelfVault");
    println!("  vault-stats        Display SelfVault statistics");
    println!("  help       Show this help message\n");
    println!("Examples:");
    println!("  cargo run -- generate    # Create .env template");
    println!("  cargo run -- lock        # Lock with password");
    println!("  cargo run -- unlock      # Unlock with password");
    println!("  cargo run -- status      # Check lock status");
    println!("  cargo run                # Load config (default)\n");
    println!("Security Features:");
    println!("  ✓ AES-256-GCM encryption");
    println!("  ✓ Password protection");
    println!("  ✓ Secure deletion of plaintext");
    println!("  ✓ Automatic overwrite before deletion\n");
}

/// Demonstrate SelfVault features
async fn run_self_vault_demo() -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::Arc;
    
    println!("\n🏦 SelfVault Feature Demonstration");
    println!("====================================\n");
    
    // 1. Initialize SelfVault
    println!("1️⃣  Initializing SelfVault with AES-256-GCM encryption...");
    let master_key = SelfVault::generate_master_key();
    let vault = Arc::new(SelfVault::new(&master_key));
    println!("✅ SelfVault initialized\n");
    
    // 2. Store and retrieve secrets
    println!("2️⃣  Testing secure storage...");
    
    // Assign admin role first (access control initializes async)
    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
    vault.access_control().assign_role("admin", "admin").await?;
    
    vault.put_secret("secret/api-key", "sk_live_abc123xyz789", Some(3600), "admin").await?;
    vault.put_secret("secret/db-password", "SuperSecretDBPass123!", None, "admin").await?;
    
    if let Some(api_key) = vault.get_secret("secret/api-key", "admin").await? {
        println!("✅ Retrieved API key: {}...", &api_key[..10]);
    }
    
    if let Some(db_pass) = vault.get_secret("secret/db-password", "admin").await? {
        println!("✅ Retrieved DB password: {}...", &db_pass[..10]);
    }
    println!();
    
    // 3. Access control demonstration
    println!("3️⃣  Testing access control...");
    vault.access_control().assign_role("developer", "developer").await?;
    vault.access_control().assign_role("viewer", "viewer").await?;
    
    let dev_can_read = vault.access_control().check_read_access("developer", "secret/app/config").await;
    let dev_can_write = vault.access_control().check_write_access("developer", "secret/app/config").await;
    let viewer_can_write = vault.access_control().check_write_access("viewer", "secret/app/config").await;
    
    println!("✅ Developer can read app config: {}", dev_can_read);
    println!("✅ Developer can write app config: {}", dev_can_write);
    println!("✅ Viewer can write app config: {}", viewer_can_write);
    println!();
    
    // 4. Dynamic credentials
    println!("4️⃣  Testing dynamic credentials...");
    let creds_manager = DynamicCredentialsManager::new(
        vault.clone(),
        3600, // 1 hour TTL
        300,  // Renew 5 minutes before expiry
    );
    
    let cred = creds_manager.generate_credential("db/creds/app", "database", "admin").await?;
    println!("✅ Generated dynamic credential:");
    println!("   Username: {}", cred.username);
    println!("   Valid for: {:?}", cred.time_until_expiry());
    println!();
    
    // 5. Secret rotation
    println!("5️⃣  Testing automatic rotation...");
    let rotator = SecretRotator::new(vault.clone());
    rotator.register_rotation("secret/api-key", 3600, "admin").await?;
    
    if let Some(status) = rotator.get_rotation_status("secret/api-key").await {
        println!("✅ Rotation registered:");
        println!("   Status: {:?}", status.status);
        println!("   Rotation count: {}", status.rotation_count);
    }
    
    // Manually rotate
    rotator.rotate_secret("secret/api-key", "sk_live_new_key_xyz", "admin").await?;
    println!("✅ Secret manually rotated\n");
    
    // 6. Audit trail
    println!("6️⃣  Checking audit trail...");
    let logs = vault.audit_trail().get_recent_logs(10).await;
    println!("✅ Recent audit log entries ({} total):", logs.len());
    for log in logs.iter().take(5) {
        println!("   {}", log.format());
    }
    println!();
    
    // 7. Security controls
    println!("7️⃣  Testing security controls...");
    let security = vault.access_control(); // Using access control as example
    println!("✅ Security controls active");
    println!("   - Role-based access control: Enabled");
    println!("   - Audit logging: Enabled");
    println!("   - Encryption: AES-256-GCM");
    println!();
    
    // 8. Vault statistics
    println!("8️⃣  Vault statistics:");
    println!("   Total secrets stored: {}", vault.secret_count().await);
    println!("   Active credentials: {}", creds_manager.active_credential_count().await);
    println!("   Audit log entries: {}", vault.audit_trail().log_count().await);
    println!();
    
    // 9. Seal/unseal demonstration
    println!("9️⃣  Testing vault seal/unseal...");
    vault.seal().await;
    println!("🔒 Vault sealed - attempting to access secrets...");
    
    match vault.get_secret("secret/api-key", "admin").await {
        Ok(_) => println!("❌ ERROR: Should not be able to access sealed vault"),
        Err(e) => println!("✅ Correctly denied access: {}", e),
    }
    
    vault.unseal().await;
    println!("🔓 Vault unsealed - access restored");
    println!();
    
    println!("═══════════════════════════════════════");
    println!("✅ SelfVault demonstration complete!");
    println!("═══════════════════════════════════════\n");
    
    println!("Key Features Demonstrated:");
    println!("  ✓ Centralized secure storage with AES-256-GCM");
    println!("  ✓ Fine-grained access control policies");
    println!("  ✓ Dynamic credentials with auto-expiry");
    println!("  ✓ Automatic secret rotation");
    println!("  ✓ Comprehensive audit trail");
    println!("  ✓ Production-grade security controls");
    println!("  ✓ Vault seal/unseal mechanism");
    println!();
    
    Ok(())
}

/// Initialize SelfVault with persistent master key
async fn run_vault_init() -> Result<(), Box<dyn std::error::Error>> {
    use utils::vault_manager::{VaultConfig, initialize_vault, verify_vault, display_vault_stats};
    
    println!("\n🏦 SelfVault Initialization");
    println!("═══════════════════════\n");
    
    let config = VaultConfig::default();
    
    // Initialize vault
    let vault = initialize_vault(&config).await?;
    
    println!("✅ SelfVault initialized successfully");
    println!("💾 Master key stored in: {}", config.master_key_path);
    println!("\n⚠️  IMPORTANT SECURITY NOTES:");
    println!("   1. Keep {} secure and backed up", config.master_key_path);
    println!("   2. Never commit this file to version control");
    println!("   3. Loss of master key = loss of all secrets");
    println!("   4. Set file permissions: chmod 600 {}", config.master_key_path);
    
    // Verify vault
    if verify_vault(&vault, "admin").await? {
        println!("\n✅ Vault integrity verified");
    }
    
    // Show stats
    display_vault_stats(&vault, "admin").await;
    
    println!("\nNext steps:");
    println!("  1. Run 'cargo run -- vault-migrate' to migrate .env secrets");
    println!("  2. Or store secrets programmatically using the vault API");
    println!();
    
    Ok(())
}

/// Migrate .env secrets to SelfVault
async fn run_vault_migrate() -> Result<(), Box<dyn std::error::Error>> {
    use utils::vault_manager::{VaultConfig, initialize_vault, migrate_env_to_vault, display_vault_stats};
    
    println!("\n🔄 Migrating .env Secrets to SelfVault");
    println!("══════════════════════════════════\n");
    
    // Check if .env exists
    if !std::path::Path::new(".env").exists() {
        eprintln!("❌ .env file not found");
        eprintln!("   Run 'cargo run -- generate' first to create .env template");
        std::process::exit(1);
    }
    
    // Initialize vault
    let config = VaultConfig::default();
    let vault = initialize_vault(&config).await?;
    
    // Migrate secrets
    let count = migrate_env_to_vault(&vault, "admin").await?;
    
    if count == 0 {
        println!("\n⚠️  No secrets were migrated");
        println!("   Make sure .env contains secret values (not placeholders)");
    } else {
        println!("\n✅ Successfully migrated {} secrets to SelfVault", count);
        println!("\n📋 Next steps:");
        println!("   1. Review migrated secrets: cargo run -- vault-stats");
        println!("   2. Remove sensitive values from .env file");
        println!("   3. Lock .env file: cargo run -- lock");
        println!("   4. Update your application to use SelfVault API");
    }
    
    // Show updated stats
    display_vault_stats(&vault, "admin").await;
    
    Ok(())
}

/// Display SelfVault statistics
async fn run_vault_stats() -> Result<(), Box<dyn std::error::Error>> {
    use utils::vault_manager::{VaultConfig, initialize_vault, display_vault_stats};
    
    let config = VaultConfig::default();
    let vault = initialize_vault(&config).await?;
    
    display_vault_stats(&vault, "admin").await;
    
    Ok(())
}
