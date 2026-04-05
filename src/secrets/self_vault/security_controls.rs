use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use serde::{Deserialize, Serialize};

/// Security configuration for the vault
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub max_failed_attempts: u32,
    pub lockout_duration_seconds: u64,
    pub session_timeout_seconds: u64,
    pub enable_ip_whitelist: bool,
    pub allowed_ips: Vec<String>,
    pub require_mfa: bool,
    pub audit_all_operations: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_failed_attempts: 5,
            lockout_duration_seconds: 900, // 15 minutes
            session_timeout_seconds: 3600, // 1 hour
            enable_ip_whitelist: false,
            allowed_ips: vec![],
            require_mfa: false,
            audit_all_operations: true,
        }
    }
}

/// Track failed login attempts
#[derive(Debug, Clone)]
struct FailedAttemptTracker {
    count: u32,
    first_attempt: Instant,
    last_attempt: Instant,
}

/// Production-grade security controls
pub struct SecurityControls {
    config: RwLock<SecurityConfig>,
    failed_attempts: RwLock<std::collections::HashMap<String, FailedAttemptTracker>>,
    locked_users: RwLock<std::collections::HashMap<String, Instant>>,
    active_sessions: RwLock<std::collections::HashMap<String, Instant>>,
}

impl SecurityControls {
    /// Create new security controls with default configuration
    pub fn new() -> Self {
        info!("🛡️  Initializing production-grade security controls");
        
        Self {
            config: RwLock::new(SecurityConfig::default()),
            failed_attempts: RwLock::new(std::collections::HashMap::new()),
            locked_users: RwLock::new(std::collections::HashMap::new()),
            active_sessions: RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Update security configuration
    pub async fn update_config(&self, config: SecurityConfig) {
        let mut current = self.config.write().await;
        *current = config;
        info!("✅ Security configuration updated");
    }

    /// Get current security configuration
    pub async fn get_config(&self) -> SecurityConfig {
        let config = self.config.read().await;
        config.clone()
    }

    /// Record a failed access attempt
    pub async fn record_failed_attempt(&self, user: &str) {
        let config = self.config.read().await;
        let mut attempts = self.failed_attempts.write().await;
        
        let tracker = attempts.entry(user.to_string()).or_insert(FailedAttemptTracker {
            count: 0,
            first_attempt: Instant::now(),
            last_attempt: Instant::now(),
        });
        
        tracker.count += 1;
        tracker.last_attempt = Instant::now();
        
        warn!(
            "⚠️  Failed attempt {} for user '{}' (max: {})",
            tracker.count, user, config.max_failed_attempts
        );
        
        // Check if should lock out
        if tracker.count >= config.max_failed_attempts {
            drop(attempts);
            self.lock_user(user, config.lockout_duration_seconds).await;
        }
    }

    /// Lock a user account temporarily
    async fn lock_user(&self, user: &str, duration_seconds: u64) {
        let lockout_until = Instant::now() + Duration::from_secs(duration_seconds);
        let mut locked = self.locked_users.write().await;
        locked.insert(user.to_string(), lockout_until);
        
        error!("🔒 User '{}' locked out for {} seconds due to excessive failed attempts", 
               user, duration_seconds);
    }

    /// Check if a user is currently locked out
    pub async fn is_locked_out(&self, user: &str) -> bool {
        let mut locked = self.locked_users.write().await;
        
        if let Some(lockout_until) = locked.get(user) {
            if Instant::now() < *lockout_until {
                return true;
            } else {
                // Lockout expired, remove it
                locked.remove(user);
                
                // Reset failed attempts
                let mut attempts = self.failed_attempts.write().await;
                attempts.remove(user);
                
                info!("🔓 Lockout expired for user '{}'", user);
                return false;
            }
        }
        
        false
    }

    /// Record a successful login and start session
    pub async fn start_session(&self, user: &str) {
        let config = self.config.read().await;
        let timeout = Instant::now() + Duration::from_secs(config.session_timeout_seconds);
        
        let mut sessions = self.active_sessions.write().await;
        sessions.insert(user.to_string(), timeout);
        
        // Clear failed attempts on successful login
        let mut attempts = self.failed_attempts.write().await;
        attempts.remove(user);
        
        info!("✅ Session started for user '{}'", user);
    }

    /// Check if user's session is still valid
    pub async fn is_session_valid(&self, user: &str) -> bool {
        let mut sessions = self.active_sessions.write().await;
        
        if let Some(timeout) = sessions.get(user) {
            if Instant::now() < *timeout {
                return true;
            } else {
                // Session expired
                sessions.remove(user);
                info!("⏰ Session expired for user '{}'", user);
                return false;
            }
        }
        
        false
    }

    /// End a user session
    pub async fn end_session(&self, user: &str) {
        let mut sessions = self.active_sessions.write().await;
        sessions.remove(user);
        info!("🚪 Session ended for user '{}'", user);
    }

    /// Validate IP address against whitelist (if enabled)
    pub async fn validate_ip(&self, ip: &str) -> bool {
        let config = self.config.read().await;
        
        if !config.enable_ip_whitelist {
            return true; // Whitelist disabled, allow all
        }
        
        config.allowed_ips.contains(&ip.to_string())
    }

    /// Add IP to whitelist
    pub async fn add_allowed_ip(&self, ip: &str) {
        let mut config = self.config.write().await;
        if !config.allowed_ips.contains(&ip.to_string()) {
            config.allowed_ips.push(ip.to_string());
            info!("✅ Added IP to whitelist: {}", ip);
        }
    }

    /// Remove IP from whitelist
    pub async fn remove_allowed_ip(&self, ip: &str) {
        let mut config = self.config.write().await;
        config.allowed_ips.retain(|allowed_ip| allowed_ip != ip);
        info!("🗑️  Removed IP from whitelist: {}", ip);
    }

    /// Get security statistics
    pub async fn get_security_stats(&self) -> SecurityStats {
        let locked = self.locked_users.read().await;
        let sessions = self.active_sessions.read().await;
        let attempts = self.failed_attempts.read().await;
        
        SecurityStats {
            locked_users: locked.len(),
            active_sessions: sessions.len(),
            users_with_failed_attempts: attempts.len(),
        }
    }

    /// Emergency lockdown - lock all users
    pub async fn emergency_lockdown(&self) {
        warn!("🚨 EMERGENCY LOCKDOWN INITIATED");
        
        let config = self.config.read().await;
        let mut locked = self.locked_users.write().await;
        
        // Lock all users with extended duration
        let lockout_until = Instant::now() + Duration::from_secs(3600); // 1 hour
        
        for user in self.active_sessions.read().await.keys() {
            locked.insert(user.clone(), lockout_until);
        }
        
        // Clear all sessions
        self.active_sessions.write().await.clear();
        
        error!("🚨 All users locked down for 1 hour");
    }

    /// Clear all security state (emergency use only)
    pub async fn clear_all(&self) {
        self.failed_attempts.write().await.clear();
        self.locked_users.write().await.clear();
        self.active_sessions.write().await.clear();
        warn!("🗑️  All security state cleared");
    }
}

/// Security statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityStats {
    pub locked_users: usize,
    pub active_sessions: usize,
    pub users_with_failed_attempts: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_failed_attempts_lockout() {
        let controls = SecurityControls::new();
        
        // Configure low threshold for testing
        let config = SecurityConfig {
            max_failed_attempts: 3,
            lockout_duration_seconds: 60,
            ..Default::default()
        };
        controls.update_config(config).await;
        
        // Record failed attempts
        for _ in 0..3 {
            controls.record_failed_attempt("test_user").await;
        }
        
        // Should be locked out
        assert!(controls.is_locked_out("test_user").await);
    }

    #[tokio::test]
    async fn test_session_management() {
        let controls = SecurityControls::new();
        
        // Start session
        controls.start_session("user1").await;
        assert!(controls.is_session_valid("user1").await);
        
        // End session
        controls.end_session("user1").await;
        assert!(!controls.is_session_valid("user1").await);
    }

    #[tokio::test]
    async fn test_ip_whitelist() {
        let controls = SecurityControls::new();
        
        // Without whitelist, all IPs allowed
        assert!(controls.validate_ip("192.168.1.1").await);
        
        // Enable whitelist
        let mut config = controls.get_config().await;
        config.enable_ip_whitelist = true;
        config.allowed_ips = vec!["10.0.0.1".to_string()];
        controls.update_config(config).await;
        
        // Only whitelisted IP allowed
        assert!(controls.validate_ip("10.0.0.1").await);
        assert!(!controls.validate_ip("192.168.1.1").await);
    }
}
