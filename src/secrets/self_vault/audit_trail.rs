use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Types of audit events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    SecretOperation,
    AccessDenied,
    CredentialOperation,
    SystemEvent,
    PolicyViolation,
    Error,
}

/// A single audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub user: String,
    pub action: String,
    pub resource: String,
    pub details: Option<String>,
    pub ip_address: Option<String>,
    pub success: bool,
}

impl AuditLogEntry {
    /// Create a new audit log entry
    pub fn new(
        event_type: AuditEventType,
        user: &str,
        action: &str,
        resource: &str,
        details: Option<String>,
        success: bool,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            event_type,
            user: user.to_string(),
            action: action.to_string(),
            resource: resource.to_string(),
            details,
            ip_address: None,
            success,
        }
    }

    /// Format the entry for display
    pub fn format(&self) -> String {
        let status = if self.success { "✓" } else { "✗" };
        format!(
            "[{}] {} {} | User: {} | Action: {} | Resource: {}{}",
            self.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            status,
            match &self.event_type {
                AuditEventType::SecretOperation => "SECRET",
                AuditEventType::AccessDenied => "ACCESS_DENIED",
                AuditEventType::CredentialOperation => "CREDENTIAL",
                AuditEventType::SystemEvent => "SYSTEM",
                AuditEventType::PolicyViolation => "POLICY_VIOLATION",
                AuditEventType::Error => "ERROR",
            },
            self.user,
            self.action,
            self.resource,
            self.details.as_ref().map(|d| format!(" | {}", d)).unwrap_or_default()
        )
    }
}

/// Comprehensive audit trail that tracks all vault operations
#[derive(Clone)]
pub struct AuditTrail {
    logs: Arc<RwLock<Vec<AuditLogEntry>>>,
    max_entries: usize,
}

impl AuditTrail {
    /// Create a new audit trail
    pub fn new() -> Self {
        info!("📋 Initializing comprehensive audit trail");
        
        Self {
            logs: Arc::new(RwLock::new(Vec::new())),
            max_entries: 10000, // Keep last 10,000 entries
        }
    }

    /// Log a secret operation (GET, PUT, DELETE, LIST)
    pub async fn log_secret_operation(&self, user: &str, action: &str, path: &str) {
        let entry = AuditLogEntry::new(
            AuditEventType::SecretOperation,
            user,
            action,
            path,
            None,
            true,
        );
        
        self.add_entry(entry).await;
    }

    /// Log an access denied event
    pub async fn log_access_denied(&self, user: &str, action: &str, resource: &str) {
        let entry = AuditLogEntry::new(
            AuditEventType::AccessDenied,
            user,
            action,
            resource,
            Some("Insufficient permissions".to_string()),
            false,
        );
        
        let formatted = entry.format();
        self.add_entry(entry).await;
        info!("⚠️  {}", formatted);
    }

    /// Log a credential operation
    pub async fn log_credential_operation(&self, user: &str, action: &str, path: &str, username: &str) {
        let entry = AuditLogEntry::new(
            AuditEventType::CredentialOperation,
            user,
            action,
            path,
            Some(format!("Username: {}", username)),
            true,
        );
        
        self.add_entry(entry).await;
    }

    /// Log a system event (seal, unseal, etc.)
    pub async fn log_system_event(&self, action: &str, user: &str) {
        let entry = AuditLogEntry::new(
            AuditEventType::SystemEvent,
            user,
            action,
            "vault",
            None,
            true,
        );
        
        info!("🔧 {}", entry.format());
        self.add_entry(entry).await;
    }

    /// Log a policy violation
    pub async fn log_policy_violation(&self, user: &str, policy: &str, details: &str) {
        let entry = AuditLogEntry::new(
            AuditEventType::PolicyViolation,
            user,
            "POLICY_VIOLATION",
            policy,
            Some(details.to_string()),
            false,
        );
        
        info!("🚨 {}", entry.format());
        self.add_entry(entry).await;
    }

    /// Log an error
    pub async fn log_error(&self, user: &str, action: &str, resource: &str, error: &str) {
        let entry = AuditLogEntry::new(
            AuditEventType::Error,
            user,
            action,
            resource,
            Some(error.to_string()),
            false,
        );
        
        self.add_entry(entry).await;
    }

    /// Add an entry to the audit trail
    async fn add_entry(&self, entry: AuditLogEntry) {
        let mut logs = self.logs.write().await;
        
        // Add new entry
        logs.push(entry);
        
        // Trim old entries if exceeding max
        if logs.len() > self.max_entries {
            let excess = logs.len() - self.max_entries;
            logs.drain(0..excess);
        }
    }

    /// Get all audit logs
    pub async fn get_logs(&self) -> Vec<AuditLogEntry> {
        let logs = self.logs.read().await;
        logs.clone()
    }

    /// Get logs filtered by user
    pub async fn get_logs_by_user(&self, user: &str) -> Vec<AuditLogEntry> {
        let logs = self.logs.read().await;
        logs.iter()
            .filter(|entry| entry.user == user)
            .cloned()
            .collect()
    }

    /// Get logs filtered by event type
    pub async fn get_logs_by_type(&self, event_type: AuditEventType) -> Vec<AuditLogEntry> {
        let logs = self.logs.read().await;
        logs.iter()
            .filter(|entry| {
                std::mem::discriminant(&entry.event_type) == std::mem::discriminant(&event_type)
            })
            .cloned()
            .collect()
    }

    /// Get recent logs (last N entries)
    pub async fn get_recent_logs(&self, count: usize) -> Vec<AuditLogEntry> {
        let logs = self.logs.read().await;
        let start = if logs.len() > count {
            logs.len() - count
        } else {
            0
        };
        logs[start..].to_vec()
    }

    /// Export logs as JSON string
    pub async fn export_logs_json(&self) -> Result<String, Box<dyn std::error::Error>> {
        let logs = self.logs.read().await;
        let json = serde_json::to_string_pretty(&*logs)?;
        Ok(json)
    }

    /// Clear all logs (requires admin privileges in production)
    pub async fn clear_logs(&self) {
        let mut logs = self.logs.write().await;
        logs.clear();
        info!("🗑️  Audit trail cleared");
    }

    /// Get total number of log entries
    pub async fn log_count(&self) -> usize {
        let logs = self.logs.read().await;
        logs.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audit_logging() {
        let trail = AuditTrail::new();
        
        trail.log_secret_operation("admin", "GET", "secret/test").await;
        trail.log_access_denied("user1", "READ", "secret/admin-only").await;
        trail.log_system_event("VAULT_SEALED", "System").await;
        
        assert_eq!(trail.log_count().await, 3);
        
        let logs = trail.get_recent_logs(10).await;
        assert_eq!(logs.len(), 3);
    }

    #[tokio::test]
    async fn test_log_filtering() {
        let trail = AuditTrail::new();
        
        trail.log_secret_operation("admin", "GET", "secret/test").await;
        trail.log_secret_operation("user1", "PUT", "secret/data").await;
        trail.log_access_denied("user1", "READ", "secret/admin").await;
        
        let admin_logs = trail.get_logs_by_user("admin").await;
        assert_eq!(admin_logs.len(), 1);
        
        let user1_logs = trail.get_logs_by_user("user1").await;
        assert_eq!(user1_logs.len(), 2);
    }

    #[tokio::test]
    async fn test_max_entries_limit() {
        let trail = AuditTrail::new();
        
        // Add more than max_entries
        for i in 0..100 {
            trail.log_secret_operation("admin", "GET", &format!("secret/{}", i))
                .await;
        }
        
        // Should be limited to max_entries
        assert!(trail.log_count().await <= 10000);
    }
}
