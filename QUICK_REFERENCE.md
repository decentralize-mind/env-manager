# Quick Reference Guide

## 🚀 Running the Application

```bash
# Basic run (uses .env file)
cargo run

# With custom environment variables
APP__APP__NAME=MyApp APP__APP__PORT=8080 cargo run

# Production build
cargo build --release
./target/release/secure-config
```

## 📝 Configuration Format

Environment variables use this pattern: `APP__{SECTION}__{FIELD}`

Examples:
```bash
APP__APP__NAME=MyApp              # app.name
APP__APP__PORT=8080               # app.port
APP__DB__URL=postgres://...       # db.url
APP__SECURITY__JWT_SECRET=xxx     # security.jwt_secret
```

## 🔐 Using with Vault

```bash
export VAULT_TOKEN="hvs.xxxxx"
export VAULT_ADDR="https://vault.example.com:8200"
cargo run
```

## 🏗️ Module Overview

| Module | Purpose | Key Files |
|--------|---------|-----------|
| **config** | Load & validate settings | schema.rs, loader.rs, validator.rs |
| **secrets** | Secure secret management | memory.rs, vault.rs, rotator.rs |
| **security** | Access control & audit | access.rs, audit.rs |

## 🔧 Common Tasks

### Add New Config Field
1. Edit `src/config/schema.rs` - add field to struct
2. Set env var: `APP__SECTION__FIELD=value`
3. Rebuild: `cargo build`

### Enable Key Rotation
Uncomment in `src/main.rs`:
```rust
tokio::spawn(secrets::rotator::rotation_loop());
```

### Customize Rotation Interval
Edit `src/secrets/rotator.rs`:
```rust
sleep(Duration::from_secs(3600)).await; // Change 3600 to desired seconds
```

### Add Audit Logging
```rust
use security::audit::log_access;
log_access("user", "action");
```

## 🛡️ Security Checklist

- [ ] Never commit `.env` files (already in .gitignore)
- [ ] Use short-lived Vault tokens
- [ ] Enable HTTPS for Vault communication
- [ ] Monitor audit logs
- [ ] Rotate secrets regularly
- [ ] Use role-based access control

## 📊 Logs Output

The application uses structured logging:
```
INFO secure_config: 🔐 Loading secure configuration...
INFO secure_config: ✅ Configuration validated successfully
INFO secure_config::security::audit: AUDIT: system performed config_load
INFO secure_config: 🚀 Application ready!
```

## ⚡ Performance Tips

- Use `--release` flag for production builds
- Consider enabling LTO in Cargo.toml for smaller binaries
- Use async operations for I/O-bound tasks
- Cache Vault responses when appropriate

## 🐛 Troubleshooting

**Config loading fails:**
- Check environment variable format (double underscore)
- Ensure required fields are set
- Verify .env file syntax

**Vault connection refused:**
- System gracefully falls back to env vars
- Check VAULT_ADDR and VAULT_TOKEN
- Verify Vault server is running

**Compilation warnings:**
- Unused code warnings are normal (framework functions)
- No action needed unless you see errors

## 📚 Learn More

- [README.md](README.md) - Full documentation
- [PROJECT_SUMMARY.md](PROJECT_SUMMARY.md) - Architecture overview
- [env-manager.md](env-manager.md) - Original specification

---

**Quick Help:** Run `cargo doc --open` to view generated documentation
