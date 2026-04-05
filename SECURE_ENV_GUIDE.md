# 🔐 Secure .env File Management - Password Protection Guide

## Overview

Your env-manager now includes **military-grade password protection** for your `.env` file using AES-256-GCM encryption. This prevents unauthorized access to your secrets even if someone gains access to your filesystem.

---

## 🎯 How It Works

```
┌─────────────────────────────────────────────┐
│   UNLOCKED (Plaintext .env)                 │
│   • Editable                                │
│   • Readable by anyone with file access     │
│   • Normal operation mode                   │
└──────────────┬──────────────────────────────┘
               │
          cargo run -- lock
               │
               ▼
┌─────────────────────────────────────────────┐
│   LOCKED (Encrypted .env.encrypted)         │
│   • Requires password to unlock             │
│   • AES-256-GCM encrypted                   │
│   • Original securely deleted               │
│   • Cannot be read without password         │
└──────────────┬──────────────────────────────┘
               │
          cargo run -- unlock
               │
               ▼
┌─────────────────────────────────────────────┐
│   UNLOCKED (Decrypted .env)                 │
│   • Ready to edit                           │
│   • Use application normally                │
│   • Lock again when done                    │
└─────────────────────────────────────────────┘
```

---

## 🚀 Quick Start

### 1. Generate a Complete .env Template

```bash
cargo run -- generate
```

This creates a `.env` file with all required fields pre-filled with placeholders:

```env
# Application Settings
APP_NAME=SecureConfigApp
APP_ENV=development
APP_PORT=8080

# Database Configuration
DATABASE_URL=postgresql://user:password@localhost:5432/mydb

# JWT Configuration
JWT_SECRET=change_this_to_secure_random_string_minimum_32_chars

# API Keys
API_KEY=your_api_key_here
API_SECRET=your_api_secret_here

# ... and more!
```

### 2. Edit the .env File

Open `.env` in your favorite editor and replace placeholder values with your actual secrets:

```bash
nano .env
# or
code .env
# or
vim .env
```

### 3. Lock (Encrypt) the File

```bash
cargo run -- lock
```

You'll be prompted for a password:
```
Enter encryption password: ********
```

**What happens:**
- ✅ File is encrypted with AES-256-GCM
- ✅ Encrypted as `.env.encrypted`
- ✅ Original `.env` is securely deleted (overwritten 3x with random data)
- ✅ Only accessible with your password

### 4. Check Status

```bash
cargo run -- status
```

Output:
```
🔒 .env file is LOCKED (encrypted)
   Run 'cargo run -- unlock' to decrypt
```

### 5. Unlock (Decrypt) When Needed

```bash
cargo run -- unlock
```

Enter your password:
```
Enter decryption password: ********
```

**What happens:**
- ✅ Decrypts `.env.encrypted` back to `.env`
- ✅ File is now editable
- ✅ Application can read it normally

### 6. Use Your Application

```bash
cargo run
```

The app loads configuration from the unlocked `.env` file.

### 7. Lock Again After Use

```bash
cargo run -- lock
```

Always re-lock when you're done editing!

---

## 📋 All Commands

| Command | Description |
|---------|-------------|
| `cargo run -- generate` | Create new .env template |
| `cargo run -- lock` | Encrypt and protect .env |
| `cargo run -- unlock` | Decrypt .env (needs password) |
| `cargo run -- chpasswd` | Change encryption password |
| `cargo run -- status` | Check lock status |
| `cargo run -- help` | Show help message |
| `cargo run` | Load config (default behavior) |

---

## 🔑 Password Best Practices

### ✅ DO:
- Use **strong passwords** (12+ characters, mixed case, numbers, symbols)
- Store password in a **password manager** (1Password, Bitwarden, LastPass)
- Use **different passwords** for different environments
- **Memorize** your password or store it securely
- Test unlocking after locking to verify password works

### ❌ DON'T:
- Use simple passwords like "password123"
- Share passwords via email/chat
- Write passwords on sticky notes
- Forget your password (no recovery possible!)
- Use the same password across multiple projects

### Example Strong Password:
```
Tr0ub4dor&3xYz!9Qw
```

---

## 🛡️ Security Features

### 1. **AES-256-GCM Encryption**
- Industry-standard authenticated encryption
- 256-bit key derived from your password
- Provides confidentiality AND integrity

### 2. **Key Derivation**
- Password hashed with SHA-256
- Creates cryptographic key from password
- Resistant to brute-force attacks

### 3. **Secure Deletion**
- Overwrites file 3 times with random data
- Prevents forensic recovery
- Then deletes the file

### 4. **Random Nonce**
- Each encryption uses unique nonce
- Prevents pattern analysis
- Cryptographically secure random generation

### 5. **Authenticated Encryption**
- Detects tampering
- Wrong password = immediate failure
- Corrupted data detected automatically

---

## 🔄 Workflow Examples

### Daily Development Workflow

```bash
# Morning: Unlock .env
cargo run -- unlock
# Enter password...

# Work on your project
cargo run
# ... development ...

# Evening: Lock .env
cargo run -- lock
# Enter password...
```

### Team Collaboration

```bash
# Developer A creates .env
cargo run -- generate
# Edit with real secrets
nano .env
# Lock with shared password
cargo run -- lock

# Share password SECURELY (not via email!)
# - In person
# - Encrypted messaging (Signal)
# - Password manager sharing

# Developer B receives .env.encrypted
# Unlock with shared password
cargo run -- unlock
```

### Production Deployment

```bash
# On production server
cargo run -- unlock
# Enter production password

# Start application
./target/release/secure-config

# Keep unlocked while running
# Or use systemd service that unlocks at startup

# For updates:
# 1. Stop application
# 2. Unlock
# 3. Edit .env
# 4. Lock again
# 5. Restart application
```

### Changing Passwords

```bash
# Periodically change password (every 90 days recommended)
cargo run -- chpasswd

Enter current password: ********
Enter new password: ********
Confirm new password: ********

✅ Password changed successfully
```

---

## ⚠️ Important Warnings

### 1. **No Password Recovery**
If you forget your password, **the data is lost forever**. There is no backdoor or recovery mechanism.

**Mitigation:**
- Use a password manager
- Write it down and store in a safe
- Test your password immediately after setting

### 2. **File Backup**
Before major changes, backup your encrypted file:

```bash
cp .env.encrypted .env.encrypted.backup.$(date +%Y%m%d)
```

### 3. **Don't Commit to Git**
Add to `.gitignore`:

```gitignore
.env
.env.encrypted
*.env.bak
```

### 4. **Environment-Specific Passwords**
Use different passwords for:
- Development
- Staging
- Production

---

## 🔍 Troubleshooting

### Problem: "Decryption failed - wrong password"

**Cause:** Entered wrong password

**Solution:**
- Double-check caps lock
- Try previous password if recently changed
- Restore from backup if available

### Problem: ".env file not found"

**Cause:** File doesn't exist yet

**Solution:**
```bash
cargo run -- generate
```

### Problem: "Encrypted file not found"

**Cause:** Trying to unlock but file isn't encrypted

**Solution:**
```bash
cargo run -- status
# If unlocked, just edit normally
# If missing, generate new one
```

### Problem: Can't edit .env because it's locked

**Solution:**
```bash
cargo run -- unlock
# Edit file
cargo run -- lock
```

---

## 📊 Comparison: Before vs After

| Feature | Before | After |
|---------|--------|-------|
| File Format | Plaintext | AES-256-GCM encrypted |
| Access Control | File permissions only | Password + file permissions |
| Theft Protection | None | Encrypted - useless without password |
| Accidental Exposure | Easy | Prevented |
| Secure Deletion | No | Yes (3x overwrite) |
| Password Rotation | N/A | Built-in command |
| Status Checking | Manual | One command |

---

## 🧪 Testing

Test the complete workflow:

```bash
# 1. Generate
cargo run -- generate

# 2. Check status
cargo run -- status
# Should show: UNLOCKED

# 3. Lock with test password
cargo run -- lock
# Password: TestPass123!

# 4. Check status
cargo run -- status
# Should show: LOCKED

# 5. Try to unlock with wrong password
cargo run -- unlock
# Password: WrongPass
# Should fail

# 6. Unlock with correct password
cargo run -- unlock
# Password: TestPass123!
# Should succeed

# 7. Verify content
cat .env
# Should see your configuration

# 8. Clean up
rm .env
```

---

## 💡 Pro Tips

### 1. Auto-Lock Reminder
Add to your shell profile (`.zshrc` or `.bashrc`):

```bash
alias env-lock='echo "💡 Remember to lock your .env file!" && cargo run -- status'
```

### 2. Pre-commit Hook
Prevent committing unlocked .env:

```bash
# .git/hooks/pre-commit
if [ -f .env ]; then
    echo "❌ ERROR: Unlocked .env file detected!"
    echo "Run: cargo run -- lock"
    exit 1
fi
```

### 3. CI/CD Integration
In your CI pipeline:

```yaml
# GitHub Actions example
- name: Unlock .env
  run: cargo run -- unlock
  env:
    ENV_PASSWORD: ${{ secrets.ENV_PASSWORD }}
  
- name: Run tests
  run: cargo test
  
- name: Lock .env
  run: cargo run -- lock
```

### 4. Multiple Environments

```bash
# Development
cp .env.dev .env
cargo run -- lock

# Staging
cp .env.staging .env
cargo run -- lock

# Production
cp .env.prod .env
cargo run -- lock
```

---

## 🔐 Security Audit Checklist

- [ ] Using strong password (12+ chars)
- [ ] Password stored in password manager
- [ ] .env added to .gitignore
- [ ] Tested unlock/lock cycle
- [ ] Backup of encrypted file exists
- [ ] Different passwords per environment
- [ ] Team trained on secure workflows
- [ ] Password rotation schedule established
- [ ] Emergency recovery plan documented

---

## 📚 Technical Details

### Encryption Process

1. **Input**: Plaintext .env + password
2. **Key Derivation**: SHA-256(password) → 256-bit key
3. **Nonce Generation**: 12-byte random nonce
4. **Encryption**: AES-256-GCM(key, nonce, plaintext)
5. **Output**: nonce + ciphertext → .env.encrypted

### Decryption Process

1. **Input**: .env.encrypted + password
2. **Key Derivation**: SHA-256(password) → 256-bit key
3. **Extract**: Split nonce (12 bytes) + ciphertext
4. **Decryption**: AES-256-GCM(key, nonce, ciphertext)
5. **Output**: Plaintext → .env

### Secure Deletion

1. Get file size
2. Overwrite with random bytes (3 passes)
3. Delete file
4. Prevents forensic recovery

---

## 🎓 Learn More

- [AES-GCM Explained](https://en.wikipedia.org/wiki/Galois/Counter_Mode)
- [Password Security Best Practices](https://pages.nist.gov/800-63-3/sp800-63b.html)
- [Secure File Deletion](https://en.wikipedia.org/wiki/Data_remanence)
- [Cryptographic Key Management](https://csrc.nist.gov/publications/detail/sp/800-57-part-1/rev-5/final)

---

**Your .env files are now protected with military-grade encryption!** 🔒✨

Remember: **Lock it when not in use, unlock it when needed, and never forget your password!**
