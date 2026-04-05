use tracing::info;

pub fn log_access(user: &str, action: &str) {
    info!("AUDIT: {} performed {}", user, action);
}
