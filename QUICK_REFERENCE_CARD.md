# 🚀 Quick Reference Card

## 🔐 Secure .env Management

```bash
# Generate template
cargo run -- generate

# Lock with password
cargo run -- lock

# Unlock with password  
cargo run -- unlock

# Check status
cargo run -- status

# Change password
cargo run -- chpasswd

# Multi-environment
APP_ENV=staging cargo run -- lock
APP_ENV=production cargo run -- unlock
```

---

## ☸️ Kubernetes Commands

```bash
# Deploy all
kubectl apply -f k8s/

# Check pods
kubectl get pods -n secure-app

# View logs
kubectl logs -f deployment/secure-config-app -n secure-app

# Scale up/down
kubectl scale deployment/secure-config-app -n secure-app --replicas=5

# Restart
kubectl rollout restart deployment/secure-config-app -n secure-app

# Rollback
kubectl rollout undo deployment/secure-config-app -n secure-app

# Debug
kubectl exec -it <pod-name> -n secure-app -- sh
```

---

## 🔄 CI/CD

```bash
# Trigger deployment
git push origin main

# Tag release
git tag v1.0.0
git push origin v1.0.0

# Manual trigger
# GitHub Actions → Deploy to Production → Run workflow
```

---

## 🔑 Vault Commands

```bash
# Store secret
vault kv put secret/secure-app jwt_secret="value" db_password="value"

# Read secret
vault kv get secret/secure-app

# List versions
vault kv metadata get secret/secure-app

# Rollback version
vault kv rollback -version=1 secret/secure-app

# Backup
vault operator raft snapshot save backup.snap
```

---

## 🐳 Docker

```bash
# Build
docker build -t secure-config:latest .

# Run
docker run -p 8080:8080 secure-config:latest

# Push
docker tag secure-config:latest ghcr.io/username/secure-config:latest
docker push ghcr.io/username/secure-config:latest
```

---

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run specific module
cargo test transaction_validator

# With output
cargo test -- --nocapture

# Coverage (requires tarpaulin)
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

---

## 🔍 Monitoring

```bash
# Pod health
kubectl get pods -n secure-app

# Resource usage
kubectl top pods -n secure-app

# Events
kubectl get events -n secure-app --sort-by='.lastTimestamp'

# Service status
kubectl get svc -n secure-app

# Ingress
kubectl get ingress -n secure-app
```

---

## 🆘 Emergency Procedures

### Security Breach
```bash
# 1. Shutdown immediately
kubectl scale deployment/secure-config-app -n secure-app --replicas=0

# 2. Rotate all secrets in Vault
vault kv put secret/secure-app jwt_secret="new_value"

# 3. Redeploy
kubectl rollout restart deployment/secure-config-app -n secure-app
```

### Rollback Bad Deployment
```bash
# View history
kubectl rollout history deployment/secure-config-app -n secure-app

# Rollback
kubectl rollout undo deployment/secure-config-app -n secure-app
```

### Restore from Backup
```bash
# Stop Vault
kubectl scale statefulset vault -n vault --replicas=0

# Restore
kubectl cp backup.snap vault/$VAULT_POD:/tmp/backup.snap
kubectl exec -it $VAULT_POD -n vault -- vault operator raft snapshot restore /tmp/backup.snap

# Restart
kubectl scale statefulset vault -n vault --replicas=1
```

---

## 📋 Common Issues

### Pod won't start
```bash
kubectl describe pod <pod-name> -n secure-app
kubectl logs <pod-name> -n secure-app --previous
```

### Can't connect to Vault
```bash
kubectl exec -it $VAULT_POD -n vault -- vault status
kubectl get svc -n vault
```

### Secrets not injected
```bash
# Check Vault Agent sidecar
kubectl get pod <pod-name> -n secure-app -o jsonpath='{.spec.containers[*].name}'

# Verify Vault annotations
kubectl get pod <pod-name> -n secure-app -o yaml | grep vault
```

### High memory usage
```bash
# Check resources
kubectl top pods -n secure-app

# Scale horizontally
kubectl scale deployment/secure-config-app -n secure-app --replicas=5
```

---

## 🎯 Environment Variables

```bash
# Application
APP_ENV=development|staging|production
APP_PORT=8080
LOG_LEVEL=info|debug|error

# Vault
VAULT_ADDR=http://vault.vault.svc.cluster.local:8200
VAULT_TOKEN=your-token

# Database
DATABASE_URL=postgresql://user:pass@host:5432/db
DB_POOL_SIZE=10

# JWT
JWT_SECRET=your-secret
JWT_EXPIRY_HOURS=24
```

---

## 📖 Documentation

| Doc | Purpose |
|-----|---------|
| `DEPLOYMENT_GUIDE.md` | Complete production setup |
| `EXCHANGE_LEVEL_FEATURES.md` | Security features guide |
| `SECURE_ENV_GUIDE.md` | Password-protected .env |
| `IMPLEMENTATION_COMPLETE.md` | System overview |
| `README.md` | Project introduction |

---

## 🔗 Useful Links

- Kubernetes Dashboard: `minikube dashboard`
- Grafana: `http://grafana.your-domain.com`
- Prometheus: `http://prometheus.your-domain.com`
- Vault UI: `kubectl port-forward svc/vault-ui 8200:8200 -n vault`

---

**Keep this card handy for quick reference!** 📌
