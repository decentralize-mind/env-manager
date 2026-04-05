# 🚀 Production Deployment Guide

Complete guide for deploying your secure env-manager to production with Kubernetes, Vault, and CI/CD.

---

## 📋 Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Prerequisites](#prerequisites)
3. [Local Development](#local-development)
4. [Kubernetes Deployment](#kubernetes-deployment)
5. [Vault Integration](#vault-integration)
6. [CI/CD Pipeline](#cicd-pipeline)
7. [Multi-Environment Setup](#multi-environment-setup)
8. [Monitoring & Observability](#monitoring--observability)
9. [Security Hardening](#security-hardening)
10. [Disaster Recovery](#disaster-recovery)

---

## 🏗️ Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│              Developer Workflow                      │
│  Code → Git → GitHub Actions → Docker Registry      │
└──────────────────┬──────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────┐
│           Kubernetes Cluster                         │
│  ┌─────────────┐    ┌──────────────────┐            │
│  │   Vault     │◄──►│  Vault Agent     │            │
│  │  (Secrets)  │    │  (Sidecar)       │            │
│  └─────────────┘    └────────┬─────────┘            │
│                              │                       │
│                   ┌──────────▼─────────┐            │
│                   │  Application Pods   │            │
│                   │  (secure-config)    │            │
│                   └──────────┬─────────┘            │
└──────────────────────────────┼──────────────────────┘
                               │
                    ┌──────────▼──────────┐
                    │   Load Balancer     │
                    │   (nginx ingress)   │
                    └──────────┬──────────┘
                               │
                    ┌──────────▼──────────┐
                    │   Users/Clients     │
                    └─────────────────────┘
```

---

## 📦 Prerequisites

### Required Tools

```bash
# Container runtime
docker --version        # >= 20.10

# Kubernetes
kubectl version         # >= 1.25
helm version           # >= 3.10 (optional)

# Infrastructure
minikube version       # For local testing
kind version          # Alternative to minikube

# Secrets Management
vault version         # >= 1.13

# CI/CD
gh --version          # GitHub CLI (optional)
```

### Cloud Providers (Choose One)

- **AWS**: EKS + RDS + Secrets Manager
- **GCP**: GKE + Cloud SQL + Secret Manager  
- **Azure**: AKS + Azure Database + Key Vault
- **Self-hosted**: Bare metal Kubernetes + Vault

---

## 💻 Local Development

### 1. Clone and Setup

```bash
git clone <your-repo>
cd env-manager

# Install Rust toolchain
rustup install stable
rustup default stable
```

### 2. Generate Environment Files

```bash
# Development environment
cargo run -- generate

# Edit with your local settings
nano .env

# Lock when done
cargo run -- lock
```

### 3. Run Locally

```bash
# Without Vault (development mode)
cargo run

# With Vault (if configured)
export VAULT_ADDR=http://127.0.0.1:8200
export VAULT_TOKEN=your-token
cargo run
```

### 4. Test All Features

```bash
# Check status
cargo run -- status

# Run tests
cargo test

# Build release binary
cargo build --release
```

---

## ☸️ Kubernetes Deployment

### Option A: Minikube (Local Testing)

```bash
# Start minikube
minikube start --cpus=4 --memory=8192

# Enable addons
minikube addons enable ingress
minikube addons enable metrics-server

# Deploy application
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/vault-auth.yaml
kubectl apply -f k8s/secrets.yaml
kubectl apply -f k8s/deployment.yaml
kubectl apply -f k8s/service.yaml

# Verify deployment
kubectl get pods -n secure-app
kubectl get svc -n secure-app

# Access application
minikube service secure-config-service -n secure-app
```

### Option B: Production Kubernetes

#### 1. Prepare Cluster

```bash
# Set context
kubectl config use-context your-production-cluster

# Create namespace
kubectl apply -f k8s/namespace.yaml

# Configure RBAC
kubectl apply -f k8s/vault-auth.yaml
```

#### 2. Configure Secrets

**Option 1: Kubernetes Secrets** (Basic)
```bash
# Encode secrets
echo -n "your-jwt-secret" | base64
echo -n "your-db-password" | base64

# Update k8s/secrets.yaml with encoded values
kubectl apply -f k8s/secrets.yaml
```

**Option 2: HashiCorp Vault** (Recommended)
See [Vault Integration](#vault-integration) section below.

#### 3. Deploy Application

```bash
# Update deployment.yaml with your image
# Replace 'secure-config:latest' with actual image

# Apply manifests
kubectl apply -f k8s/deployment.yaml
kubectl apply -f k8s/service.yaml

# Monitor rollout
kubectl rollout status deployment/secure-config-app -n secure-app

# Check logs
kubectl logs -f deployment/secure-config-app -n secure-app
```

#### 4. Configure Ingress

```bash
# Update k8s/service.yaml with your domain
# Deploy cert-manager for SSL
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.13.0/cert-manager.yaml

# Create cluster issuer
cat <<EOF | kubectl apply -f -
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: letsencrypt-prod
spec:
  acme:
    server: https://acme-v02.api.letsencrypt.org/directory
    email: your-email@domain.com
    privateKeySecretRef:
      name: letsencrypt-prod
    solvers:
      - http01:
          ingress:
            class: nginx
EOF

# Apply ingress
kubectl apply -f k8s/service.yaml
```

---

## 🔐 Vault Integration

### 1. Install Vault on Kubernetes

```bash
# Add Helm repo
helm repo add hashicorp https://helm.releases.hashicorp.com

# Install Vault
helm install vault hashicorp/vault \
  --namespace vault \
  --create-namespace \
  --set "server.dev.enabled=true"  # Remove for production
```

### 2. Configure Vault

```bash
# Get Vault pod
export VAULT_POD=$(kubectl get pods -n vault -l app.kubernetes.io/name=vault -o jsonpath='{.items[0].metadata.name}')

# Initialize Vault
kubectl exec -it $VAULT_POD -n vault -- vault operator init

# Unseal Vault (run 3 times with different keys)
kubectl exec -it $VAULT_POD -n vault -- vault operator unseal

# Login
kubectl exec -it $VAULT_POD -n vault -- vault login
```

### 3. Store Secrets in Vault

```bash
# Enable KV secrets engine
kubectl exec -it $VAULT_POD -n vault -- vault secrets enable kv

# Store secrets
kubectl exec -it $VAULT_POD -n vault -- vault kv put secret/secure-app \
  jwt_secret="your-jwt-secret" \
  db_password="your-db-password" \
  api_key="your-api-key"

# Verify
kubectl exec -it $VAULT_POD -n vault -- vault kv get secret/secure-app
```

### 4. Configure Vault Agent Injector

The deployment.yaml already includes Vault Agent annotations. The sidecar will automatically inject secrets as environment variables.

### 5. Test Vault Integration

```bash
# Restart pods to pick up new secrets
kubectl rollout restart deployment/secure-config-app -n secure-app

# Check if secrets are injected
kubectl exec -it $(kubectl get pod -n secure-app -l app=secure-config -o jsonpath='{.items[0].metadata.name}') -n secure-app -- env | grep JWT_SECRET
```

---

## 🔄 CI/CD Pipeline

### 1. GitHub Repository Setup

```bash
# Initialize git
git init
git add .
git commit -m "Initial commit"

# Create remote repository
gh repo create your-username/env-manager --public
git remote add origin https://github.com/your-username/env-manager.git
git push -u origin main
```

### 2. Configure GitHub Secrets

Go to **Settings → Secrets and variables → Actions** and add:

| Secret Name | Value | Description |
|------------|-------|-------------|
| `KUBE_CONFIG` | Base64-encoded kubeconfig | Kubernetes cluster access |
| `SLACK_WEBHOOK` | Slack webhook URL | Deployment notifications |
| `VAULT_TOKEN` | Vault token | For CI/CD secret access |
| `DOCKER_PASSWORD` | Docker registry password | Image push authentication |

### 3. Trigger Deployment

```bash
# Automatic: Push to main branch
git push origin main

# Manual: Via GitHub Actions UI
# Go to Actions → Deploy to Production → Run workflow

# Tag-based deployment
git tag v1.0.0
git push origin v1.0.0
```

### 4. Monitor Deployment

```bash
# Watch deployment progress
kubectl get pods -n secure-app -w

# Check rollout status
kubectl rollout status deployment/secure-config-app -n secure-app

# View recent events
kubectl get events -n secure-app --sort-by='.lastTimestamp'
```

---

## 🌍 Multi-Environment Setup

### Environment Structure

```
.env                  # Development (local)
.env.staging          # Staging environment
.env.production       # Production environment
```

### 1. Create Environment Files

```bash
# Development
APP_ENV=development cargo run -- generate

# Staging
APP_ENV=staging cargo run -- generate
# Edit .env.staging with staging values
APP_ENV=staging cargo run -- lock

# Production
APP_ENV=production cargo run -- generate
# Edit .env.production with production values
APP_ENV=production cargo run -- lock
```

### 2. Deploy to Different Environments

```bash
# Staging
kubectl apply -f k8s/staging/  # Separate K8s manifests

# Production
kubectl apply -f k8s/production/
```

### 3. Environment-Specific Configuration

Update deployment manifests to set `APP_ENV`:

```yaml
env:
  - name: APP_ENV
    value: "staging"  # or "production"
```

---

## 📊 Monitoring & Observability

### 1. Prometheus Metrics

Add to deployment.yaml:

```yaml
annotations:
  prometheus.io/scrape: "true"
  prometheus.io/port: "9090"
```

### 2. Grafana Dashboards

Import dashboard ID: `12345` (Rust application metrics)

### 3. Logging

```bash
# Stream logs
kubectl logs -f deployment/secure-config-app -n secure-app

# Search logs
kubectl logs deployment/secure-config-app -n secure-app | grep "ERROR"

# Export logs
kubectl logs deployment/secure-config-app -n secure-app > app.log
```

### 4. Alerting

Configure alerts for:
- Pod restarts > 3 in 1 hour
- Error rate > 5%
- Response time > 1s
- Memory usage > 80%

---

## 🛡️ Security Hardening

### 1. Network Policies

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: deny-all
  namespace: secure-app
spec:
  podSelector: {}
  policyTypes:
    - Ingress
    - Egress
```

### 2. Pod Security Standards

```yaml
securityContext:
  runAsNonRoot: true
  readOnlyRootFilesystem: true
  allowPrivilegeEscalation: false
```

### 3. Resource Limits

Already configured in deployment.yaml:
- CPU: 250m - 500m
- Memory: 256Mi - 512Mi

### 4. Regular Updates

```bash
# Update dependencies monthly
cargo update

# Rebuild and redeploy
cargo build --release
docker build -t secure-config:latest .
kubectl set image deployment/secure-config-app app=secure-config:latest -n secure-app
```

---

## 🆘 Disaster Recovery

### 1. Backup Secrets

```bash
# Backup Vault data
kubectl exec -it $VAULT_POD -n vault -- vault operator raft snapshot save /tmp/vault-backup.snap

# Copy to local
kubectl cp vault/$VAULT_POD:/tmp/vault-backup.snap ./vault-backup.snap

# Store securely (S3, GCS, etc.)
aws s3 cp vault-backup.snap s3://your-backup-bucket/
```

### 2. Restore from Backup

```bash
# Stop Vault
kubectl scale statefulset vault -n vault --replicas=0

# Restore snapshot
kubectl cp vault-backup.snap vault/$VAULT_POD:/tmp/vault-backup.snap
kubectl exec -it $VAULT_POD -n vault -- vault operator raft snapshot restore /tmp/vault-backup.snap

# Restart Vault
kubectl scale statefulset vault -n vault --replicas=1
```

### 3. Rollback Deployment

```bash
# View rollout history
kubectl rollout history deployment/secure-config-app -n secure-app

# Rollback to previous version
kubectl rollout undo deployment/secure-config-app -n secure-app

# Rollback to specific revision
kubectl rollout undo deployment/secure-config-app -n secure-app --to-revision=2
```

### 4. Emergency Shutdown

If security breach detected:

```bash
# Scale down to zero
kubectl scale deployment/secure-config-app -n secure-app --replicas=0

# Rotate all secrets in Vault
# Update deployment with new secrets
# Scale back up
kubectl scale deployment/secure-config-app -n secure-app --replicas=3
```

---

## 📝 Maintenance Checklist

### Daily
- [ ] Check pod health: `kubectl get pods -n secure-app`
- [ ] Review error logs
- [ ] Monitor resource usage

### Weekly
- [ ] Review audit logs
- [ ] Check for dependency updates
- [ ] Test backup restoration

### Monthly
- [ ] Rotate passwords
- [ ] Update base images
- [ ] Review access controls
- [ ] Run security scans

### Quarterly
- [ ] Full disaster recovery drill
- [ ] Penetration testing
- [ ] Compliance audit
- [ ] Performance optimization

---

## 🎯 Quick Reference Commands

```bash
# Deploy
kubectl apply -f k8s/

# Check status
kubectl get all -n secure-app

# View logs
kubectl logs -f deployment/secure-config-app -n secure-app

# Scale
kubectl scale deployment/secure-config-app -n secure-app --replicas=5

# Restart
kubectl rollout restart deployment/secure-config-app -n secure-app

# Debug
kubectl exec -it <pod-name> -n secure-app -- sh

# Cleanup
kubectl delete namespace secure-app
```

---

## 📚 Additional Resources

- [Kubernetes Documentation](https://kubernetes.io/docs/)
- [HashiCorp Vault Guide](https://www.vaultproject.io/docs)
- [GitHub Actions Docs](https://docs.github.com/en/actions)
- [Rust Production Best Practices](https://rust-lang.github.io/api-guidelines/)

---

## 🆘 Support

For issues:
1. Check logs: `kubectl logs -f deployment/secure-config-app -n secure-app`
2. Review events: `kubectl get events -n secure-app`
3. Check Vault: `kubectl exec -it $VAULT_POD -n vault -- vault status`
4. Consult documentation above

---

**Your production deployment is now ready!** 🚀✨

All components are configured for security, scalability, and reliability.
