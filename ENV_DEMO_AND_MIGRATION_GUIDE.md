# 🔄 Using .env.demo and Migrating to SelfVault

This guide shows you how to use the demo configuration file and migrate your secrets to SelfVault for enhanced security.

---

## Part 1: Using .env.demo File

### What is .env.demo?

The `.env.demo` file is a **pre-configured demonstration environment** with all features enabled. It's perfect for:
- Testing all env-manager features
- Understanding the configuration structure
- Development and experimentation
- Demo purposes

### Location

```
/Users/macbookpri/Documents/env-manager/.env.demo
```

### Contents Overview

The `.env.demo` file includes:
- ✅ Application settings (name, port)
- ✅ Database configuration
- ✅ Security keys (JWT, encryption)
- ✅ All feature flags enabled
- ✅ Safety controls configured
- ✅ Secret rotation settings
- ✅ Observability settings
- ✅ Vault path pointers

### How to Use .env.demo

#### Option 1: Copy to Active .env File

```bash
# Copy demo config to active .env
cp .env.demo .env

# Run the application
cargo run

# Or with the binary (after installation)
./target/release/env-manager
```

#### Option 2: Use as Template

```bash
# View the demo configuration
cat .env.demo

# Copy specific sections you need
nano .env
# Paste relevant sections from .env.demo
```

#### Option 3: Test Without Modifying Current .env

```bash
# Temporarily use demo config
cp .env .env.backup
cp .env.demo .env

# Test the application
cargo run

# Restore your original config
cp .env.backup .env
rm .env.backup
```

### Running with Demo Configuration

```bash
cd /Users/macbookpri/Documents/env-manager

# Method 1: Direct copy
cp .env.demo .env
cargo run

# Method 2: Environment variable override
APP__APP__NAME=DemoApp \
APP__APP__PORT=8080 \
cargo run
```

### Expected Output

When running with `.env.demo`, you should see:

```
✅ Configuration loaded successfully
🚀 Application ready!
   App: DemoApp on port 8080
   Database URL: postgresql://demo:demo@localhost:5432/demo
   
⚠️  Failed to load JWT secret from Vault: ...
   ℹ️  Using value from environment/config file as fallback
   
💻 Running in development mode (Vault not configured)
   ℹ️  Using secrets from .env file or environment variables
```

---

## Part 2: Migrating from .env to SelfVault

### Why Migrate to SelfVault?

| Feature | .env File | SelfVault |
|---------|-----------|-----------|
| **Encryption** | Optional (password-protected) | Always encrypted (AES-256-GCM) |
| **Access Control** | File permissions only | Role-based access control |
| **Audit Trail** | None | Complete audit logging |
| **Secret Rotation** | Manual | Automatic with TTL |
| **Dynamic Credentials** | No | Yes (auto-expiring) |
| **Memory Safety** | Plaintext in memory | Zeroized when dropped |
| **Compliance** | Basic | Enterprise-grade |

### Migration Process

SelfVault provides an **automated migration command** that:
1. Reads all secrets from your `.env` file
2. Encrypts them with AES-256-GCM
3. Stores them in SelfVault with appropriate TTLs
4. Maintains audit trail
5. Keeps original `.env` intact (you can delete it later)

### Step-by-Step Migration

#### Step 1: Initialize SelfVault (First Time Only)

```bash
# Initialize SelfVault with persistent master key
cargo run -- vault-init
```

**Output:**
```
🏦 SelfVault Initialization
═══════════════════════

✅ SelfVault initialized successfully
💾 Master key stored in: .vault_master.key
⚠️  IMPORTANT: Keep this file secure and backed up!
✅ Vault integrity verified
📊 Secrets stored: 0
```

**Important:**
- The master key is stored in `.vault_master.key`
- **Back up this file securely** - without it, you cannot access your secrets!
- Store it in a secure location (password manager, encrypted storage)

#### Step 2: Prepare Your .env File

Make sure you have a `.env` file with secrets to migrate:

```bash
# If using .env.demo
cp .env.demo .env

# Or generate a new one
cargo run -- generate

# Or use your existing .env file
# (make sure it's in the project directory)
```

#### Step 3: Run Migration

```bash
# Migrate all secrets from .env to SelfVault
cargo run -- vault-migrate
```

**What Gets Migrated:**

The migration automatically handles these secrets:

| Environment Variable | Vault Path | TTL | Category |
|---------------------|------------|-----|----------|
| `JWT_SECRET` | `secret/jwt` | 24 hours | Authentication |
| `SESSION_SECRET` | `secret/session` | 24 hours | Session Management |
| `API_KEY` | `secret/api-key` | 1 hour | API Access |
| `API_SECRET` | `secret/api-secret` | 1 hour | API Security |
| `ENCRYPTION_KEY` | `secret/encryption-key` | No expiry | Encryption |
| `DATABASE_PASSWORD` | `secret/db-password` | No expiry | Database |
| `WEB3_PRIVATE_KEY` | `secret/web3-private-key` | No expiry | Web3/Blockchain |

**Expected Output:**
```
🔄 Migrating secrets from .env to SelfVault...
  ✓ Migrated JWT_SECRET → secret/jwt
  ✓ Migrated SESSION_SECRET → secret/session
  ✓ Migrated API_KEY → secret/api-key
  ✓ Migrated API_SECRET → secret/api-secret
  ✓ Migrated ENCRYPTION_KEY → secret/encryption-key
  ⊘ Skipped DATABASE_PASSWORD (not set)
  ⊘ Skipped WEB3_PRIVATE_KEY (not set)
✅ Migration complete: 5 secrets migrated
💡 You can now remove sensitive values from .env file
```

#### Step 4: Verify Migration

```bash
# Check vault statistics
cargo run -- vault-stats
```

**Expected Output:**
```
🏦 SelfVault Statistics
═══════════════════════
Total secrets: 5
Sealed: No
Access control: Enabled
Audit entries: 7

Secrets by category:
  secret/*: 5

Recent activity:
  [2026-04-06 09:00:00] PUT secret/jwt by system
  [2026-04-06 09:00:00] PUT secret/session by system
  [2026-04-06 09:00:00] PUT secret/api-key by system
  ...
```

#### Step 5: Secure Your .env File

After successful migration, you have two options:

**Option A: Remove Sensitive Values (Recommended)**

Edit your `.env` file and replace actual secrets with placeholders:

```bash
nano .env
```

Change:
```env
JWT_SECRET=actual_secret_value_here
API_KEY=real_api_key_12345
```

To:
```env
JWT_SECRET=__VAULT__
API_KEY=__VAULT__
```

This indicates the values should be loaded from Vault.

**Option B: Delete .env File Entirely**

If all your secrets are in SelfVault:

```bash
# First, verify vault has all secrets
cargo run -- vault-stats

# Then safely remove .env
rm .env

# Or encrypt it for backup
cargo run -- lock
```

#### Step 6: Update Application Code

Modify your code to load secrets from SelfVault instead of environment variables:

**Before (using .env):**
```rust
use std::env;

let jwt_secret = env::var("JWT_SECRET")?;
let api_key = env::var("API_KEY")?;
```

**After (using SelfVault):**
```rust
use std::sync::Arc;
use env_manager::secrets::self_vault::SelfVault;

// Load master key
let master_key = std::fs::read(".vault_master.key")?;

// Initialize vault
let vault = Arc::new(SelfVault::new(&master_key));

// Retrieve secrets
if let Some(jwt_secret) = vault.get_secret("secret/jwt", "admin").await? {
    println!("JWT Secret loaded from vault");
}

if let Some(api_key) = vault.get_secret("secret/api-key", "admin").await? {
    println!("API Key loaded from vault");
}
```

### Advanced Migration Options

#### Custom Secret Mapping

You can customize which secrets get migrated and their TTLs by editing the migration function in `src/utils/vault_manager.rs`:

```rust
let secrets_to_migrate = vec![
    ("YOUR_CUSTOM_SECRET", "secret/custom-path", Some(3600u64)), // 1 hour TTL
    ("ANOTHER_SECRET", "secret/another", None),                   // No expiry
];
```

#### Selective Migration

Migrate only specific secrets:

```bash
# Edit vault_manager.rs to include only desired secrets
# Then run migration
cargo run -- vault-migrate
```

#### Export from Vault (Backup)

You can export vault secrets back to .env format for backup:

```rust
// In your code
use env_manager::utils::vault_manager::export_vault_to_env;

export_vault_to_env(&vault, "backup.env").await?;
```

---

## Part 3: Complete Workflow Example

Here's a complete example from start to finish:

```bash
# 1. Start with .env.demo
cd /Users/macbookpri/Documents/env-manager
cp .env.demo .env

# 2. Initialize SelfVault
cargo run -- vault-init
# Backup .vault_master.key securely!

# 3. Migrate secrets
cargo run -- vault-migrate

# 4. Verify migration
cargo run -- vault-stats

# 5. Secure the .env file
cargo run -- lock
# Enter password to encrypt

# 6. Run application (loads from vault)
cargo run

# 7. When done, check audit trail
cargo run -- vault-stats
```

---

## Part 4: Troubleshooting

### Problem: "Master key file not found"

**Solution:**
```bash
# Check if file exists
ls -la .vault_master.key

# If missing, reinitialize (will create new vault)
cargo run -- vault-init
```

### Problem: "Migration failed - secret not found"

**Solution:**
- Ensure the environment variable exists in `.env`
- Check for typos in variable names
- Verify `.env` file is in current directory

### Problem: "Cannot access vault - sealed"

**Solution:**
SelfVault doesn't use seal/unseal like HashiCorp Vault. If you see errors:
```bash
# Check vault status
cargo run -- vault-stats

# Reinitialize if needed (WARNING: loses all secrets!)
rm .vault_master.key
cargo run -- vault-init
```

### Problem: "TTL expired - secret not available"

**Solution:**
Secrets with TTL will expire. To refresh:
```bash
# Re-migrate the secret
cargo run -- vault-migrate

# Or manually put a new value
# (requires custom code)
```

### Problem: Lost master key

**Critical:** Without the master key, all secrets are unrecoverable!

**Prevention:**
- Back up `.vault_master.key` immediately after initialization
- Store in multiple secure locations
- Use a password manager

**Recovery:**
- Not possible - you must reinitialize and re-migrate all secrets

---

## Part 5: Best Practices

### 1. Backup Strategy

```bash
# After initialization
cp .vault_master.key /secure/backup/location/

# After migration
cargo run -- vault-stats > vault-backup-$(date +%Y%m%d).txt
```

### 2. Regular Audits

```bash
# Weekly: Check vault statistics
cargo run -- vault-stats

# Monthly: Review audit trail
# (implement custom audit log viewer)
```

### 3. Secret Rotation

For secrets with TTL, set up automatic rotation:

```bash
# Add to crontab (rotate daily at midnight)
0 0 * * * cd /path/to/env-manager && cargo run -- vault-migrate
```

### 4. Multi-Environment Setup

```bash
# Development
cp .env.dev .env
cargo run -- vault-migrate

# Staging
cp .env.staging .env
cargo run -- vault-migrate

# Production
cp .env.prod .env
cargo run -- vault-migrate
```

Use separate vault instances or namespaces for each environment.

### 5. Team Collaboration

```bash
# Share master key SECURELY (never via email!)
# - Encrypted messaging (Signal)
# - Password manager sharing
# - In-person handoff

# Team member receives:
# 1. .vault_master.key (securely)
# 2. Project code
# 3. Access to run migrations if needed
```

---

## Part 6: Next Steps

After migration, consider:

1. **Enable Auto-Rotation**: Configure automatic secret rotation
2. **Set Up Monitoring**: Track vault access and anomalies
3. **Implement RBAC**: Define user roles and permissions
4. **Add Dynamic Credentials**: Use short-lived database credentials
5. **Configure Alerts**: Get notified of suspicious access patterns

---

## Quick Reference Commands

```bash
# Initialize vault
cargo run -- vault-init

# Migrate secrets
cargo run -- vault-migrate

# Check statistics
cargo run -- vault-stats

# Lock .env file
cargo run -- lock

# Unlock .env file
cargo run -- unlock

# Change password
cargo run -- chpasswd

# Check status
cargo run -- status

# Generate new .env
cargo run -- generate
```

---

## Summary

✅ **Using .env.demo**: Copy to `.env` and run  
✅ **Migrating to SelfVault**: `vault-init` → `vault-migrate` → `vault-stats`  
✅ **Securing**: Lock `.env` or delete after migration  
✅ **Best Practices**: Backup master key, regular audits, proper rotation  

Your secrets are now enterprise-grade secure! 🔐
