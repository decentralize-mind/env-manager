use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn, error};
use chrono::{DateTime, Utc};

/// Transaction risk levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn score(&self) -> u32 {
        match self {
            RiskLevel::Low => 0,
            RiskLevel::Medium => 25,
            RiskLevel::High => 50,
            RiskLevel::Critical => 100,
        }
    }
}

/// Transaction structure for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub from: String,
    pub to: String,
    pub amount: f64,
    pub data: Option<String>,
    pub gas_limit: Option<u64>,
    pub nonce: Option<u64>,
    pub timestamp: DateTime<Utc>,
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub risk_level: RiskLevel,
    pub risk_score: u32,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub requires_approval: bool,
}

impl ValidationResult {
    pub fn passed() -> Self {
        Self {
            is_valid: true,
            risk_level: RiskLevel::Low,
            risk_score: 0,
            warnings: vec![],
            errors: vec![],
            requires_approval: false,
        }
    }

    pub fn failed(errors: Vec<String>) -> Self {
        Self {
            is_valid: false,
            risk_level: RiskLevel::Critical,
            risk_score: 100,
            warnings: vec![],
            errors,
            requires_approval: false,
        }
    }
}

/// Transaction validator with multiple checks
pub struct TransactionValidator {
    allowed_contracts: Vec<String>,
    blocked_addresses: Vec<String>,
    max_amount: f64,
    daily_limit: f64,
    current_daily_total: f64,
}

impl TransactionValidator {
    pub fn new(max_amount: f64, daily_limit: f64) -> Self {
        Self {
            allowed_contracts: vec![],
            blocked_addresses: vec![],
            max_amount,
            daily_limit,
            current_daily_total: 0.0,
        }
    }

    /// Add allowed contract addresses
    pub fn add_allowed_contract(&mut self, address: &str) {
        self.allowed_contracts.push(address.to_lowercase());
    }

    /// Add blocked/blacklisted addresses
    pub fn add_blocked_address(&mut self, address: &str) {
        self.blocked_addresses.push(address.to_lowercase());
    }

    /// Validate transaction before signing
    pub fn validate(&self, tx: &Transaction) -> ValidationResult {
        info!("🔍 Validating transaction: {} → {}", tx.from, tx.to);

        let mut warnings = vec![];
        let mut errors = vec![];
        let mut risk_score = 0u32;

        // Check 1: Validate addresses format
        if !self.is_valid_address(&tx.from) {
            errors.push("Invalid 'from' address format".to_string());
        }

        if !self.is_valid_address(&tx.to) {
            errors.push("Invalid 'to' address format".to_string());
        }

        // Check 2: Blocked addresses
        if self.is_blocked_address(&tx.to) {
            errors.push(format!("Destination address {} is blocked", tx.to));
            risk_score += 100;
        }

        // Check 3: Amount validation
        if tx.amount <= 0.0 {
            errors.push("Transaction amount must be positive".to_string());
        }

        if tx.amount > self.max_amount {
            errors.push(format!(
                "Amount {} exceeds maximum allowed {}",
                tx.amount, self.max_amount
            ));
            risk_score += 50;
        }

        // Check 4: Daily limit
        if self.current_daily_total + tx.amount > self.daily_limit {
            errors.push("Daily transaction limit exceeded".to_string());
            risk_score += 75;
        }

        // Check 5: Contract interaction validation
        if let Some(ref data) = tx.data {
            if !data.is_empty() && !self.allowed_contracts.is_empty() {
                if !self.allowed_contracts.contains(&tx.to.to_lowercase()) {
                    warnings.push(format!(
                        "Contract {} not in allowed list",
                        tx.to
                    ));
                    risk_score += 25;
                }
            }
        }

        // Check 6: Unusual patterns
        if tx.amount > self.max_amount * 0.8 {
            warnings.push("Large transaction amount (>80% of max)".to_string());
            risk_score += 15;
        }

        // Determine risk level
        let risk_level = if risk_score >= 100 {
            RiskLevel::Critical
        } else if risk_score >= 50 {
            RiskLevel::High
        } else if risk_score >= 25 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        let requires_approval = risk_score >= 50;
        let is_valid = errors.is_empty();

        let result = ValidationResult {
            is_valid,
            risk_level,
            risk_score,
            warnings,
            errors,
            requires_approval,
        };

        if result.is_valid {
            info!(
                "✅ Transaction validated (Risk: {:?}, Score: {})",
                result.risk_level, result.risk_score
            );
        } else {
            error!(
                "❌ Transaction validation failed: {:?}",
                result.errors
            );
        }

        result
    }

    /// Simulate transaction execution (placeholder for real simulation)
    pub async fn simulate(&self, tx: &Transaction) -> Result<bool, Box<dyn std::error::Error>> {
        info!("🧪 Simulating transaction execution...");
        
        // In production: Use Tenderly, Hardhat, or Anvil for simulation
        // This would check if the transaction would succeed on-chain
        
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Placeholder - always succeeds for now
        Ok(true)
    }

    /// Full validation pipeline: validate + simulate
    pub async fn validate_and_simulate(
        &self,
        tx: &Transaction,
    ) -> Result<ValidationResult, Box<dyn std::error::Error>> {
        // Step 1: Static validation
        let validation = self.validate(tx);
        
        if !validation.is_valid {
            return Ok(validation);
        }

        // Step 2: Simulation
        let simulation_ok = self.simulate(tx).await?;
        
        if !simulation_ok {
            warn!("⚠️  Transaction simulation failed");
            return Ok(ValidationResult::failed(vec![
                "Transaction simulation failed".to_string(),
            ]));
        }

        Ok(validation)
    }

    // Helper methods
    fn is_valid_address(&self, address: &str) -> bool {
        // Basic Ethereum address validation (0x + 40 hex chars)
        address.starts_with("0x") && address.len() == 42
    }

    fn is_blocked_address(&self, address: &str) -> bool {
        self.blocked_addresses.contains(&address.to_lowercase())
    }
}

/// Transaction approval workflow
pub struct ApprovalWorkflow {
    approvers: Vec<String>,
    required_approvals: usize,
}

impl ApprovalWorkflow {
    pub fn new(approvers: Vec<String>, required_approvals: usize) -> Self {
        Self {
            approvers,
            required_approvals,
        }
    }

    pub fn requires_approval(&self, validation: &ValidationResult) -> bool {
        validation.requires_approval
    }

    pub async fn request_approval(
        &self,
        tx: &Transaction,
        validation: &ValidationResult,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        info!(
            "📋 Requesting approval for high-risk transaction (Risk: {:?})",
            validation.risk_level
        );
        info!("   Approvers: {:?}", self.approvers);
        info!("   Required approvals: {}", self.required_approvals);

        // In production: Send notifications to approvers via Slack/Email/PagerDuty
        // Wait for approvals with timeout
        
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Placeholder - auto-approve for demo
        info!("✅ Transaction approved (demo mode)");
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transaction() {
        let validator = TransactionValidator::new(100.0, 1000.0);
        let tx = Transaction {
            from: "0x1234567890123456789012345678901234567890".to_string(),
            to: "0x0987654321098765432109876543210987654321".to_string(),
            amount: 10.0,
            data: None,
            gas_limit: Some(21000),
            nonce: Some(1),
            timestamp: Utc::now(),
        };

        let result = validator.validate(&tx);
        assert!(result.is_valid);
        assert_eq!(result.risk_level, RiskLevel::Low);
    }

    #[test]
    fn test_blocked_address() {
        let mut validator = TransactionValidator::new(100.0, 1000.0);
        validator.add_blocked_address("0xBADADDRESS1234567890123456789012345678");
        
        let tx = Transaction {
            from: "0x1234567890123456789012345678901234567890".to_string(),
            to: "0xBADADDRESS1234567890123456789012345678".to_string(),
            amount: 10.0,
            data: None,
            gas_limit: None,
            nonce: None,
            timestamp: Utc::now(),
        };

        let result = validator.validate(&tx);
        assert!(!result.is_valid);
        assert_eq!(result.risk_level, RiskLevel::Critical);
    }

    #[test]
    fn test_exceeds_max_amount() {
        let validator = TransactionValidator::new(100.0, 1000.0);
        let tx = Transaction {
            from: "0x1234567890123456789012345678901234567890".to_string(),
            to: "0x0987654321098765432109876543210987654321".to_string(),
            amount: 150.0, // Exceeds max of 100
            data: None,
            gas_limit: None,
            nonce: None,
            timestamp: Utc::now(),
        };

        let result = validator.validate(&tx);
        assert!(!result.is_valid);
        assert!(result.risk_score >= 50);
    }
}
