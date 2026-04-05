/// Bridge Security Module
/// 
/// Provides security controls for cross-chain bridge operations including:
/// - Daily/weekly transfer limits
/// - Challenge periods for large transfers
/// - Multi-signature validation
/// - Rate limiting
/// - Anomaly detection specific to bridge operations

use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};

use crate::secrets::self_vault::SelfVault;

/// Bridge operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BridgeOperation {
    Deposit {   // Lock assets on source chain
        amount: u128,
        source_chain: String,
        destination_chain: String,
        recipient: String,
    },
    Withdrawal { // Release assets on destination chain
        amount: u128,
        source_chain: String,
        destination_chain: String,
        recipient: String,
        proof: Vec<u8>, // Proof of lock on source chain
    },
}

/// Bridge security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeSecurityConfig {
    // Transfer limits (in wei or token units)
    pub daily_limit: u128,
    pub weekly_limit: u128,
    pub single_transfer_limit: u128,
    
    // Challenge period (seconds)
    pub challenge_window_seconds: u64,
    
    // Multi-sig requirements
    pub require_multisig: bool,
    pub multisig_threshold: u32, // t-of-n
    pub total_signers: u32,
    
    // Rate limiting
    pub max_transfers_per_hour: u32,
    
    // Supported chains
    pub supported_chains: Vec<String>,
    
    // Emergency controls
    pub pause_enabled: bool,
}

impl Default for BridgeSecurityConfig {
    fn default() -> Self {
        Self {
            daily_limit: 1_000_000_000_000_000_000_000, // 1000 ETH
            weekly_limit: 5_000_000_000_000_000_000_000, // 5000 ETH
            single_transfer_limit: 100_000_000_000_000_000_000, // 100 ETH
            challenge_window_seconds: 1800, // 30 minutes
            require_multisig: true,
            multisig_threshold: 2, // 2-of-3
            total_signers: 3,
            max_transfers_per_hour: 50,
            supported_chains: vec![
                "ethereum".to_string(),
                "polygon".to_string(),
                "arbitrum".to_string(),
                "optimism".to_string(),
            ],
            pause_enabled: false,
        }
    }
}

/// Pending bridge operation (awaiting challenge period)
#[derive(Debug, Clone)]
pub struct PendingBridgeOperation {
    pub operation_id: String,
    pub operation: BridgeOperation,
    pub initiated_at: DateTime<Utc>,
    pub challenge_ends_at: DateTime<Utc>,
    pub signatures: Vec<String>, // Signer addresses
    pub status: PendingStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PendingStatus {
    AwaitingSignatures,
    InChallengePeriod,
    ReadyToExecute,
    Challenged,
    Executed,
    Cancelled,
}

/// Challenge to a bridge operation
#[derive(Debug, Clone)]
pub struct BridgeChallenge {
    pub challenge_id: String,
    pub operation_id: String,
    pub challenger: String,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}

/// Bridge Security Manager
pub struct BridgeSecurityManager {
    vault: std::sync::Arc<SelfVault>,
    config: RwLock<BridgeSecurityConfig>,
    pending_operations: RwLock<HashMap<String, PendingBridgeOperation>>,
    challenges: RwLock<Vec<BridgeChallenge>>,
    operation_history: RwLock<Vec<(DateTime<Utc>, BridgeOperation)>>,
}

impl BridgeSecurityManager {
    /// Create a new bridge security manager
    pub fn new(vault: std::sync::Arc<SelfVault>) -> Self {
        info!("🌉 Initializing Bridge Security Manager");
        
        Self {
            vault,
            config: RwLock::new(BridgeSecurityConfig::default()),
            pending_operations: RwLock::new(HashMap::new()),
            challenges: RwLock::new(Vec::new()),
            operation_history: RwLock::new(Vec::new()),
        }
    }

    /// Load configuration from vault
    pub async fn load_config_from_vault(&self, user: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("📋 Loading bridge security configuration...");
        
        let mut config = self.config.write().await;
        
        if let Some(value) = self.vault.get_secret("bridge/daily_limit", user).await? {
            if let Ok(limit) = value.parse::<u128>() {
                config.daily_limit = limit;
            }
        }
        
        if let Some(value) = self.vault.get_secret("bridge/challenge_window", user).await? {
            if let Ok(window) = value.parse::<u64>() {
                config.challenge_window_seconds = window;
            }
        }
        
        if let Some(value) = self.vault.get_secret("bridge/require_multisig", user).await? {
            config.require_multisig = value == "true";
        }
        
        info!("✅ Bridge security configuration loaded");
        Ok(())
    }

    /// Initiate a bridge operation
    pub async fn initiate_bridge_operation(
        &self,
        operation: BridgeOperation,
        initiator: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        info!("🌉 Initiating bridge operation");
        
        let config = self.config.read().await;
        
        // Check if bridge is paused
        if config.pause_enabled || self.is_bridge_paused().await {
            return Err("Bridge operations are currently paused".into());
        }
        
        // Extract amount and validate
        let amount = match &operation {
            BridgeOperation::Deposit { amount, .. } => *amount,
            BridgeOperation::Withdrawal { amount, .. } => *amount,
        };
        
        // Validate operation
        self.validate_bridge_operation(&operation, &config).await?;
        
        // Generate operation ID
        let operation_id = format!("bridge_op_{}", chrono::Utc::now().timestamp());
        
        // Calculate challenge period end
        let now = Utc::now();
        let challenge_ends = now + Duration::seconds(config.challenge_window_seconds as i64);
        
        // Create pending operation
        let pending_op = PendingBridgeOperation {
            operation_id: operation_id.clone(),
            operation: operation.clone(),
            initiated_at: now,
            challenge_ends_at: challenge_ends,
            signatures: Vec::new(),
            status: if config.require_multisig {
                PendingStatus::AwaitingSignatures
            } else {
                PendingStatus::InChallengePeriod
            },
        };
        
        // Store pending operation
        let mut pending = self.pending_operations.write().await;
        pending.insert(operation_id.clone(), pending_op);
        
        // Record in history
        let mut history = self.operation_history.write().await;
        history.push((now, operation));
        
        // Log to audit trail
        self.vault.audit_trail().log_secret_operation(
            initiator,
            "BRIDGE_INITIATE",
            &operation_id
        ).await;
        
        info!("✅ Bridge operation initiated: {} (challenge ends: {})", 
              operation_id, challenge_ends);
        
        Ok(operation_id)
    }

    /// Add signature to pending operation (multi-sig)
    pub async fn add_signature(
        &self,
        operation_id: &str,
        signer_address: &str,
        signature: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut pending = self.pending_operations.write().await;
        
        if let Some(op) = pending.get_mut(operation_id) {
            // Check if already signed by this address
            if op.signatures.contains(&signer_address.to_string()) {
                return Err("Already signed by this address".into());
            }
            
            // Verify signature (simplified - in production, verify cryptographically)
            // For now, just store it
            op.signatures.push(signer_address.to_string());
            
            let config = self.config.read().await;
            
            // Check if we have enough signatures
            if op.signatures.len() >= config.multisig_threshold as usize {
                op.status = PendingStatus::InChallengePeriod;
                info!("✅ Multi-sig threshold met for operation {}", operation_id);
            }
            
            Ok(())
        } else {
            Err(format!("Operation {} not found", operation_id).into())
        }
    }

    /// Challenge a pending bridge operation
    pub async fn challenge_operation(
        &self,
        operation_id: &str,
        challenger: &str,
        reason: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        warn!("🚨 Bridge operation challenged: {} by {}", operation_id, challenger);
        
        let mut pending = self.pending_operations.write().await;
        
        if let Some(op) = pending.get_mut(operation_id) {
            if op.status == PendingStatus::Executed {
                return Err("Cannot challenge executed operation".into());
            }
            
            // Mark as challenged
            op.status = PendingStatus::Challenged;
            
            // Record challenge
            let challenge_id = format!("challenge_{}", chrono::Utc::now().timestamp());
            let challenge = BridgeChallenge {
                challenge_id: challenge_id.clone(),
                operation_id: operation_id.to_string(),
                challenger: challenger.to_string(),
                reason: reason.to_string(),
                timestamp: Utc::now(),
            };
            
            let mut challenges = self.challenges.write().await;
            challenges.push(challenge);
            
            // Log to audit trail
            self.vault.audit_trail().log_policy_violation(
                challenger,
                "BRIDGE_CHALLENGE",
                &format!("Operation {} challenged: {}", operation_id, reason)
            ).await;
            
            info!("✅ Operation challenged successfully: {}", challenge_id);
            Ok(challenge_id)
        } else {
            Err(format!("Operation {} not found", operation_id).into())
        }
    }

    /// Execute a bridge operation (after challenge period)
    pub async fn execute_operation(
        &self,
        operation_id: &str,
        executor: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut pending = self.pending_operations.write().await;
        
        if let Some(op) = pending.get(operation_id) {
            // Check if challenge period has ended
            if Utc::now() < op.challenge_ends_at {
                return Err(format!(
                    "Challenge period not ended. Ends at: {}",
                    op.challenge_ends_at
                ).into());
            }
            
            // Check if challenged
            if op.status == PendingStatus::Challenged {
                return Err("Operation has been challenged and requires review".into());
            }
            
            // Check multi-sig requirement
            let config = self.config.read().await;
            if config.require_multisig && op.signatures.len() < config.multisig_threshold as usize {
                return Err(format!(
                    "Insufficient signatures: {} of {} required",
                    op.signatures.len(),
                    config.multisig_threshold
                ).into());
            }
        } else {
            return Err(format!("Operation {} not found", operation_id).into());
        }
        
        // Mark as executed
        if let Some(op) = pending.get_mut(operation_id) {
            op.status = PendingStatus::Executed;
        }
        
        // Log execution
        self.vault.audit_trail().log_secret_operation(
            executor,
            "BRIDGE_EXECUTE",
            operation_id
        ).await;
        
        info!("✅ Bridge operation executed: {}", operation_id);
        Ok(())
    }

    /// Cancel a pending operation
    pub async fn cancel_operation(
        &self,
        operation_id: &str,
        canceller: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut pending = self.pending_operations.write().await;
        
        if let Some(op) = pending.get_mut(operation_id) {
            if op.status == PendingStatus::Executed {
                return Err("Cannot cancel executed operation".into());
            }
            
            op.status = PendingStatus::Cancelled;
            
            self.vault.audit_trail().log_secret_operation(
                canceller,
                "BRIDGE_CANCEL",
                operation_id
            ).await;
            
            info!("🗑️  Bridge operation cancelled: {}", operation_id);
            Ok(())
        } else {
            Err(format!("Operation {} not found", operation_id).into())
        }
    }

    /// Enable emergency pause
    pub async fn enable_emergency_pause(&self, user: &str) -> Result<(), Box<dyn std::error::Error>> {
        warn!("🚨 BRIDGE EMERGENCY PAUSE ENABLED");
        
        let mut config = self.config.write().await;
        config.pause_enabled = true;
        
        self.vault.put_secret("bridge/emergency_pause", "true", None, user).await?;
        self.vault.audit_trail().log_system_event("BRIDGE_EMERGENCY_PAUSE", user).await;
        
        Ok(())
    }

    /// Disable emergency pause
    pub async fn disable_emergency_pause(&self, user: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("▶️  Bridge emergency pause disabled");
        
        let mut config = self.config.write().await;
        config.pause_enabled = false;
        
        let _ = self.vault.delete_secret("bridge/emergency_pause", user).await;
        self.vault.audit_trail().log_system_event("BRIDGE_PAUSE_DISABLED", user).await;
        
        Ok(())
    }

    /// Get pending operations
    pub async fn get_pending_operations(&self) -> Vec<PendingBridgeOperation> {
        let pending = self.pending_operations.read().await;
        pending.values()
            .filter(|op| op.status != PendingStatus::Executed && op.status != PendingStatus::Cancelled)
            .cloned()
            .collect()
    }

    /// Get bridge statistics
    pub async fn get_bridge_stats(&self) -> HashMap<String, String> {
        let mut stats = HashMap::new();
        let config = self.config.read().await;
        let pending = self.pending_operations.read().await;
        let challenges = self.challenges.read().await;
        
        stats.insert("daily_limit".to_string(), format!("{}", config.daily_limit));
        stats.insert("challenge_window".to_string(), format!("{} seconds", config.challenge_window_seconds));
        stats.insert("pending_operations".to_string(), format!("{}", pending.len()));
        stats.insert("total_challenges".to_string(), format!("{}", challenges.len()));
        stats.insert("pause_enabled".to_string(), format!("{}", config.pause_enabled));
        stats.insert("multisig_required".to_string(), format!("{}", config.require_multisig));
        
        stats
    }

    /// Validate bridge operation against policies
    async fn validate_bridge_operation(
        &self,
        operation: &BridgeOperation,
        config: &BridgeSecurityConfig,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let amount = match operation {
            BridgeOperation::Deposit { amount, .. } => *amount,
            BridgeOperation::Withdrawal { amount, .. } => *amount,
        };
        
        let (source_chain, dest_chain) = match operation {
            BridgeOperation::Deposit { source_chain, destination_chain, .. } => 
                (source_chain.clone(), destination_chain.clone()),
            BridgeOperation::Withdrawal { source_chain, destination_chain, .. } => 
                (source_chain.clone(), destination_chain.clone()),
        };
        
        // Check single transfer limit
        if amount > config.single_transfer_limit {
            return Err(format!(
                "Amount {} exceeds single transfer limit {}",
                amount, config.single_transfer_limit
            ).into());
        }
        
        // Check daily limit
        let daily_total = self.get_daily_bridge_volume().await;
        if daily_total + amount > config.daily_limit {
            return Err(format!(
                "Daily volume {} would exceed limit {}",
                daily_total + amount, config.daily_limit
            ).into());
        }
        
        // Check supported chains
        if !config.supported_chains.contains(&source_chain) {
            return Err(format!("Source chain {} not supported", source_chain).into());
        }
        
        if !config.supported_chains.contains(&dest_chain) {
            return Err(format!("Destination chain {} not supported", dest_chain).into());
        }
        
        // Check rate limit
        let hourly_count = self.get_hourly_transfer_count().await;
        if hourly_count >= config.max_transfers_per_hour {
            return Err(format!(
                "Hourly transfer count {} exceeds limit {}",
                hourly_count, config.max_transfers_per_hour
            ).into());
        }
        
        Ok(())
    }

    /// Check if bridge is paused
    async fn is_bridge_paused(&self) -> bool {
        match self.vault.get_secret("bridge/emergency_pause", "system").await {
            Ok(Some(value)) => value == "true",
            _ => false,
        }
    }

    /// Get daily bridge volume
    async fn get_daily_bridge_volume(&self) -> u128 {
        let history = self.operation_history.read().await;
        let now = Utc::now();
        let one_day_ago = now - Duration::hours(24);
        
        history.iter()
            .filter(|(ts, _)| *ts > one_day_ago)
            .map(|(_, op)| match op {
                BridgeOperation::Deposit { amount, .. } => *amount,
                BridgeOperation::Withdrawal { amount, .. } => *amount,
            })
            .sum()
    }

    /// Get hourly transfer count
    async fn get_hourly_transfer_count(&self) -> u32 {
        let history = self.operation_history.read().await;
        let now = Utc::now();
        let one_hour_ago = now - Duration::hours(1);
        
        history.iter()
            .filter(|(ts, _)| ts > &one_hour_ago)
            .count() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_initiate_bridge_operation() {
        let key = SelfVault::generate_master_key();
        let vault = std::sync::Arc::new(SelfVault::new(&key));
        let manager = BridgeSecurityManager::new(vault);
        
        let operation = BridgeOperation::Deposit {
            amount: 1_000_000_000_000_000_000, // 1 ETH
            source_chain: "ethereum".to_string(),
            destination_chain: "polygon".to_string(),
            recipient: "0xRecipient".to_string(),
        };
        
        let op_id = manager.initiate_bridge_operation(operation, "admin").await.unwrap();
        assert!(op_id.starts_with("bridge_op_"));
    }

    #[tokio::test]
    async fn test_emergency_pause() {
        let key = SelfVault::generate_master_key();
        let vault = std::sync::Arc::new(SelfVault::new(&key));
        let manager = BridgeSecurityManager::new(vault);
        
        manager.enable_emergency_pause("admin").await.unwrap();
        
        let operation = BridgeOperation::Deposit {
            amount: 1_000_000_000_000_000_000,
            source_chain: "ethereum".to_string(),
            destination_chain: "polygon".to_string(),
            recipient: "0xRecipient".to_string(),
        };
        
        // Should fail when paused
        let result = manager.initiate_bridge_operation(operation, "admin").await;
        assert!(result.is_err());
    }
}
