use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Feature flags for runtime control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    pub bridge_enabled: bool,
    pub airdrop_enabled: bool,
    pub simulation_engine_enabled: bool,
    pub debug_mode: bool,
    pub global_pause: bool,
    pub policy_engine_enabled: bool,
    pub auto_rotation_enabled: bool,
    pub metrics_enabled: bool,
    pub tracing_enabled: bool,
}

impl FeatureFlags {
    pub fn from_env() -> Self {
        Self {
            bridge_enabled: std::env::var("ENABLE_BRIDGE")
                .unwrap_or_else(|_| "true".to_string())
                .to_lowercase() == "true",
            airdrop_enabled: std::env::var("ENABLE_AIRDROP")
                .unwrap_or_else(|_| "false".to_string())
                .to_lowercase() == "true",
            simulation_engine_enabled: std::env::var("ENABLE_SIMULATION_ENGINE")
                .unwrap_or_else(|_| "true".to_string())
                .to_lowercase() == "true",
            debug_mode: std::env::var("ENABLE_DEBUG_MODE")
                .unwrap_or_else(|_| "false".to_string())
                .to_lowercase() == "true",
            global_pause: std::env::var("ENABLE_GLOBAL_PAUSE")
                .unwrap_or_else(|_| "false".to_string())
                .to_lowercase() == "true",
            policy_engine_enabled: std::env::var("POLICY_ENGINE_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .to_lowercase() == "true",
            auto_rotation_enabled: std::env::var("ENABLE_AUTO_ROTATION")
                .unwrap_or_else(|_| "true".to_string())
                .to_lowercase() == "true",
            metrics_enabled: std::env::var("METRICS_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .to_lowercase() == "true",
            tracing_enabled: std::env::var("TRACING_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .to_lowercase() == "true",
        }
    }

    pub fn check_bridge(&self) -> Result<(), String> {
        if !self.bridge_enabled {
            return Err("Bridge feature is disabled".to_string());
        }
        Ok(())
    }

    pub fn check_airdrop(&self) -> Result<(), String> {
        if !self.airdrop_enabled {
            return Err("Airdrop feature is disabled".to_string());
        }
        Ok(())
    }

    pub fn check_global_pause(&self) -> Result<(), String> {
        if self.global_pause {
            return Err("System is globally paused - emergency mode".to_string());
        }
        Ok(())
    }
}

/// Safety controls and circuit breakers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyControls {
    pub max_withdrawal_limit: f64,
    pub bridge_daily_limit: f64,
    pub anomaly_threshold: f64,
    pub policy_mode: String, // "strict" or "audit"
}

impl SafetyControls {
    pub fn from_env() -> Self {
        Self {
            max_withdrawal_limit: std::env::var("MAX_WITHDRAWAL_LIMIT")
                .unwrap_or_else(|_| "100000".to_string())
                .parse::<f64>()
                .unwrap_or(100000.0),
            bridge_daily_limit: std::env::var("BRIDGE_DAILY_LIMIT")
                .unwrap_or_else(|_| "1000000".to_string())
                .parse::<f64>()
                .unwrap_or(1000000.0),
            anomaly_threshold: std::env::var("ANOMALY_THRESHOLD")
                .unwrap_or_else(|_| "0.8".to_string())
                .parse::<f64>()
                .unwrap_or(0.8),
            policy_mode: std::env::var("POLICY_MODE")
                .unwrap_or_else(|_| "strict".to_string()),
        }
    }

    pub fn check_withdrawal_limit(&self, amount: f64) -> Result<(), String> {
        if amount > self.max_withdrawal_limit {
            return Err(format!(
                "Withdrawal amount {} exceeds limit {}",
                amount, self.max_withdrawal_limit
            ));
        }
        Ok(())
    }

    pub fn check_bridge_limit(&self, amount: f64) -> Result<(), String> {
        if amount > self.bridge_daily_limit {
            return Err(format!(
                "Bridge amount {} exceeds daily limit {}",
                amount, self.bridge_daily_limit
            ));
        }
        Ok(())
    }
}

/// Secret rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationConfig {
    pub refresh_interval_secs: u64,
    pub max_ttl_secs: u64,
}

impl RotationConfig {
    pub fn from_env() -> Self {
        Self {
            refresh_interval_secs: std::env::var("SECRET_REFRESH_INTERVAL")
                .unwrap_or_else(|_| "300".to_string())
                .parse::<u64>()
                .unwrap_or(300),
            max_ttl_secs: std::env::var("SECRET_MAX_TTL")
                .unwrap_or_else(|_| "900".to_string())
                .parse::<u64>()
                .unwrap_or(900),
        }
    }
}

/// Observability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    pub metrics_port: u16,
}

impl ObservabilityConfig {
    pub fn from_env() -> Self {
        Self {
            metrics_port: std::env::var("METRICS_PORT")
                .unwrap_or_else(|_| "9090".to_string())
                .parse::<u16>()
                .unwrap_or(9090),
        }
    }
}

/// Vault secret paths configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretPaths {
    pub jwt_path: Option<String>,
    pub session_path: Option<String>,
    pub api_path: Option<String>,
    pub db_path: Option<String>,
    pub encryption_path: Option<String>,
}

impl SecretPaths {
    pub fn from_env() -> Self {
        Self {
            jwt_path: std::env::var("SECRET_JWT_PATH").ok(),
            session_path: std::env::var("SECRET_SESSION_PATH").ok(),
            api_path: std::env::var("SECRET_API_PATH").ok(),
            db_path: std::env::var("SECRET_DB_PATH").ok(),
            encryption_path: std::env::var("SECRET_ENCRYPTION_PATH").ok(),
        }
    }

    pub fn is_configured(&self) -> bool {
        self.jwt_path.is_some() || self.db_path.is_some()
    }

    pub fn get_all_paths(&self) -> HashMap<String, String> {
        let mut paths = HashMap::new();
        if let Some(ref path) = self.jwt_path {
            paths.insert("jwt".to_string(), path.clone());
        }
        if let Some(ref path) = self.session_path {
            paths.insert("session".to_string(), path.clone());
        }
        if let Some(ref path) = self.api_path {
            paths.insert("api".to_string(), path.clone());
        }
        if let Some(ref path) = self.db_path {
            paths.insert("db".to_string(), path.clone());
        }
        if let Some(ref path) = self.encryption_path {
            paths.insert("encryption".to_string(), path.clone());
        }
        paths
    }
}

/// Complete advanced configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedConfig {
    pub feature_flags: FeatureFlags,
    pub safety_controls: SafetyControls,
    pub rotation_config: RotationConfig,
    pub observability: ObservabilityConfig,
    pub secret_paths: SecretPaths,
}

impl AdvancedConfig {
    pub fn from_env() -> Self {
        Self {
            feature_flags: FeatureFlags::from_env(),
            safety_controls: SafetyControls::from_env(),
            rotation_config: RotationConfig::from_env(),
            observability: ObservabilityConfig::from_env(),
            secret_paths: SecretPaths::from_env(),
        }
    }

    /// Check if system should allow operations
    pub fn check_system_health(&self) -> Result<(), Vec<String>> {
        let mut errors = vec![];

        // Check global pause
        if let Err(e) = self.feature_flags.check_global_pause() {
            errors.push(e);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_flags_from_env() {
        std::env::set_var("ENABLE_BRIDGE", "true");
        std::env::set_var("ENABLE_GLOBAL_PAUSE", "false");
        
        let flags = FeatureFlags::from_env();
        assert!(flags.bridge_enabled);
        assert!(!flags.global_pause);
        
        // Cleanup
        std::env::remove_var("ENABLE_BRIDGE");
        std::env::remove_var("ENABLE_GLOBAL_PAUSE");
    }

    #[test]
    fn test_safety_controls_from_env() {
        std::env::set_var("MAX_WITHDRAWAL_LIMIT", "50000");
        
        let controls = SafetyControls::from_env();
        assert_eq!(controls.max_withdrawal_limit, 50000.0);
        
        // Cleanup
        std::env::remove_var("MAX_WITHDRAWAL_LIMIT");
    }

    #[test]
    fn test_secret_paths_from_env() {
        std::env::set_var("SECRET_JWT_PATH", "secret/data/test/jwt");
        
        let paths = SecretPaths::from_env();
        assert_eq!(paths.jwt_path, Some("secret/data/test/jwt".to_string()));
        assert!(paths.is_configured());
        
        // Cleanup
        std::env::remove_var("SECRET_JWT_PATH");
    }

    #[test]
    fn test_withdrawal_limit_check() {
        let controls = SafetyControls {
            max_withdrawal_limit: 100000.0,
            bridge_daily_limit: 1000000.0,
            anomaly_threshold: 0.8,
            policy_mode: "strict".to_string(),
        };

        assert!(controls.check_withdrawal_limit(50000.0).is_ok());
        assert!(controls.check_withdrawal_limit(150000.0).is_err());
    }
}
