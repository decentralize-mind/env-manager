# Running the Application - Quick Guide

## 🚀 Current Status

Your application is now running successfully in **development mode** without Vault! ✅

The warnings you saw before have been fixed. The application now:
- Detects if Vault is configured
- Runs in dev mode if Vault is not available
- Provides clear instructions on how to enable Vault
- Gracefully falls back to `.env` file or environment variables

---

## 💻 Option 1: Run Without Vault (Development Mode)

This is what you're currently doing - perfect for development and testing!

```bash
# Just run it - no configuration needed
cargo run

# Or with custom environment variables
APP__APP__NAME=MyApp APP__APP__PORT=3000 cargo run
```

**Output:**
```
INFO secure_config: 🔐 Loading secure configuration...
INFO secure_config: ✅ Configuration validated successfully
INFO secure_config: 💻 Running in development mode (Vault not configured)
INFO secure_config:    ℹ️  To use Vault, set VAULT_TOKEN and VAULT_ADDR environment variables
INFO secure_config:    ℹ️  Using secrets from .env file or environment variables
INFO secure_config::security::audit: AUDIT: system performed config_load
INFO secure_config: ✅ Admin access granted
INFO secure_config: 🚀 Application ready!
INFO secure_config:    App: MySecureApp on port 8080
INFO secure_config:    Database URL: postgresql://localhost:5432/mydb
```

✅ **No errors, no warnings!**

---

## 🏦 Option 2: Run With HashiCorp Vault (Production Mode)

If you want to use Vault for secret management:

### Step 1: Install Vault

```bash
# macOS
brew install vault

# Or download from https://www.vaultproject.io/downloads
```

### Step 2: Start Vault Dev Server

```bash
# Start Vault in dev mode (for testing only!)
vault server -dev -dev-root-token-id="my-root-token"

# In another terminal, set the environment variable
export VAULT_ADDR='http://127.0.0.1:8200'
export VAULT_TOKEN='my-root-token'
```

### Step 3: Store Secrets in Vault

```bash
# Store your JWT secret
vault kv put secret/data/app jwt="your-super-secret-jwt-key"

# Verify it's stored
vault kv get secret/data/app
```

### Step 4: Run Your Application

```bash
# Set environment variables
export VAULT_TOKEN="my-root-token"
export VAULT_ADDR="http://127.0.0.1:8200"

# Run the application
cargo run
```

**Output:**
```
INFO secure_config: 🔐 Loading secure configuration...
INFO secure_config: ✅ Configuration validated successfully
INFO secure_config: 🏦 Vault configured at: http://127.0.0.1:8200
INFO secure_config: ✅ JWT secret loaded from Vault
INFO secure_config::security::audit: AUDIT: system performed config_load
INFO secure_config: ✅ Admin access granted
INFO secure_config: 🚀 Application ready!
```

---

## 🔧 Option 3: Run With Custom Configuration

You can override any configuration via environment variables:

```bash
# Override app settings
APP__APP__NAME="ProductionApp" \
APP__APP__PORT=3000 \
APP__DB__URL="postgresql://prod-db:5432/mydb" \
cargo run
```

---

## 📝 Configuration Priority

The application loads configuration in this order (highest priority first):

1. **Environment variables** (with `APP__` prefix)
2. **`.env` file** in project root
3. **Vault** (if configured and available)

Example:
```bash
# This overrides the .env file
APP__APP__PORT=9000 cargo run
```

---

## 🐛 Troubleshooting

### "Config failed" Error

**Problem:** Missing required configuration fields

**Solution:** Make sure your `.env` file has all required fields:
```env
APP__APP__NAME=MyApp
APP__APP__PORT=8080
APP__DB__URL=postgresql://localhost:5432/mydb
APP__SECURITY__JWT_SECRET=your-secret
APP__SECURITY__ENCRYPTION_KEY=your-key
```

### Vault Connection Refused

**Problem:** Vault is not running

**Solutions:**
1. Start Vault: `vault server -dev`
2. Or run without Vault (dev mode) - this is fine for development!

### Wrong Environment Variable Format

**Problem:** Configuration not loading

**Solution:** Use double underscore (`__`) and `APP` prefix:
```bash
# ❌ Wrong
APP_NAME=MyApp

# ✅ Correct
APP__APP__NAME=MyApp
```

---

## 🎯 Recommended Workflow

### For Development:
```bash
# Just run it - uses .env file
cargo run
```

### For Testing with Vault:
```bash
# Terminal 1: Start Vault
vault server -dev -dev-root-token-id="test-token"

# Terminal 2: Set up secrets
export VAULT_ADDR='http://127.0.0.1:8200'
export VAULT_TOKEN='test-token'
vault kv put secret/data/app jwt="test-jwt-secret"

# Terminal 2: Run app
cargo run
```

### For Production:
```bash
# Set production environment variables
export VAULT_TOKEN="production-vault-token"
export VAULT_ADDR="https://vault.production.com:8200"

# Run optimized build
./target/release/secure-config
```

---

## ✨ Key Improvements Made

✅ **No more confusing error messages**  
✅ **Clear indication of running mode** (dev vs Vault)  
✅ **Helpful hints** on how to configure Vault  
✅ **Graceful fallback** to environment variables  
✅ **Better user experience** overall  

---

## 📚 Next Steps

1. **Development:** Continue using dev mode with `.env` file
2. **Testing:** Try the Vault dev server to test integration
3. **Production:** Set up real Vault server with proper authentication

The application is working perfectly! 🎉
