use crate::config::schema::AppConfig;

pub fn validate(cfg: &AppConfig) {
    assert!(!cfg.security.jwt_secret.is_empty(), "JWT secret missing");
    assert!(cfg.app.port > 0, "Invalid port");
}
