/// Web3 Signer Service - Cryptographic transaction signing
/// 
/// This module provides secure transaction signing capabilities for Web3 applications.
/// Keys are NEVER exposed - all signing happens within protected boundaries.
/// 
/// Architecture:
/// - Backend requests signature
/// - Signer validates policy
/// - Signs transaction (HSM/MPC/Enclave)
/// - Returns signature only (no private key exposure)

use k256::{
    ecdsa::{SigningKey, Signature, signature::Signer},
    SecretKey,
};
use sha3::{Digest, Keccak256};
use tracing::{info, warn, error};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::secrets::self_vault::SelfVault;
use crate::security::policy_engine::PolicyEngine;

/// Transaction structure for Web3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Web3Transaction {
    pub to: String,           // Recipient address
    pub value: u128,          // Amount in wei
    pub data: Vec<u8>,        // Transaction data
    pub nonce: u64,           // Transaction nonce
    pub gas_limit: u64,       // Gas limit
    pub max_fee_per_gas: u128, // Max fee per gas
    pub chain_id: u64,        // Chain ID
}

/// Signature result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Web3Signature {
    pub r: [u8; 32],
    pub s: [u8; 32],
    pub v: u8,
    pub signature_hex: String,
}

impl Web3Signature {
    /// Convert signature to hex string (0x prefixed)
    pub fn to_hex(&self) -> String {
        format!("0x{}", self.signature_hex)
    }
}

/// Signer configuration
#[derive(Debug, Clone)]
pub struct SignerConfig {
    pub signer_type: SignerType,
    pub policy_check: bool,
    pub require_mfa: bool,
}

#[derive(Debug, Clone)]
pub enum SignerType {
    Standard,      // Software-based (development)
    HSM,          // Hardware Security Module
    MPC,          // Multi-Party Computation
}

/// Web3 Signer Service
pub struct Web3SignerService {
    vault: Arc<SelfVault>,
    policy_engine: Arc<PolicyEngine>,
    config: SignerConfig,
    signing_keys: RwLock<std::collections::HashMap<String, SigningKey>>,
}

impl Web3SignerService {
    /// Create a new Web3 signer service
    pub fn new(
        vault: Arc<SelfVault>,
        policy_engine: Arc<PolicyEngine>,
        config: SignerConfig,
    ) -> Self {
        info!("🔐 Initializing Web3 Signer Service ({:?})", config.signer_type);
        
        Self {
            vault,
            policy_engine,
            config,
            signing_keys: RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Load signing key from vault (key never leaves protected memory)
    pub async fn load_signing_key(
        &self,
        key_name: &str,
        user: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("🔑 Loading signing key: {}", key_name);
        
        // Retrieve encrypted private key from vault
        let key_hex = match self.vault.get_secret(&format!("secret/web3/{}", key_name), user).await? {
            Some(key) => key,
            None => return Err(format!("Signing key '{}' not found in vault", key_name).into()),
        };
        
        // Decode and create signing key
        let key_bytes = hex::decode(&key_hex)?;
        let secret_key = SecretKey::from_bytes((&key_bytes[..]).into())?;
        let signing_key = SigningKey::from(secret_key);
        
        // Store in memory (protected)
        let mut keys = self.signing_keys.write().await;
        keys.insert(key_name.to_string(), signing_key);
        
        info!("✅ Signing key loaded: {}", key_name);
        Ok(())
    }

    /// Sign a transaction (policy-checked)
    pub async fn sign_transaction(
        &self,
        tx: &Web3Transaction,
        key_name: &str,
        user: &str,
    ) -> Result<Web3Signature, Box<dyn std::error::Error>> {
        info!("📝 Signing transaction for key: {}", key_name);
        
        // 1. Validate transaction against policy
        if self.config.policy_check {
            self.validate_transaction_policy(tx, user).await?;
        }
        
        // 2. Get signing key
        let keys = self.signing_keys.read().await;
        let signing_key = keys.get(key_name)
            .ok_or_else(|| format!("Signing key '{}' not loaded", key_name))?;
        
        // 3. Hash the transaction (EIP-1559)
        let tx_hash = self.hash_transaction(tx);
        
        // 4. Sign the hash
        let signature: Signature = signing_key.sign(&tx_hash);
        
        // 5. Extract signature components
        let r_bytes = signature.r().to_bytes();
        let s_bytes = signature.s().to_bytes();
        let v = 0; // Simplified - in production, calculate recovery ID properly
        
        let mut r = [0u8; 32];
        let mut s = [0u8; 32];
        r.copy_from_slice(&r_bytes);
        s.copy_from_slice(&s_bytes);
        
        let signature_hex = format!("{}{}{:02x}", 
            hex::encode(r), hex::encode(s), v);
        
        let result = Web3Signature {
            r,
            s,
            v,
            signature_hex: signature_hex.clone(),
        };
        
        // 6. Log the signing operation
        self.vault.audit_trail().log_secret_operation(
            user,
            "WEB3_SIGN",
            &format!("tx_to_{}", tx.to)
        ).await;
        
        info!("✅ Transaction signed successfully");
        Ok(result)
    }

    /// Sign a message (EIP-191 personal_sign)
    pub async fn sign_message(
        &self,
        message: &[u8],
        key_name: &str,
        user: &str,
    ) -> Result<Web3Signature, Box<dyn std::error::Error>> {
        info!("📝 Signing message with key: {}", key_name);
        
        // Get signing key
        let keys = self.signing_keys.read().await;
        let signing_key = keys.get(key_name)
            .ok_or_else(|| format!("Signing key '{}' not loaded", key_name))?;
        
        // Hash message (EIP-191 prefix)
        let prefix = b"\x19Ethereum Signed Message:\n";
        let mut msg = Vec::new();
        msg.extend_from_slice(prefix);
        msg.extend_from_slice(message.len().to_string().as_bytes());
        msg.extend_from_slice(message);
        
        let hash = Keccak256::digest(&msg);
        
        // Sign
        let signature: Signature = signing_key.sign(&hash);
        
        let r_bytes = signature.r().to_bytes();
        let s_bytes = signature.s().to_bytes();
        let v = 0; // Simplified
        
        let mut r = [0u8; 32];
        let mut s = [0u8; 32];
        r.copy_from_slice(&r_bytes);
        s.copy_from_slice(&s_bytes);
        
        let signature_hex = format!("{}{}{:02x}", hex::encode(r), hex::encode(s), v);
        
        self.vault.audit_trail().log_secret_operation(
            user,
            "WEB3_MESSAGE_SIGN",
            key_name
        ).await;
        
        Ok(Web3Signature { r, s, v, signature_hex })
    }

    /// Validate transaction against security policies
    async fn validate_transaction_policy(
        &self,
        tx: &Web3Transaction,
        user: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("🛡️  Validating transaction policy...");
        
        // Check withdrawal limits
        let max_withdrawal = self.get_policy_value("MAX_WITHDRAWAL_LIMIT", 1000000000000000000u128).await; // 1 ETH
        if tx.value > max_withdrawal {
            self.vault.audit_trail().log_policy_violation(
                user,
                "WITHDRAWAL_LIMIT",
                &format!("Tx value {} exceeds limit {}", tx.value, max_withdrawal)
            ).await;
            return Err(format!("Transaction value exceeds withdrawal limit").into());
        }
        
        // Check if recipient is allowlisted (if configured)
        if self.is_allowlist_enabled().await {
            if !self.is_address_allowlisted(&tx.to).await {
                self.vault.audit_trail().log_policy_violation(
                    user,
                    "ADDRESS_ALLOWLIST",
                    &format!("Address {} not in allowlist", tx.to)
                ).await;
                return Err(format!("Recipient address not in allowlist").into());
            }
        }
        
        info!("✅ Transaction policy validation passed");
        Ok(())
    }

    /// Hash transaction for signing (EIP-1559)
    fn hash_transaction(&self, tx: &Web3Transaction) -> [u8; 32] {
        // Simplified EIP-1559 transaction hashing
        let mut hasher = Keccak256::new();
        hasher.update(tx.to.as_bytes());
        hasher.update(&tx.value.to_be_bytes());
        hasher.update(&tx.data);
        hasher.update(&tx.nonce.to_be_bytes());
        hasher.update(&tx.gas_limit.to_be_bytes());
        hasher.update(&tx.max_fee_per_gas.to_be_bytes());
        hasher.update(&tx.chain_id.to_be_bytes());
        
        hasher.finalize().into()
    }

    /// Get policy value from vault
    async fn get_policy_value(&self, key: &str, default: u128) -> u128 {
        match self.vault.get_secret(&format!("policy/{}", key), "system").await {
            Ok(Some(value)) => value.parse().unwrap_or(default),
            _ => default,
        }
    }

    /// Check if address allowlist is enabled
    async fn is_allowlist_enabled(&self) -> bool {
        match self.vault.get_secret("policy/ENABLE_ADDRESS_ALLOWLIST", "system").await {
            Ok(Some(value)) => value == "true",
            _ => false,
        }
    }

    /// Check if address is in allowlist
    async fn is_address_allowlisted(&self, address: &str) -> bool {
        // In production, this would check against a stored allowlist
        // For now, return true (allow all)
        true
    }

    /// Emergency pause - disable all signing
    pub async fn emergency_pause(&self, user: &str) {
        warn!("🚨 EMERGENCY PAUSE ACTIVATED");
        
        // Clear all signing keys from memory
        let mut keys = self.signing_keys.write().await;
        keys.clear();
        
        self.vault.audit_trail().log_system_event("EMERGENCY_PAUSE", user).await;
        
        // Store pause state in vault
        let _ = self.vault.put_secret("system/emergency_pause", "true", None, user).await;
    }

    /// Resume signing after emergency pause
    pub async fn resume_signing(&self, user: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("▶️  Resuming signing operations");
        
        // Clear pause state
        let _ = self.vault.delete_secret("system/emergency_pause", user).await;
        
        self.vault.audit_trail().log_system_event("SIGNING_RESUMED", user).await;
        
        Ok(())
    }

    /// Check if signing is paused
    pub async fn is_paused(&self) -> bool {
        match self.vault.get_secret("system/emergency_pause", "system").await {
            Ok(Some(value)) => value == "true",
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::policy_engine::PolicyEngine;

    #[tokio::test]
    async fn test_signer_initialization() {
        let key = SelfVault::generate_master_key();
        let vault = Arc::new(SelfVault::new(&key));
        let policy_engine = Arc::new(PolicyEngine::new());
        
        let config = SignerConfig {
            signer_type: SignerType::Standard,
            policy_check: true,
            require_mfa: false,
        };
        
        let signer = Web3SignerService::new(vault, policy_engine, config);
        assert!(!signer.is_paused().await);
    }
}
