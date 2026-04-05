mod config;
mod secrets;
mod security;
mod utils;

use config::{loader::load_config, validator::validate, advanced::AdvancedConfig};
use secrets::vault::VaultClient;
use utils::secure_env::{SecureEnvManager, Environment};
use utils::metrics::get_metrics;
use tracing::info;
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
