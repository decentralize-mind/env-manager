use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use serde::{Deserialize, Serialize};

/// Permission levels for vault operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Permission {
    Read,
    Write,
    Delete,
    List,
    Admin,
}

/// A policy rule that grants permissions to paths
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub name: String,
    pub paths: Vec<String>,
    pub permissions: Vec<Permission>,
    pub description: String,
}

impl PolicyRule {
    /// Check if this rule grants the specified permission for a path
    pub fn grants_permission(&self, path: &str, permission: &Permission) -> bool {
        // Check if path matches (supports wildcards)
        let path_matches = self.paths.iter().any(|p| Self::path_matches(p, path));
        
        // Check if permission is granted
        let perm_granted = self.permissions.contains(permission);
        
        path_matches && perm_granted
    }

    /// Check if a path pattern matches a specific path
    fn path_matches(pattern: &str, path: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        
        if pattern.ends_with("/*") {
            let prefix = &pattern[..pattern.len() - 2];
            return path.starts_with(prefix);
        }
        
        pattern == path
    }
}

/// User role with associated policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub policies: Vec<String>, // Policy names
    pub description: String,
}

/// Access control system with RBAC (Role-Based Access Control)
#[derive(Clone)]
pub struct AccessControl {
    policies: Arc<RwLock<HashMap<String, PolicyRule>>>,
    roles: Arc<RwLock<HashMap<String, Role>>>,
    user_roles: Arc<RwLock<HashMap<String, String>>>, // username -> role name
}

impl AccessControl {
    /// Create a new access control system with default policies
    pub fn new() -> Self {
        info!("🛡️  Initializing access control system");
        
        let ac = Self {
            policies: Arc::new(RwLock::new(HashMap::new())),
            roles: Arc::new(RwLock::new(HashMap::new())),
            user_roles: Arc::new(RwLock::new(HashMap::new())),
        };

        // Initialize with default policies and roles
        let ac_clone = ac.clone();
        tokio::spawn(async move {
            ac_clone.initialize_defaults().await;
        });

        ac
    }

    /// Initialize default policies and roles
    async fn initialize_defaults(&self) {
        // Default policies
        self.add_policy(PolicyRule {
            name: "admin-full-access".to_string(),
            paths: vec!["*".to_string()],
            permissions: vec![
                Permission::Read,
                Permission::Write,
                Permission::Delete,
                Permission::List,
                Permission::Admin,
            ],
            description: "Full access to all vault paths".to_string(),
        }).await;

        self.add_policy(PolicyRule {
            name: "read-only".to_string(),
            paths: vec!["secret/*".to_string()],
            permissions: vec![Permission::Read, Permission::List],
            description: "Read-only access to secrets".to_string(),
        }).await;

        self.add_policy(PolicyRule {
            name: "app-read-write".to_string(),
            paths: vec![
                "secret/app/*".to_string(),
                "config/*".to_string(),
            ],
            permissions: vec![Permission::Read, Permission::Write, Permission::List],
            description: "Read-write access for application secrets".to_string(),
        }).await;

        // Default roles
        self.add_role(Role {
            name: "admin".to_string(),
            policies: vec!["admin-full-access".to_string()],
            description: "Administrator with full access".to_string(),
        }).await;

        self.add_role(Role {
            name: "developer".to_string(),
            policies: vec!["app-read-write".to_string()],
            description: "Developer with app-level access".to_string(),
        }).await;

        self.add_role(Role {
            name: "viewer".to_string(),
            policies: vec!["read-only".to_string()],
            description: "Viewer with read-only access".to_string(),
        }).await;

        info!("✅ Default policies and roles initialized");
    }

    /// Add a new policy
    pub async fn add_policy(&self, policy: PolicyRule) {
        let mut policies = self.policies.write().await;
        info!("📋 Adding policy: {}", policy.name);
        policies.insert(policy.name.clone(), policy);
    }

    /// Add a new role
    pub async fn add_role(&self, role: Role) {
        let mut roles = self.roles.write().await;
        info!("👥 Adding role: {}", role.name);
        roles.insert(role.name.clone(), role);
    }

    /// Assign a role to a user
    pub async fn assign_role(&self, username: &str, role_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let roles = self.roles.read().await;
        if !roles.contains_key(role_name) {
            return Err(format!("Role '{}' not found", role_name).into());
        }

        let mut user_roles = self.user_roles.write().await;
        user_roles.insert(username.to_string(), role_name.to_string());
        
        info!("✅ Assigned role '{}' to user '{}'", role_name, username);
        Ok(())
    }

    /// Check if user has read access to a path
    pub async fn check_read_access(&self, user: &str, path: &str) -> bool {
        self.check_permission(user, path, &Permission::Read).await
    }

    /// Check if user has write access to a path
    pub async fn check_write_access(&self, user: &str, path: &str) -> bool {
        self.check_permission(user, path, &Permission::Write).await
    }

    /// Check if user has delete access to a path
    pub async fn check_delete_access(&self, user: &str, path: &str) -> bool {
        self.check_permission(user, path, &Permission::Delete).await
    }

    /// Check if user has list access
    pub async fn check_list_access(&self, user: &str) -> bool {
        self.check_permission(user, "*", &Permission::List).await
    }

    /// Check if user has a specific permission for a path
    async fn check_permission(&self, user: &str, path: &str, permission: &Permission) -> bool {
        let user_roles = self.user_roles.read().await;
        
        // Get user's role
        let role_name = match user_roles.get(user) {
            Some(role) => role.clone(),
            None => {
                warn!("⚠️  User '{}' has no assigned role, denying access", user);
                return false;
            }
        };

        // Get role's policies
        let roles = self.roles.read().await;
        let role = match roles.get(&role_name) {
            Some(role) => role,
            None => {
                warn!("⚠️  Role '{}' not found for user '{}'", role_name, user);
                return false;
            }
        };

        // Check each policy
        let policies = self.policies.read().await;
        for policy_name in &role.policies {
            if let Some(policy) = policies.get(policy_name) {
                if policy.grants_permission(path, permission) {
                    return true;
                }
            }
        }

        false
    }

    /// Get all policies
    pub async fn list_policies(&self) -> Vec<PolicyRule> {
        let policies = self.policies.read().await;
        policies.values().cloned().collect()
    }

    /// Get all roles
    pub async fn list_roles(&self) -> Vec<Role> {
        let roles = self.roles.read().await;
        roles.values().cloned().collect()
    }

    /// Get user's assigned role
    pub async fn get_user_role(&self, user: &str) -> Option<String> {
        let user_roles = self.user_roles.read().await;
        user_roles.get(user).cloned()
    }

    /// Remove a user's role assignment
    pub async fn remove_user_role(&self, user: &str) {
        let mut user_roles = self.user_roles.write().await;
        user_roles.remove(user);
        info!("🗑️  Removed role assignment for user '{}'", user);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_path_matching() {
        let rule = PolicyRule {
            name: "test".to_string(),
            paths: vec!["secret/app/*".to_string()],
            permissions: vec![Permission::Read],
            description: "Test rule".to_string(),
        };

        assert!(rule.grants_permission("secret/app/config", &Permission::Read));
        assert!(rule.grants_permission("secret/app/secrets/db", &Permission::Read));
        assert!(!rule.grants_permission("secret/admin", &Permission::Read));
    }

    #[tokio::test]
    async fn test_access_control() {
        let ac = AccessControl::new();
        
        // Wait for initialization
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Assign roles
        ac.assign_role("admin_user", "admin").await.unwrap();
        ac.assign_role("dev_user", "developer").await.unwrap();
        ac.assign_role("viewer_user", "viewer").await.unwrap();

        // Test admin access
        assert!(ac.check_read_access("admin_user", "secret/anything").await);
        assert!(ac.check_write_access("admin_user", "secret/anything").await);
        assert!(ac.check_delete_access("admin_user", "secret/anything").await);

        // Test developer access
        assert!(ac.check_read_access("dev_user", "secret/app/config").await);
        assert!(ac.check_write_access("dev_user", "secret/app/config").await);
        assert!(!ac.check_delete_access("dev_user", "secret/app/config").await);

        // Test viewer access
        assert!(ac.check_read_access("viewer_user", "secret/app/config").await);
        assert!(!ac.check_write_access("viewer_user", "secret/app/config").await);
    }
}
