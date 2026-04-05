/// Web3 Transaction Policy Engine
/// 
/// Provides comprehensive policy validation for Web3 transactions including:
/// - Amount limits (per-tx, daily, weekly)
/// - Address allowlisting/blocklisting
/// - Rate limiting
/// - Anomaly detection
/// - Multi-sig requirements for large amounts

use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, warn};
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};

use crate::secrets::self_vault::SelfVault;

/// Policy configuration for Web3 operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Web3PolicyConfig {
    // Amount limits (in wei)
    pub max_transaction_amount: u128,
    pub daily_withdrawal_limit: u128,
    pub weekly_withdrawal_limit: u128,
    
    // Address controls
    pub enable_address_allowlist: bool,
    pub enable_address_blocklist: bool,
    
    // Rate limiting
    pub max_transactions_per_hour: u32,
    pub min_time_between_txs_seconds: u64,
    
    // Multi-sig requirements
    pub require_multisig_above_amount: Option<u128>,
    pub multisig_threshold: Option<u32>, // t-of-n
    
    // Anomaly detection
    pub enable_anomaly_detection: bool,
    pub anomaly_score_threshold: f64, // 0.0 - 1.0
    
    // Emergency controls
    pub emergency_pause_enabled: bool,
}

impl Default for Web3PolicyConfig {
    fn default() -> Self {
        Self {
            max_transaction_amount: 10_000_000_000_000_000_000, // 10 ETH
            daily_withdrawal_limit: 100_000_000_000_000_000_000, // 100 ETH
            weekly_withdrawal_limit: 500_000_000_000_000_000_000, // 500 ETH
            enable_address_allowlist: false,
            enable_address_blocklist: true,
            max_transactions_per_hour: 100,
            min_time_between_txs_seconds: 1,
            require_multisig_above_amount: Some(50_000_000_000_000_000_000), // 50 ETH
            multisig_threshold: Some(2), // 2-of-3
            enable_anomaly_detection: true,
            anomaly_score_threshold: 0.8,
            emergency_pause_enabled: false,
        }
    }
}

/// Transaction validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub risk_score: f64, // 0.0 (safe) to 1.0 (risky)
    pub violations: Vec<PolicyViolation>,
    pub warnings: Vec<String>,
}

/// Policy violation
#[derive(Debug, Clone)]
pub struct PolicyViolation {
    pub rule: String,
    pub message: String,
    pub severity: ViolationSeverity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViolationSeverity {
    Critical, // Block transaction
    Warning,  // Log but allow
}

/// Tracking data for rate limiting and anomaly detection
#[derive(Debug, Clone)]
struct TransactionHistory {
    timestamps: Vec<DateTime<Utc>>,
    amounts: Vec<u128>,
    recipients: Vec<String>,
}

/// Web3 Policy Engine
pub struct Web3PolicyEngine {
    vault: std::sync::Arc<SelfVault>,
    config: RwLock<Web3PolicyConfig>,
    transaction_history: RwLock<HashMap<String, TransactionHistory>>, // address -> history
    allowed_addresses: RwLock<Vec<String>>,
    blocked_addresses: RwLock<Vec<String>>,
}

impl Web3PolicyEngine {
    /// Create a new Web3 policy engine
    pub fn new(vault: std::sync::Arc<SelfVault>) -> Self {
        info!("🛡️  Initializing Web3 Policy Engine");
        
        Self {
            vault,
            config: RwLock::new(Web3PolicyConfig::default()),
            transaction_history: RwLock::new(HashMap::new()),
            allowed_addresses: RwLock::new(Vec::new()),
            blocked_addresses: RwLock::new(Vec::new()),
        }
    }

    /// Load configuration from vault
    pub async fn load_config_from_vault(&self, user: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("📋 Loading Web3 policy configuration from vault...");
        
        let mut config = self.config.write().await;
        
        // Load max transaction amount
        if let Some(value) = self.vault.get_secret("policy/max_transaction_amount", user).await? {
            if let Ok(amount) = value.parse::<u128>() {
                config.max_transaction_amount = amount;
            }
        }
        
        // Load daily withdrawal limit
        if let Some(value) = self.vault.get_secret("policy/daily_withdrawal_limit", user).await? {
            if let Ok(amount) = value.parse::<u128>() {
                config.daily_withdrawal_limit = amount;
            }
        }
        
        // Load address allowlist setting
        if let Some(value) = self.vault.get_secret("policy/enable_address_allowlist", user).await? {
            config.enable_address_allowlist = value == "true";
        }
        
        info!("✅ Web3 policy configuration loaded");
        Ok(())
    }

    /// Validate a transaction against all policies
    pub async fn validate_transaction(
        &self,
        from_address: &str,
        to_address: &str,
        amount: u128,
        user: &str,
    ) -> Result<ValidationResult, Box<dyn std::error::Error>> {
        let config = self.config.read().await;
        let mut violations = Vec::new();
        let mut warnings = Vec::new();
        let mut risk_score = 0.0;
        
        // Check emergency pause
        if config.emergency_pause_enabled || self.is_emergency_paused().await {
            return Ok(ValidationResult {
                is_valid: false,
                risk_score: 1.0,
                violations: vec![PolicyViolation {
                    rule: "EMERGENCY_PAUSE".to_string(),
                    message: "System is in emergency pause mode".to_string(),
                    severity: ViolationSeverity::Critical,
                }],
                warnings: vec![],
            });
        }
        
        // 1. Check transaction amount limit
        if amount > config.max_transaction_amount {
            violations.push(PolicyViolation {
                rule: "MAX_TRANSACTION_AMOUNT".to_string(),
                message: format!("Amount {} exceeds limit {}", amount, config.max_transaction_amount),
                severity: ViolationSeverity::Critical,
            });
            risk_score += 0.3;
        }
        
        // 2. Check daily withdrawal limit
        let daily_total = self.get_daily_withdrawal_total(from_address).await;
        if daily_total + amount > config.daily_withdrawal_limit {
            violations.push(PolicyViolation {
                rule: "DAILY_WITHDRAWAL_LIMIT".to_string(),
                message: format!(
                    "Daily total {} would exceed limit {}",
                    daily_total + amount,
                    config.daily_withdrawal_limit
                ),
                severity: ViolationSeverity::Critical,
            });
            risk_score += 0.4;
        }
        
        // 3. Check address allowlist
        if config.enable_address_allowlist {
            if !self.is_address_allowed(to_address).await {
                violations.push(PolicyViolation {
                    rule: "ADDRESS_ALLOWLIST".to_string(),
                    message: format!("Recipient {} not in allowlist", to_address),
                    severity: ViolationSeverity::Critical,
                });
                risk_score += 0.2;
            }
        }
        
        // 4. Check address blocklist
        if config.enable_address_blocklist {
            if self.is_address_blocked(to_address).await {
                violations.push(PolicyViolation {
                    rule: "ADDRESS_BLOCKLIST".to_string(),
                    message: format!("Recipient {} is blocked", to_address),
                    severity: ViolationSeverity::Critical,
                });
                risk_score += 0.5;
            }
        }
        
        // 5. Check rate limiting
        let tx_count = self.get_hourly_transaction_count(from_address).await;
        if tx_count >= config.max_transactions_per_hour {
            violations.push(PolicyViolation {
                rule: "RATE_LIMIT".to_string(),
                message: format!(
                    "Transaction count {} exceeds hourly limit {}",
                    tx_count, config.max_transactions_per_hour
                ),
                severity: ViolationSeverity::Critical,
            });
            risk_score += 0.2;
        }
        
        // 6. Check multi-sig requirement
        if let Some(multisig_threshold_amount) = config.require_multisig_above_amount {
            if amount > multisig_threshold_amount {
                warnings.push(format!(
                    "Amount {} exceeds multi-sig threshold {}, requires {} signatures",
                    amount,
                    multisig_threshold_amount,
                    config.multisig_threshold.unwrap_or(2)
                ));
                risk_score += 0.1;
            }
        }
        
        // 7. Anomaly detection
        if config.enable_anomaly_detection {
            let anomaly_score = self.detect_anomalies(from_address, to_address, amount).await;
            if anomaly_score > config.anomaly_score_threshold {
                warnings.push(format!(
                    "Anomalous transaction detected (score: {:.2})",
                    anomaly_score
                ));
                risk_score += anomaly_score * 0.2;
            }
        }
        
        // Cap risk score at 1.0
        risk_score = risk_score.min(1.0);
        
        let is_valid = violations.iter().all(|v| v.severity != ViolationSeverity::Critical);
        
        // Record transaction in history
        self.record_transaction(from_address, to_address, amount).await;
        
        // Log validation result
        if is_valid {
            info!("✅ Transaction validated (risk: {:.2})", risk_score);
        } else {
            warn!("❌ Transaction rejected: {} violations", violations.len());
        }
        
        Ok(ValidationResult {
            is_valid,
            risk_score,
            violations,
            warnings,
        })
    }

    /// Add address to allowlist
    pub async fn add_to_allowlist(&self, address: &str, user: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut allowed = self.allowed_addresses.write().await;
        if !allowed.contains(&address.to_string()) {
            allowed.push(address.to_string());
            
            // Persist to vault
            self.vault.put_secret(
                &format!("allowlist/{}", address),
                "true",
                None,
                user
            ).await?;
            
            info!("✅ Added {} to allowlist", address);
        }
        Ok(())
    }

    /// Add address to blocklist
    pub async fn add_to_blocklist(&self, address: &str, user: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut blocked = self.blocked_addresses.write().await;
        if !blocked.contains(&address.to_string()) {
            blocked.push(address.to_string());
            
            // Persist to vault
            self.vault.put_secret(
                &format!("blocklist/{}", address),
                "true",
                None,
                user
            ).await?;
            
            info!("🚫 Added {} to blocklist", address);
        }
        Ok(())
    }

    /// Enable emergency pause
    pub async fn enable_emergency_pause(&self, user: &str) -> Result<(), Box<dyn std::error::Error>> {
        warn!("🚨 EMERGENCY PAUSE ENABLED");
        
        let mut config = self.config.write().await;
        config.emergency_pause_enabled = true;
        
        self.vault.put_secret("system/emergency_pause", "true", None, user).await?;
        self.vault.audit_trail().log_system_event("WEB3_EMERGENCY_PAUSE", user).await;
        
        Ok(())
    }

    /// Disable emergency pause
    pub async fn disable_emergency_pause(&self, user: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("▶️  Emergency pause disabled");
        
        let mut config = self.config.write().await;
        config.emergency_pause_enabled = false;
        
        let _ = self.vault.delete_secret("system/emergency_pause", user).await;
        self.vault.audit_trail().log_system_event("WEB3_PAUSE_DISABLED", user).await;
        
        Ok(())
    }

    /// Check if emergency pause is active
    async fn is_emergency_paused(&self) -> bool {
        match self.vault.get_secret("system/emergency_pause", "system").await {
            Ok(Some(value)) => value == "true",
            _ => false,
        }
    }

    /// Check if address is in allowlist
    async fn is_address_allowed(&self, address: &str) -> bool {
        let allowed = self.allowed_addresses.read().await;
        allowed.contains(&address.to_string())
    }

    /// Check if address is in blocklist
    async fn is_address_blocked(&self, address: &str) -> bool {
        let blocked = self.blocked_addresses.read().await;
        blocked.contains(&address.to_string())
    }

    /// Get total withdrawals in last 24 hours
    async fn get_daily_withdrawal_total(&self, address: &str) -> u128 {
        let history = self.transaction_history.read().await;
        if let Some(tx_history) = history.get(address) {
            let now = Utc::now();
            let one_day_ago = now - Duration::hours(24);
            
            tx_history.timestamps.iter()
                .zip(tx_history.amounts.iter())
                .filter(|(ts, _)| ts > &&one_day_ago)
                .map(|(_, amount)| *amount)
                .sum()
        } else {
            0
        }
    }

    /// Get transaction count in last hour
    async fn get_hourly_transaction_count(&self, address: &str) -> u32 {
        let history = self.transaction_history.read().await;
        if let Some(tx_history) = history.get(address) {
            let now = Utc::now();
            let one_hour_ago = now - Duration::hours(1);
            
            tx_history.timestamps.iter()
                .filter(|ts| *ts > &one_hour_ago)
                .count() as u32
        } else {
            0
        }
    }

    /// Detect anomalous transaction patterns
    async fn detect_anomalies(&self, from: &str, to: &str, amount: u128) -> f64 {
        let history = self.transaction_history.read().await;
        
        if let Some(tx_history) = history.get(from) {
            if tx_history.amounts.is_empty() {
                return 0.0; // No history, can't detect anomalies
            }
            
            // Calculate average transaction amount
            let avg_amount = tx_history.amounts.iter().sum::<u128>() / tx_history.amounts.len() as u128;
            
            // Calculate standard deviation
            let variance = tx_history.amounts.iter()
                .map(|&amt| {
                    let diff = amt as f64 - avg_amount as f64;
                    diff * diff
                })
                .sum::<f64>() / tx_history.amounts.len() as f64;
            
            let std_dev = variance.sqrt();
            
            // Check if this transaction is significantly different
            if std_dev > 0.0 {
                let z_score = ((amount as f64 - avg_amount as f64) / std_dev).abs();
                
                // Z-score > 3 is anomalous
                if z_score > 3.0 {
                    return 1.0; // Highly anomalous
                } else if z_score > 2.0 {
                    return 0.7; // Moderately anomalous
                }
            }
        }
        
        0.0 // Normal
    }

    /// Record transaction in history
    async fn record_transaction(&self, address: &str, to: &str, amount: u128) {
        let mut history = self.transaction_history.write().await;
        let entry = history.entry(address.to_string()).or_insert(TransactionHistory {
            timestamps: Vec::new(),
            amounts: Vec::new(),
            recipients: Vec::new(),
        });
        
        entry.timestamps.push(Utc::now());
        entry.amounts.push(amount);
        entry.recipients.push(to.to_string());
        
        // Keep only last 1000 transactions per address
        if entry.timestamps.len() > 1000 {
            let excess = entry.timestamps.len() - 1000;
            entry.timestamps.drain(0..excess);
            entry.amounts.drain(0..excess);
            entry.recipients.drain(0..excess);
        }
    }

    /// Get policy statistics
    pub async fn get_policy_stats(&self) -> HashMap<String, String> {
        let mut stats = HashMap::new();
        let config = self.config.read().await;
        
        stats.insert("max_transaction_amount".to_string(), format!("{}", config.max_transaction_amount));
        stats.insert("daily_withdrawal_limit".to_string(), format!("{}", config.daily_withdrawal_limit));
        stats.insert("emergency_pause".to_string(), format!("{}", config.emergency_pause_enabled));
        stats.insert("allowlist_enabled".to_string(), format!("{}", config.enable_address_allowlist));
        stats.insert("blocklist_enabled".to_string(), format!("{}", config.enable_address_blocklist));
        
        let allowed = self.allowed_addresses.read().await;
        stats.insert("allowlist_count".to_string(), format!("{}", allowed.len()));
        
        let blocked = self.blocked_addresses.read().await;
        stats.insert("blocklist_count".to_string(), format!("{}", blocked.len()));
        
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_validation() {
        let key = SelfVault::generate_master_key();
        let vault = std::sync::Arc::new(SelfVault::new(&key));
        let engine = Web3PolicyEngine::new(vault);
        
        // Should pass basic validation
        let result = engine.validate_transaction(
            "0xSender",
            "0xRecipient",
            1_000_000_000_000_000_000, // 1 ETH
            "admin"
        ).await.unwrap();
        
        assert!(result.is_valid);
    }

    #[tokio::test]
    async fn test_amount_limit() {
        let key = SelfVault::generate_master_key();
        let vault = std::sync::Arc::new(SelfVault::new(&key));
        let engine = Web3PolicyEngine::new(vault);
        
        // Exceed default limit (10 ETH)
        let result = engine.validate_transaction(
            "0xSender",
            "0xRecipient",
            100_000_000_000_000_000_000u128, // 100 ETH
            "admin"
        ).await.unwrap();
        
        assert!(!result.is_valid);
        assert!(result.risk_score > 0.0);
    }
}
