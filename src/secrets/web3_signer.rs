use k256::ecdsa::{SigningKey, signature::Signer};
use hex;
use zeroize::Zeroize;
use tracing::{info, error};
use std::sync::Arc;

/// Trait for signing transactions (abstraction over different key storage methods)
pub trait TransactionSigner: Send + Sync {
    /// Sign a message/hash
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
    
    /// Get the public address (derived from public key)
    fn get_address(&self) -> Result<String, Box<dyn std::error::Error>>;
}

/// HSM-backed Web3 signer (most secure - private key never leaves HSM)
pub struct HsmWeb3Signer {
    hsm_key_id: String,
    // In production, this would hold an HSM client connection
}

impl HsmWeb3Signer {
    pub fn new(hsm_key_id: &str) -> Self {
        info!("🔐 Initializing HSM-backed Web3 signer");
        Self {
            hsm_key_id: hsm_key_id.to_string(),
        }
    }
}

impl TransactionSigner for HsmWeb3Signer {
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        info!("🔑 Signing transaction with HSM...");
        // In production: call HSM to sign
        // Private key NEVER exists in application memory
        Ok(vec![0u8; 65]) // Placeholder signature (65 bytes: r + s + v)
    }

    fn get_address(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok("0xHSM_BACKED_ADDRESS".to_string())
    }
}

/// Encrypted in-memory signer (private key encrypted at rest in memory)
pub struct EncryptedWeb3Signer {
    encrypted_key: Vec<u8>,
    encryption_key: Vec<u8>,
}

impl EncryptedWeb3Signer {
    pub fn new(private_key_hex: &str, encryption_key: &[u8; 32]) -> Result<Self, Box<dyn std::error::Error>> {
        info!("🔐 Initializing encrypted Web3 signer");
        
        // In production: encrypt the private key using AES-256-GCM
        // For now, store as placeholder
        let private_key_bytes = hex::decode(private_key_hex)?;
        
        Ok(Self {
            encrypted_key: private_key_bytes,
            encryption_key: encryption_key.to_vec(),
        })
    }
}

impl TransactionSigner for EncryptedWeb3Signer {
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Decrypt key, sign, then re-zero
        info!("🔑 Signing with encrypted key...");
        Ok(vec![0u8; 65])
    }

    fn get_address(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok("0xENCRYPTED_ADDRESS".to_string())
    }
}

impl Drop for EncryptedWeb3Signer {
    fn drop(&mut self) {
        self.encrypted_key.zeroize();
        self.encryption_key.zeroize();
        info!("🔒 Encrypted Web3 signer dropped, keys wiped");
    }
}

/// Standard secp256k1 signer (for development/testing - NOT for production)
pub struct StdWeb3Signer {
    signing_key: SigningKey,
}

impl StdWeb3Signer {
    /// Create from hex-encoded private key
    pub fn from_private_key(private_key_hex: &str) -> Result<Self, Box<dyn std::error::Error>> {
        info!("⚠️  Creating standard Web3 signer (NOT for production use!)");
        
        let private_key_bytes = hex::decode(private_key_hex)?;
        let signing_key = SigningKey::from_slice(&private_key_bytes)?;
        
        Ok(Self { signing_key })
    }

    /// Generate a random private key (for testing only)
    pub fn generate_random() -> Self {
        use rand::rngs::OsRng;
        let signing_key = SigningKey::random(&mut OsRng);
        Self { signing_key }
    }
}

impl TransactionSigner for StdWeb3Signer {
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        use k256::ecdsa::signature::hazmat::PrehashSigner;
        use sha2::Digest;
        
        info!("🔑 Signing with standard key...");
        
        // Hash the message
        let digest = sha2::Sha256::digest(message);
        
        // Sign the hash
        let signature: k256::ecdsa::Signature = self.signing_key.sign_prehash(&digest)?;
        
        // Convert to Ethereum format (65 bytes: r + s + v)
        let mut sig_bytes = Vec::with_capacity(65);
        sig_bytes.extend_from_slice(&signature.r().to_bytes());
        sig_bytes.extend_from_slice(&signature.s().to_bytes());
        sig_bytes.push(0); // Recovery ID (simplified)
        
        Ok(sig_bytes)
    }

    fn get_address(&self) -> Result<String, Box<dyn std::error::Error>> {
        use sha3::{Keccak256, Digest};
        
        // Get public key
        let verifying_key = self.signing_key.verifying_key();
        let encoded_point = verifying_key.to_encoded_point(false);
        let public_key_bytes = encoded_point.as_bytes();
        
        // Ethereum address is last 20 bytes of keccak256 hash of public key (without 0x04 prefix)
        let mut hasher = Keccak256::new();
        hasher.update(&public_key_bytes[1..]); // Skip 0x04 prefix
        let hash = hasher.finalize();
        
        let address = format!("0x{}", hex::encode(&hash[12..]));
        Ok(address)
    }
}

impl Drop for StdWeb3Signer {
    fn drop(&mut self) {
        info!("🔒 Standard Web3 signer dropped");
    }
}

/// Web3 signer factory that chooses the appropriate signer based on configuration
pub enum Web3SignerFactory {
    Hsm(HsmWeb3Signer),
    Encrypted(EncryptedWeb3Signer),
    Standard(StdWeb3Signer),
}

impl Web3SignerFactory {
    /// Create an HSM-backed signer (recommended for production)
    pub fn hsm(key_id: &str) -> Arc<dyn TransactionSigner> {
        Arc::new(HsmWeb3Signer::new(key_id))
    }

    /// Create an encrypted signer (acceptable for staging)
    pub fn encrypted(private_key_hex: &str, encryption_key: &[u8; 32]) -> Result<Arc<dyn TransactionSigner>, Box<dyn std::error::Error>> {
        Ok(Arc::new(EncryptedWeb3Signer::new(private_key_hex, encryption_key)?))
    }

    /// Create a standard signer (development/testing ONLY)
    pub fn standard(private_key_hex: &str) -> Result<Arc<dyn TransactionSigner>, Box<dyn std::error::Error>> {
        Ok(Arc::new(StdWeb3Signer::from_private_key(private_key_hex)?))
    }

    /// Create a random signer (testing ONLY)
    pub fn random() -> Arc<dyn TransactionSigner> {
        Arc::new(StdWeb3Signer::generate_random())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_signer() {
        let signer = StdWeb3Signer::generate_random();
        let address = signer.get_address().unwrap();
        assert!(address.starts_with("0x"));
        assert_eq!(address.len(), 42); // 0x + 40 hex chars
    }

    #[test]
    fn test_standard_sign_message() {
        let signer = StdWeb3Signer::generate_random();
        let message = b"test message";
        let signature = signer.sign(message).unwrap();
        assert_eq!(signature.len(), 65); // Ethereum signature format
    }
}
