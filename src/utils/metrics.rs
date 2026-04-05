use tracing::info;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Simple metrics collector for Prometheus-compatible output
pub struct MetricsCollector {
    secret_fetches: AtomicU64,
    secret_rotations: AtomicU64,
    policy_violations: AtomicU64,
    transaction_validations: AtomicU64,
    errors: AtomicU64,
    start_time: std::time::Instant,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            secret_fetches: AtomicU64::new(0),
            secret_rotations: AtomicU64::new(0),
            policy_violations: AtomicU64::new(0),
            transaction_validations: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            start_time: std::time::Instant::now(),
        }
    }

    pub fn increment_secret_fetches(&self) {
        self.secret_fetches.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_secret_rotations(&self) {
        self.secret_rotations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_policy_violations(&self) {
        self.policy_violations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_transaction_validations(&self) {
        self.transaction_validations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_errors(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Generate Prometheus-format metrics
    pub fn generate_metrics(&self) -> String {
        let uptime_secs = self.start_time.elapsed().as_secs();
        
        format!(
            "# HELP env_manager_secret_fetches_total Total number of secret fetches from Vault\n\
             # TYPE env_manager_secret_fetches_total counter\n\
             env_manager_secret_fetches_total {}\n\n\
             # HELP env_manager_secret_rotations_total Total number of secret rotations\n\
             # TYPE env_manager_secret_rotations_total counter\n\
             env_manager_secret_rotations_total {}\n\n\
             # HELP env_manager_policy_violations_total Total policy violations detected\n\
             # TYPE env_manager_policy_violations_total counter\n\
             env_manager_policy_violations_total {}\n\n\
             # HELP env_manager_transaction_validations_total Total transaction validations\n\
             # TYPE env_manager_transaction_validations_total counter\n\
             env_manager_transaction_validations_total {}\n\n\
             # HELP env_manager_errors_total Total errors encountered\n\
             # TYPE env_manager_errors_total counter\n\
             env_manager_errors_total {}\n\n\
             # HELP env_manager_uptime_seconds Application uptime in seconds\n\
             # TYPE env_manager_uptime_seconds gauge\n\
             env_manager_uptime_seconds {}\n",
            self.secret_fetches.load(Ordering::Relaxed),
            self.secret_rotations.load(Ordering::Relaxed),
            self.policy_violations.load(Ordering::Relaxed),
            self.transaction_validations.load(Ordering::Relaxed),
            self.errors.load(Ordering::Relaxed),
            uptime_secs
        )
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Global metrics instance (lazy initialization)
static METRICS: std::sync::OnceLock<Arc<MetricsCollector>> = std::sync::OnceLock::new();

pub fn get_metrics() -> Arc<MetricsCollector> {
    METRICS.get_or_init(|| Arc::new(MetricsCollector::new())).clone()
}

/// Start a simple HTTP metrics server (optional, for production use proper server)
pub async fn start_metrics_server(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    info!("📊 Starting metrics server on port {}", port);
    
    // For now, just log that metrics are available
    // In production, use a proper HTTP server like axum or actix-web
    info!("   Metrics endpoint would be: http://localhost:{}/metrics", port);
    info!("   ℹ️  Full HTTP server implementation requires additional dependencies");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_generation() {
        let collector = MetricsCollector::new();
        collector.increment_secret_fetches();
        collector.increment_errors();
        
        let metrics = collector.generate_metrics();
        assert!(metrics.contains("env_manager_secret_fetches_total 1"));
        assert!(metrics.contains("env_manager_errors_total 1"));
        assert!(metrics.contains("env_manager_uptime_seconds"));
    }

    #[test]
    fn test_global_metrics() {
        let metrics = get_metrics();
        metrics.increment_transaction_validations();
        
        let metrics2 = get_metrics();
        assert_eq!(
            metrics2.transaction_validations.load(Ordering::Relaxed),
            1
        );
    }
}
