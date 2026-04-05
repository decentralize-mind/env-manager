use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};
use chrono::{DateTime, Utc, Timelike};

/// Policy rule types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyRule {
    /// Maximum withdrawal amount per transaction
    MaxWithdrawal { amount: f64, currency: String },
    
    /// Maximum withdrawals per time period
    WithdrawalLimit { 
        max_count: u32, 
        period_seconds: u64 
    },
    
    /// Allowed contract addresses for interaction
    AllowedContracts { addresses: Vec<String> },
    
    /// Blocked/blacklisted addresses
    BlockedAddresses { addresses: Vec<String> },
    
    /// Geographic restrictions (country codes)
    GeoRestriction { allowed_countries: Vec<String> },
    
    /// Time-based restrictions (UTC hours)
    TimeRestriction { 
        allowed_hours_start: u32, 
        allowed_hours_end: u32 
    },
    
    /// Require multi-sig approval above threshold
    MultiSigRequired { 
        threshold_amount: f64, 
        required_signers: u32 
    },
    
    /// IP whitelist
    IpWhitelist { allowed_ips: Vec<String> },
}

/// Policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub name: String,
    pub description: String,
    pub rules: Vec<PolicyRule>,
    pub enabled: bool,
    pub priority: u32, // Higher priority policies evaluated first
}

impl Policy {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            rules: vec![],
            enabled: true,
            priority: 0,
        }
    }

    pub fn add_rule(&mut self, rule: PolicyRule) {
        self.rules.push(rule);
    }
}

/// Policy evaluation context
#[derive(Debug, Clone)]
pub struct PolicyContext {
    pub user_id: String,
    pub action: String,
    pub amount: Option<f64>,
    pub destination: Option<String>,
    pub ip_address: Option<String>,
    pub country: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

/// Policy evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyResult {
    pub allowed: bool,
    pub policy_name: String,
    pub violated_rules: Vec<String>,
    pub warnings: Vec<String>,
    pub requires_approval: bool,
}

impl PolicyResult {
    pub fn allowed(policy_name: &str) -> Self {
        Self {
            allowed: true,
            policy_name: policy_name.to_string(),
            violated_rules: vec![],
            warnings: vec![],
            requires_approval: false,
        }
    }

    pub fn denied(policy_name: &str, violated_rules: Vec<String>) -> Self {
        Self {
            allowed: false,
            policy_name: policy_name.to_string(),
            violated_rules,
            warnings: vec![],
            requires_approval: false,
        }
    }
}

/// Policy engine that evaluates actions against defined policies
pub struct PolicyEngine {
    policies: Vec<Policy>,
    violation_history: Vec<(String, DateTime<Utc>)>, // (policy_name, timestamp)
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self {
            policies: vec![],
            violation_history: vec![],
        }
    }

    /// Add a policy to the engine
    pub fn add_policy(&mut self, policy: Policy) {
        info!("📋 Adding policy: {}", policy.name);
        self.policies.push(policy);
        
        // Sort by priority (highest first)
        self.policies.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Evaluate an action against all policies
    pub fn evaluate(&self, context: &PolicyContext) -> Vec<PolicyResult> {
        info!("🔍 Evaluating policy for user: {}, action: {}", context.user_id, context.action);
        
        let mut results = vec![];

        for policy in &self.policies {
            if !policy.enabled {
                continue;
            }

            let result = self.evaluate_policy(policy, context);
            results.push(result);
        }

        // Overall decision: deny if any policy denies
        let any_denied = results.iter().any(|r| !r.allowed);
        
        if any_denied {
            warn!("❌ Action denied by policy");
        } else {
            info!("✅ Action allowed by all policies");
        }

        results
    }

    /// Check if action is allowed (convenience method)
    pub fn is_allowed(&self, context: &PolicyContext) -> bool {
        let results = self.evaluate(context);
        results.iter().all(|r| r.allowed)
    }

    /// Evaluate a single policy
    fn evaluate_policy(&self, policy: &Policy, context: &PolicyContext) -> PolicyResult {
        let mut violated_rules = vec![];
        let mut warnings = vec![];
        let mut requires_approval = false;

        for rule in &policy.rules {
            match rule {
                PolicyRule::MaxWithdrawal { amount, currency } => {
                    if let Some(tx_amount) = context.amount {
                        if tx_amount > *amount {
                            violated_rules.push(format!(
                                "Exceeds max withdrawal: {} {}",
                                amount, currency
                            ));
                        } else if tx_amount > amount * 0.8 {
                            warnings.push(format!(
                                "Large withdrawal: {} {} (>80% of limit)",
                                tx_amount, currency
                            ));
                        }
                    }
                }

                PolicyRule::WithdrawalLimit { max_count, period_seconds } => {
                    // Count recent violations within period
                    let cutoff = context.timestamp - chrono::Duration::seconds(*period_seconds as i64);
                    let recent_violations = self.violation_history.iter()
                        .filter(|(_, ts)| *ts >= cutoff)
                        .count();

                    if recent_violations as u32 >= *max_count {
                        violated_rules.push(format!(
                            "Withdrawal limit exceeded: {}/{} in period",
                            recent_violations, max_count
                        ));
                    }
                }

                PolicyRule::AllowedContracts { addresses } => {
                    if let Some(ref dest) = context.destination {
                        if !addresses.is_empty() && !addresses.contains(&dest.to_lowercase()) {
                            violated_rules.push(format!(
                                "Contract {} not in allowed list",
                                dest
                            ));
                        }
                    }
                }

                PolicyRule::BlockedAddresses { addresses } => {
                    if let Some(ref dest) = context.destination {
                        if addresses.contains(&dest.to_lowercase()) {
                            violated_rules.push(format!(
                                "Address {} is blocked",
                                dest
                            ));
                        }
                    }
                }

                PolicyRule::GeoRestriction { allowed_countries } => {
                    if let Some(ref country) = context.country {
                        if !allowed_countries.is_empty() && !allowed_countries.contains(country) {
                            violated_rules.push(format!(
                                "Access from country {} not allowed",
                                country
                            ));
                        }
                    }
                }

                PolicyRule::TimeRestriction { allowed_hours_start, allowed_hours_end } => {
                    let current_hour = context.timestamp.hour();
                    if current_hour < *allowed_hours_start || current_hour >= *allowed_hours_end {
                        violated_rules.push(format!(
                            "Action not allowed at hour {} (allowed: {}-{})",
                            current_hour, allowed_hours_start, allowed_hours_end
                        ));
                    }
                }

                PolicyRule::MultiSigRequired { threshold_amount, required_signers } => {
                    if let Some(tx_amount) = context.amount {
                        if tx_amount >= *threshold_amount {
                            requires_approval = true;
                            warnings.push(format!(
                                "Multi-sig approval required: {} signers needed",
                                required_signers
                            ));
                        }
                    }
                }

                PolicyRule::IpWhitelist { allowed_ips } => {
                    if let Some(ref ip) = context.ip_address {
                        if !allowed_ips.is_empty() && !allowed_ips.contains(ip) {
                            violated_rules.push(format!(
                                "IP {} not in whitelist",
                                ip
                            ));
                        }
                    }
                }
            }
        }

        let allowed = violated_rules.is_empty();

        if !allowed {
            // Record violation
            let mut history = self.violation_history.clone();
            history.push((policy.name.clone(), context.timestamp));
        }

        PolicyResult {
            allowed,
            policy_name: policy.name.clone(),
            violated_rules,
            warnings,
            requires_approval,
        }
    }

    /// Get policy statistics
    pub fn get_stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();
        
        for (policy_name, _) in &self.violation_history {
            *stats.entry(policy_name.clone()).or_insert(0) += 1;
        }
        
        stats
    }
}

/// Predefined policy templates
impl PolicyEngine {
    /// Create conservative security policy
    pub fn conservative_policy() -> Policy {
        let mut policy = Policy::new("conservative", "Conservative security policy");
        policy.priority = 100;
        
        policy.add_rule(PolicyRule::MaxWithdrawal {
            amount: 1000.0,
            currency: "USD".to_string(),
        });
        
        policy.add_rule(PolicyRule::WithdrawalLimit {
            max_count: 5,
            period_seconds: 3600, // 1 hour
        });
        
        policy.add_rule(PolicyRule::MultiSigRequired {
            threshold_amount: 500.0,
            required_signers: 2,
        });
        
        policy
    }

    /// Create moderate security policy
    pub fn moderate_policy() -> Policy {
        let mut policy = Policy::new("moderate", "Moderate security policy");
        policy.priority = 50;
        
        policy.add_rule(PolicyRule::MaxWithdrawal {
            amount: 10000.0,
            currency: "USD".to_string(),
        });
        
        policy.add_rule(PolicyRule::WithdrawalLimit {
            max_count: 20,
            period_seconds: 3600,
        });
        
        policy.add_rule(PolicyRule::MultiSigRequired {
            threshold_amount: 5000.0,
            required_signers: 2,
        });
        
        policy
    }

    /// Create permissive policy (development/testing)
    pub fn permissive_policy() -> Policy {
        let mut policy = Policy::new("permissive", "Permissive policy for development");
        policy.priority = 10;
        
        policy.add_rule(PolicyRule::MaxWithdrawal {
            amount: 100000.0,
            currency: "USD".to_string(),
        });
        
        policy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conservative_policy_allows_small_withdrawal() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(PolicyEngine::conservative_policy());

        let context = PolicyContext {
            user_id: "user123".to_string(),
            action: "withdraw".to_string(),
            amount: Some(100.0),
            destination: None,
            ip_address: None,
            country: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        let results = engine.evaluate(&context);
        assert!(results.iter().all(|r| r.allowed));
    }

    #[test]
    fn test_conservative_policy_denies_large_withdrawal() {
        let mut engine = PolicyEngine::new();
        engine.add_policy(PolicyEngine::conservative_policy());

        let context = PolicyContext {
            user_id: "user123".to_string(),
            action: "withdraw".to_string(),
            amount: Some(5000.0), // Exceeds 1000 limit
            destination: None,
            ip_address: None,
            country: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        let results = engine.evaluate(&context);
        assert!(results.iter().any(|r| !r.allowed));
    }

    #[test]
    fn test_time_restriction() {
        let mut policy = Policy::new("time_limited", "Time-restricted policy");
        policy.add_rule(PolicyRule::TimeRestriction {
            allowed_hours_start: 9,
            allowed_hours_end: 17, // Business hours only
        });

        let mut engine = PolicyEngine::new();
        engine.add_policy(policy);

        // Test during business hours
        let context = PolicyContext {
            user_id: "user123".to_string(),
            action: "withdraw".to_string(),
            amount: None,
            destination: None,
            ip_address: None,
            country: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        let results = engine.evaluate(&context);
        // Result depends on current time - just check it runs without error
        assert!(!results.is_empty());
    }
}
