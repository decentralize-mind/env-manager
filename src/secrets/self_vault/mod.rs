/// Self-contained Vault implementation with enterprise-grade security features
/// 
/// This module provides a complete secrets management system that runs within your application,
/// offering the same capabilities as HashiCorp Vault but without external dependencies.
/// 
/// Features:
/// - Centralized secure storage with AES-256-GCM encryption
/// - Dynamic credentials with automatic expiry
/// - Comprehensive audit trail with tamper-proof logging
/// - Automatic secret rotation without downtime
/// - Fine-grained access control policies
/// - Production-grade security for financial systems

pub mod self_vault;
pub mod dynamic_credentials;
pub mod audit_trail;
pub mod secret_rotation;
pub mod access_control;
pub mod security_controls;

pub use self_vault::SelfVault;
pub use dynamic_credentials::DynamicCredentialsManager;
pub use audit_trail::AuditTrail;
pub use secret_rotation::SecretRotator;
pub use access_control::AccessControl;
pub use security_controls::SecurityControls;
