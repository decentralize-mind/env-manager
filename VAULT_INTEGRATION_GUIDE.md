# 🏗️ Vault Integration & Protocol-Level .env Guide

## Overview

Your env-manager now supports **dual-mode operation**:

1. **Development Mode** - Auto-generated secrets in encrypted .env
2. **Production Mode** - Vault-based secret orchestration (protocol-level)

This guide explains how to use both modes and transition between them.

---

## 🎯 What Changed?

### Before
```env
JWT_SECRET=auto_generated_value
ENCRYPTION_KEY=auto_generated_hex
# No Vault configuration
# No secret path pointers
# No safety controls
```

### After (Hybrid Approach)
```env
# Development: Auto-generated secrets
JWT_SECRET=auto_generated_value
ENCRYPTION_KEY=auto_generated_hex

# Production: Vault configuration (commented by default)
# SECRETS_PROVIDER=vault
# VAULT_ADDR=http://vault.service:8200
# SECRET_JWT_PATH=secret/data/secure-app/jwt

# Safety controls (NEW!)
MAX_WITHDRAWAL_LIMIT=100000
ENABLE_GLOBAL_PAUSE=false
POLICY_ENGINE_ENABLED=true
```

---

## 📋 Complete .env Structure

The new template includes **11 organized sections**:

### 1. 🧩 Application Settings
```env
APP_NAME=SecureConfigApp
APP_ENV=development
APP_PORT=8080
APP_LOG_LEVEL=info
```

### 2. 🗄️ Database Configuration
```env
DATABASE_URL=postgresql://user:password@localhost:5432/mydb
DATABASE_POOL_SIZE=10
DATABASE_TIMEOUT=30
```

### 3. 🔐 Auto-Generated Secrets (Development)
```env
# Cryptographically secure, auto-generated
JWT_SECRET=<64-char-random>
SESSION_SECRET=<64-char-random>
API_KEY=<32-char-random>
API_SECRET=<48-char-random>
ENCRYPTION_KEY=<64-hex-chars>
```

### 4. ☁️ Vault Configuration (Production)
```env
# Uncomment for production deployment
# SECRETS_PROVIDER=vault
# VAULT_ADDR=http://vault.service:8200
# VAULT_AUTH_METHOD=kubernetes
# VAULT_ROLE=secure-app-role
# VAULT_TOKEN=your_vault_token  # Only for local testing
```

### 5. 🔑 Secret Paths (Vault Pointers)
```env
# These point to WHERE secrets are stored (not the values!)
# SECRET_JWT_PATH=secret/data/secure-app/jwt
# SECRET_SESSION_PATH=secret/data/secure-app/session
# SECRET_API_PATH=secret/data/secure-app/api
# SECRET_DB_PATH=database/creds/secure-app
# SECRET_ENCRYPTION_PATH=secret/data/secure-app/encryption
```

### 6. 🌐 Web3 Configuration
```env
# WEB3_PRIVATE_KEY=__VAULT_ONLY__  # Never store in .env!
# WEB3_RPC_URL=https://mainnet.infura.io/v3/YOUR_PROJECT_ID
# WEB3_SIGNER_SERVICE=http://signer-service:8080
```

### 7. 🛡️ Security Controls
```env
ALLOWED_ORIGINS=http://localhost:3000,http://localhost:8080
CORS_ENABLED=true
RATE_LIMIT_REQUESTS=100
RATE_LIMIT_WINDOW=3600
```

### 8. 🚨 Safety Controls & Circuit Breakers (NEW!)
```env
MAX_WITHDRAWAL_LIMIT=100000
BRIDGE_DAILY_LIMIT=1000000
ENABLE_GLOBAL_PAUSE=false
ANOMALY_THRESHOLD=0.8
POLICY_ENGINE_ENABLED=true
POLICY_MODE=strict
```

### 9. 🔄 Secret Rotation Config (NEW!)
```env
SECRET_REFRESH_INTERVAL=300   # seconds
SECRET_MAX_TTL=900            # seconds
ENABLE_AUTO_ROTATION=true
```

### 10. 📊 Observability (NEW!)
```env
METRICS_ENABLED=true
METRICS_PORT=9090
TRACING_ENABLED=true
```

### 11. 🧠 Feature Flags
```env
ENABLE_BRIDGE=true
ENABLE_AIRDROP=false
ENABLE_SIMULATION_ENGINE=true
ENABLE_DEBUG_MODE=false
```

---

## 🔄 Two Operating Modes

### Mode 1: Development (Default)

**Use when:**
- Local development
- Testing
- Staging without Vault

**Configuration:**
```env
# Use auto-generated secrets
JWT_SECRET=OqMNABkxYarMTaf&O2a^hzXiivREVxAwliQ9z9L2FTt4132c2raePUHH4uiu3r%q
ENCRYPTION_KEY=59cec450d760cf0dfb86a604f2dbab405f2afb8c3f91922954c6981be64375f6

# Vault section stays commented
# SECRETS_PROVIDER=vault
# VAULT_ADDR=...
```

**Workflow:**
```bash
# Generate with auto-secrets
cargo run -- generate

# Lock with password
cargo run -- lock

# Run application
cargo run
```

---

### Mode 2: Production (Vault-Based)

**Use when:**
- Production deployment
- Exchange/protocol operations
- Compliance requirements

**Configuration:**
```env
# Comment out auto-generated secrets
# JWT_SECRET=...
# ENCRYPTION_KEY=...

# Enable Vault
SECRETS_PROVIDER=vault
VAULT_ADDR=http://vault.service:8200
VAULT_AUTH_METHOD=kubernetes
VAULT_ROLE=secure-app-role

# Define secret paths
SECRET_JWT_PATH=secret/data/secure-app/jwt
SECRET_DB_PATH=database/creds/secure-app
SECRET_API_PATH=secret/data/secure-app/api
```

**Workflow:**
```bash
# 1. Store secrets in Vault
vault kv put secret/data/secure-app/jwt value="your-jwt-secret"
vault kv put secret/data/secure-app/api key="api-key" secret="api-secret"

# 2. Configure .env with Vault paths
# (Uncomment Vault section, comment auto-generated secrets)

# 3. Deploy to Kubernetes
kubectl apply -f k8s/

# App will fetch secrets from Vault at runtime
```

---

## 🏗️ Vault Setup Guide

### Step 1: Install Vault on Kubernetes

```bash
# Add Helm repo
helm repo add hashicorp https://helm.releases.hashicorp.com

# Install Vault
helm install vault hashicorp/vault \
  --namespace vault \
  --create-namespace \
  --set "server.dev.enabled=false"  # Production mode
```

### Step 2: Initialize & Unseal Vault

```bash
# Get Vault pod
export VAULT_POD=$(kubectl get pods -n vault -l app.kubernetes.io/name=vault -o jsonpath='{.items[0].metadata.name}')

# Initialize
kubectl exec -it $VAULT_POD -n vault -- vault operator init

# Save unseal keys and root token securely!

# Unseal (run 3 times with different keys)
kubectl exec -it $VAULT_POD -n vault -- vault operator unseal
```

### Step 3: Enable Secrets Engines

```bash
# Enable KV v2 for general secrets
kubectl exec -it $VAULT_POD -n vault -- vault secrets enable -path=secret kv-v2

# Enable database engine for dynamic credentials
kubectl exec -it $VAULT_POD -n vault -- vault secrets enable database
```

### Step 4: Store Secrets

```bash
# JWT Secret
kubectl exec -it $VAULT_POD -n vault -- vault kv put secret/data/secure-app/jwt \
  value="your-secure-jwt-secret-minimum-64-chars"

# API Credentials
kubectl exec -it $VAULT_POD -n vault -- vault kv put secret/data/secure-app/api \
  key="your-api-key" \
  secret="your-api-secret"

# Encryption Key
kubectl exec -it $VAULT_POD -n vault -- vault kv put secret/data/secure-app/encryption \
  key="32-byte-hex-encryption-key-here"
```

### Step 5: Configure Dynamic Database Credentials

```bash
# Configure PostgreSQL connection
kubectl exec -it $VAULT_POD -n vault -- vault write database/config/secure-app \
  plugin_name=postgresql-database-plugin \
  allowed_roles="secure-app-role" \
  connection_url="postgresql://{{username}}:{{password}}@postgres:5432/mydb" \
  username="vault-admin" \
  password="admin-password"

# Create role with TTL
kubectl exec -it $VAULT_POD -n vault -- vault write database/roles/secure-app-role \
  db_name=secure-app \
  creation_statements="CREATE ROLE \"{{name}}\" WITH LOGIN PASSWORD '{{password}}' VALID UNTIL '{{expiration}}'; GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO \"{{name}}\";" \
  default_ttl="15m" \
  max_ttl="24h"
```

### Step 6: Configure Kubernetes Auth

```bash
# Enable Kubernetes auth method
kubectl exec -it $VAULT_POD -n vault -- vault auth enable kubernetes

# Configure Kubernetes auth
kubectl exec -it $VAULT_POD -n vault -- vault write auth/kubernetes/config \
  token_reviewer_jwt="$(cat /var/run/secrets/kubernetes.io/serviceaccount/token)" \
  kubernetes_host="https://$KUBERNETES_PORT_443_TCP_ADDR:443" \
  kubernetes_ca_cert=@/var/run/secrets/kubernetes.io/serviceaccount/ca.crt

# Create policy
kubectl exec -it $VAULT_POD -n vault -- vault policy write secure-app-policy - <<EOF
path "secret/data/secure-app/*" {
  capabilities = ["read"]
}

path "database/creds/secure-app-role" {
  capabilities = ["read"]
}
EOF

# Create role binding
kubectl exec -it $VAULT_POD -n vault -- vault write auth/kubernetes/role/secure-app-role \
  bound_service_account_names=vault-auth \
  bound_service_account_namespaces=secure-app \
  policies=secure-app-policy \
  ttl=24h
```

### Step 7: Update .env for Production

```env
# Comment out auto-generated secrets
# JWT_SECRET=...
# ENCRYPTION_KEY=...

# Enable Vault
SECRETS_PROVIDER=vault
VAULT_ADDR=http://vault.vault.svc.cluster.local:8200
VAULT_AUTH_METHOD=kubernetes
VAULT_ROLE=secure-app-role

# Secret paths
SECRET_JWT_PATH=secret/data/secure-app/jwt
SECRET_DB_PATH=database/creds/secure-app-role
SECRET_API_PATH=secret/data/secure-app/api
SECRET_ENCRYPTION_PATH=secret/data/secure-app/encryption
```

---

## 🔄 Migration Guide: Dev → Production

### Phase 1: Prepare Vault (Week 1)

```bash
# 1. Set up Vault cluster
# 2. Store all secrets in Vault
# 3. Configure dynamic credentials
# 4. Test Vault access locally
```

### Phase 2: Update Configuration (Week 2)

```bash
# 1. Generate new .env template
cargo run -- generate

# 2. Edit .env:
#    - Comment auto-generated secrets
#    - Uncomment Vault section
#    - Add secret paths

# 3. Test with Vault locally
VAULT_ADDR=http://localhost:8200 cargo run
```

### Phase 3: Deploy to Staging (Week 3)

```bash
# 1. Deploy Vault to staging K8s
# 2. Update staging .env with Vault config
# 3. Deploy application
# 4. Verify secrets are fetched from Vault
kubectl logs -f deployment/secure-config-app -n secure-app-staging
```

### Phase 4: Production Rollout (Week 4)

```bash
# 1. Deploy Vault to production K8s
# 2. Migrate secrets to production Vault
# 3. Update production .env
# 4. Deploy application
# 5. Monitor closely
```

---

## 🛡️ Safety Controls Explained

### MAX_WITHDRAWAL_LIMIT
```env
MAX_WITHDRAWAL_LIMIT=100000
```
- Maximum single withdrawal amount
- Prevents large unauthorized transfers
- Enforced by policy engine

### BRIDGE_DAILY_LIMIT
```env
BRIDGE_DAILY_LIMIT=1000000
```
- Daily limit for cross-chain bridge operations
- Resets every 24 hours
- Protects against bridge exploits

### ENABLE_GLOBAL_PAUSE
```env
ENABLE_GLOBAL_PAUSE=false
```
- Emergency kill switch
- Set to `true` to halt all operations
- Use during security incidents

### ANOMALY_THRESHOLD
```env
ANOMALY_THRESHOLD=0.8
```
- Sensitivity for anomaly detection (0.0 - 1.0)
- Lower = more sensitive
- Triggers alerts when exceeded

### POLICY_ENGINE_ENABLED
```env
POLICY_ENGINE_ENABLED=true
POLICY_MODE=strict
```
- Enables policy enforcement
- `strict` mode blocks violations
- `audit` mode only logs violations

---

## 🔄 Secret Rotation Configuration

### SECRET_REFRESH_INTERVAL
```env
SECRET_REFRESH_INTERVAL=300   # 5 minutes
```
- How often to check for secret updates
- Balances freshness vs performance
- Recommended: 300-600 seconds

### SECRET_MAX_TTL
```env
SECRET_MAX_TTL=900   # 15 minutes
```
- Maximum time before forced rotation
- Security requirement for compliance
- Shorter = more secure, more overhead

### ENABLE_AUTO_ROTATION
```env
ENABLE_AUTO_ROTATION=true
```
- Automatically rotate expiring secrets
- Zero-downtime rotation
- Maintains active + previous versions

---

## 📊 Observability Configuration

### METRICS_ENABLED
```env
METRICS_ENABLED=true
METRICS_PORT=9090
```
- Exposes Prometheus metrics
- Track secret fetches, rotations, errors
- Scrape endpoint: `http://app:9090/metrics`

### TRACING_ENABLED
```env
TRACING_ENABLED=true
```
- Distributed tracing support
- Track secret access across services
- Integrate with Jaeger/Zipkin

---

## 🧪 Testing

### Test Development Mode

```bash
# Generate with auto-secrets
cargo run -- generate

# Verify auto-generated values
cat .env | grep JWT_SECRET
# Output: JWT_SECRET=<64-char-random-string>

# Lock it
cargo run -- lock

# Unlock and verify
cargo run -- unlock
cat .env | grep JWT_SECRET
```

### Test Production Mode (Local Vault)

```bash
# Start Vault locally
vault server -dev

# Store test secret
vault kv put secret/data/secure-app/jwt value="test-jwt-secret"

# Configure .env
cat > .env <<EOF
SECRETS_PROVIDER=vault
VAULT_ADDR=http://127.0.0.1:8200
VAULT_TOKEN=root
SECRET_JWT_PATH=secret/data/secure-app/jwt
EOF

# Run application
cargo run
# Should fetch JWT_SECRET from Vault
```

### Test Kubernetes Deployment

```bash
# Deploy Vault to K8s
kubectl apply -f k8s/vault.yaml

# Store secrets
vault kv put secret/data/secure-app/jwt value="prod-jwt-secret"

# Deploy app
kubectl apply -f k8s/

# Verify secret fetching
kubectl logs -f deployment/secure-config-app -n secure-app | grep "Vault"
# Should see: "✅ JWT secret loaded from Vault"
```

---

## ⚠️ Important Notes

### DO ✅

- Use auto-generated secrets for development
- Use Vault for production
- Keep .env encrypted when not editing
- Rotate secrets regularly
- Monitor Vault access logs
- Test failover scenarios

### DON'T ❌

- Commit .env to Git (even with Vault paths)
- Store actual secrets in .env for production
- Use same Vault instance for dev and prod
- Disable auto-rotation in production
- Ignore Vault audit logs
- Share Vault tokens via chat/email

---

## 🔍 Troubleshooting

### Problem: Can't connect to Vault

**Check:**
```bash
# Test connectivity
curl http://vault.service:8200/v1/sys/health

# Check Vault status
kubectl get pods -n vault
kubectl logs -f vault-0 -n vault
```

### Problem: Authentication failed

**Check:**
```bash
# Verify service account
kubectl get sa vault-auth -n secure-app

# Check role binding
kubectl get clusterrolebinding vault-auth-binding

# Test auth
vault write auth/kubernetes/login jwt=$JWT_TOKEN role=secure-app-role
```

### Problem: Secret not found

**Check:**
```bash
# List secrets
vault kv list secret/data/secure-app/

# Verify path
vault kv get secret/data/secure-app/jwt

# Check policy
vault read auth/kubernetes/role/secure-app-role
```

---

## 📚 Architecture Comparison

| Aspect | Development Mode | Production Mode |
|--------|-----------------|----------------|
| Secret Storage | Encrypted .env | HashiCorp Vault |
| Secret Generation | Auto-generated | Manually stored |
| Access Control | Password protection | RBAC + Policies |
| Rotation | Manual | Automatic |
| Audit Trail | Basic | Comprehensive |
| Compliance | Limited | Full |
| Best For | Dev/Test | Production |

---

## 🚀 Quick Reference

```bash
# Development workflow
cargo run -- generate  # Auto-generate secrets
cargo run -- lock      # Encrypt
cargo run              # Run app

# Production workflow
# 1. Store secrets in Vault
vault kv put secret/data/secure-app/jwt value="..."

# 2. Configure .env with Vault paths
# (Uncomment Vault section)

# 3. Deploy
kubectl apply -f k8s/

# Monitoring
kubectl logs -f deployment/secure-config-app -n secure-app
vault audit list  # Check Vault audit logs
```

---

## 🎓 Next Steps

1. **Set up Vault** in your environment
2. **Migrate secrets** from .env to Vault
3. **Update .env** with Vault configuration
4. **Test thoroughly** in staging
5. **Deploy to production**
6. **Monitor and maintain**

---

**Your system now supports both development convenience AND production-grade Vault orchestration!** 🎉☁️🔐

Choose the mode that fits your needs, or use both (dev mode for development, Vault for production).
