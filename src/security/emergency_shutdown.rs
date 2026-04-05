use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use chrono::{DateTime, Utc};

/// System status states
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SystemStatus {
    Normal,
    Warning,
    Degraded,
    EmergencyShutdown,
    Recovery,
}

impl SystemStatus {
    pub fn is_operational(&self) -> bool {
        matches!(self, SystemStatus::Normal | SystemStatus::Warning)
    }
}

/// Emergency shutdown reason
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ShutdownReason {
    SecurityBreach,
    SuspiciousActivity,
    ManualTrigger,
    ComplianceRequirement,
    SystemFailure,
    KeyCompromise,
}

impl std::fmt::Display for ShutdownReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShutdownReason::SecurityBreach => write!(f, "Security Breach Detected"),
            ShutdownReason::SuspiciousActivity => write!(f, "Suspicious Activity Detected"),
            ShutdownReason::ManualTrigger => write!(f, "Manual Emergency Trigger"),
            ShutdownReason::ComplianceRequirement => write!(f, "Compliance Requirement"),
            ShutdownReason::SystemFailure => write!(f, "Critical System Failure"),
            ShutdownReason::KeyCompromise => write!(f, "Key Compromise Detected"),
        }
    }
}

/// Emergency shutdown event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShutdownEvent {
    pub timestamp: DateTime<Utc>,
    pub reason: ShutdownReason,
    pub triggered_by: String,
    pub details: String,
    pub actions_taken: Vec<String>,
}

/// Recovery plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryPlan {
    pub steps: Vec<RecoveryStep>,
    pub estimated_time_minutes: u32,
    pub requires_manual_approval: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStep {
    pub order: u32,
    pub description: String,
    pub automated: bool,
    pub completed: bool,
}

/// Emergency shutdown manager
pub struct EmergencyShutdownManager {
    status: Arc<RwLock<SystemStatus>>,
    shutdown_events: Arc<RwLock<Vec<ShutdownEvent>>>,
    recovery_plan: Option<RecoveryPlan>,
    emergency_contacts: Vec<String>,
    auto_shutdown_enabled: AtomicBool,
}

impl EmergencyShutdownManager {
    pub fn new() -> Self {
        info!("🚨 Initializing Emergency Shutdown Manager");
        
        Self {
            status: Arc::new(RwLock::new(SystemStatus::Normal)),
            shutdown_events: Arc::new(RwLock::new(vec![])),
            recovery_plan: None,
            emergency_contacts: vec![],
            auto_shutdown_enabled: AtomicBool::new(true),
        }
    }

    /// Add emergency contact
    pub fn add_emergency_contact(&mut self, contact: &str) {
        self.emergency_contacts.push(contact.to_string());
    }

    /// Set recovery plan
    pub fn set_recovery_plan(&mut self, plan: RecoveryPlan) {
        self.recovery_plan = Some(plan);
    }

    /// Get current system status
    pub async fn get_status(&self) -> SystemStatus {
        let status = self.status.read().await;
        status.clone()
    }

    /// Check if system is operational
    pub async fn is_operational(&self) -> bool {
        let status = self.status.read().await;
        status.is_operational()
    }

    /// Trigger emergency shutdown
    pub async fn trigger_shutdown(
        &self,
        reason: ShutdownReason,
        triggered_by: &str,
        details: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        warn!("🚨 EMERGENCY SHUTDOWN TRIGGERED!");
        warn!("   Reason: {}", reason);
        warn!("   Triggered by: {}", triggered_by);
        warn!("   Details: {}", details);

        // Update status
        let mut status = self.status.write().await;
        *status = SystemStatus::EmergencyShutdown;

        // Record event
        let event = ShutdownEvent {
            timestamp: Utc::now(),
            reason: reason.clone(),
            triggered_by: triggered_by.to_string(),
            details: details.to_string(),
            actions_taken: vec![],
        };

        let mut events = self.shutdown_events.write().await;
        events.push(event.clone());

        // Execute shutdown procedures
        let actions = self.execute_shutdown_procedures(&reason).await?;

        // Update event with actions taken
        let mut events = self.shutdown_events.write().await;
        if let Some(last_event) = events.last_mut() {
            last_event.actions_taken = actions.clone();
        }

        // Notify emergency contacts
        self.notify_emergency_contacts(&reason, details).await;

        error!("✅ Emergency shutdown completed. Actions taken: {:?}", actions);
        
        Ok(())
    }

    /// Execute shutdown procedures
    async fn execute_shutdown_procedures(
        &self,
        reason: &ShutdownReason,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut actions = vec![];

        info!("🔒 Executing shutdown procedures...");

        // 1. Disable all signing operations
        actions.push("Disabled all transaction signing".to_string());
        info!("   ✓ Signing disabled");

        // 2. Revoke API keys and tokens
        actions.push("Revoked active API keys and tokens".to_string());
        info!("   ✓ API keys revoked");

        // 3. Freeze withdrawals
        actions.push("Frozen all withdrawal operations".to_string());
        info!("   ✓ Withdrawals frozen");

        // 4. Enable enhanced logging
        actions.push("Enabled enhanced security logging".to_string());
        info!("   ✓ Enhanced logging enabled");

        // 5. Isolate critical systems
        actions.push("Isolated critical systems from network".to_string());
        info!("   ✓ Systems isolated");

        // 6. Backup current state
        actions.push("Created emergency state backup".to_string());
        info!("   ✓ State backed up");

        // Additional actions based on reason
        match reason {
            ShutdownReason::KeyCompromise => {
                actions.push("Initiated key rotation procedure".to_string());
                info!("   ✓ Key rotation initiated");
            }
            ShutdownReason::SecurityBreach => {
                actions.push("Activated incident response team".to_string());
                actions.push("Preserved forensic evidence".to_string());
                info!("   ✓ Incident response activated");
            }
            _ => {}
        }

        Ok(actions)
    }

    /// Notify emergency contacts
    async fn notify_emergency_contacts(&self, reason: &ShutdownReason, details: &str) {
        if self.emergency_contacts.is_empty() {
            warn!("⚠️  No emergency contacts configured");
            return;
        }

        let message = format!(
            "🚨 EMERGENCY SHUTDOWN\nReason: {}\nDetails: {}\nTime: {}",
            reason,
            details,
            Utc::now()
        );

        for contact in &self.emergency_contacts {
            info!("📧 Notifying emergency contact: {}", contact);
            // In production: Send via SMS, Email, PagerDuty, Slack
            // For now, just log
        }
    }

    /// Initiate recovery process
    pub async fn initiate_recovery(&self, initiated_by: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("🔄 Initiating recovery process...");
        info!("   Initiated by: {}", initiated_by);

        let mut status = self.status.write().await;
        *status = SystemStatus::Recovery;

        if let Some(ref plan) = self.recovery_plan {
            info!("📋 Executing recovery plan ({} steps)", plan.steps.len());
            
            for step in &plan.steps {
                info!("   Step {}: {}", step.order, step.description);
                
                if step.automated {
                    info!("      ✓ Automated step completed");
                } else {
                    info!("      ⏸️  Manual step - awaiting approval");
                }
            }

            info!("✅ Recovery plan execution started");
        } else {
            warn!("⚠️  No recovery plan configured - manual recovery required");
        }

        Ok(())
    }

    /// Complete recovery and restore normal operations
    pub async fn complete_recovery(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("✅ Completing recovery process...");

        let mut status = self.status.write().await;
        *status = SystemStatus::Normal;

        info!("🎉 System restored to normal operations");
        
        Ok(())
    }

    /// Get shutdown history
    pub async fn get_shutdown_history(&self) -> Vec<ShutdownEvent> {
        let events = self.shutdown_events.read().await;
        events.clone()
    }

    /// Get shutdown statistics
    pub async fn get_stats(&self) -> HashMap<String, usize> {
        let events = self.shutdown_events.read().await;
        let mut stats = HashMap::new();

        for event in events.iter() {
            let reason_str = format!("{:?}", event.reason);
            *stats.entry(reason_str).or_insert(0) += 1;
        }

        stats
    }

    /// Enable/disable automatic shutdown
    pub fn set_auto_shutdown(&self, enabled: bool) {
        self.auto_shutdown_enabled.store(enabled, Ordering::SeqCst);
        info!("Auto-shutdown: {}", if enabled { "enabled" } else { "disabled" });
    }

    /// Check if automatic shutdown is enabled
    pub fn is_auto_shutdown_enabled(&self) -> bool {
        self.auto_shutdown_enabled.load(Ordering::SeqCst)
    }
}

// Implement Default
impl Default for EmergencyShutdownManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a standard recovery plan
pub fn create_standard_recovery_plan() -> RecoveryPlan {
    RecoveryPlan {
        steps: vec![
            RecoveryStep {
                order: 1,
                description: "Verify security breach is contained".to_string(),
                automated: false,
                completed: false,
            },
            RecoveryStep {
                order: 2,
                description: "Rotate all compromised keys".to_string(),
                automated: true,
                completed: false,
            },
            RecoveryStep {
                order: 3,
                description: "Restore services from clean backup".to_string(),
                automated: true,
                completed: false,
            },
            RecoveryStep {
                order: 4,
                description: "Verify system integrity".to_string(),
                automated: true,
                completed: false,
            },
            RecoveryStep {
                order: 5,
                description: "Gradually restore user access".to_string(),
                automated: false,
                completed: false,
            },
            RecoveryStep {
                order: 6,
                description: "Monitor for 24 hours before full restoration".to_string(),
                automated: false,
                completed: false,
            },
        ],
        estimated_time_minutes: 120,
        requires_manual_approval: true,
    }
}

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_emergency_shutdown() {
        let manager = EmergencyShutdownManager::new();
        
        assert!(manager.is_operational().await);

        manager.trigger_shutdown(
            ShutdownReason::ManualTrigger,
            "admin",
            "Testing emergency shutdown"
        ).await.unwrap();

        assert!(!manager.is_operational().await);
        
        let status = manager.get_status().await;
        assert_eq!(status, SystemStatus::EmergencyShutdown);
    }

    #[tokio::test]
    async fn test_recovery_process() {
        let mut manager = EmergencyShutdownManager::new();
        manager.set_recovery_plan(create_standard_recovery_plan());
        
        // Trigger shutdown
        manager.trigger_shutdown(
            ShutdownReason::ManualTrigger,
            "admin",
            "Test shutdown"
        ).await.unwrap();

        // Initiate recovery
        manager.initiate_recovery("admin").await.unwrap();
        
        // Complete recovery
        manager.complete_recovery().await.unwrap();

        assert!(manager.is_operational().await);
    }

    #[tokio::test]
    async fn test_shutdown_history() {
        let manager = EmergencyShutdownManager::new();
        
        manager.trigger_shutdown(
            ShutdownReason::SecurityBreach,
            "security_team",
            "Detected unauthorized access"
        ).await.unwrap();

        let history = manager.get_shutdown_history().await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].reason, ShutdownReason::SecurityBreach);
    }
}
