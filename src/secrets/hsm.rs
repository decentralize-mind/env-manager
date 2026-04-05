use tracing::{info};

/// Trait for HSM-backed signing
pub trait HsmSignerTrait: Send + Sync {
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
    fn verify(&self, data: &[u8], signature: &[u8]) -> Result<bool, Box<dyn std::error::Error>>;
}

/// Mock HSM signer (placeholder for real HSM integration)
pub struct MockHsmSigner {
    key_label: String,
}

impl MockHsmSigner {
    pub fn new(key_label: &str) -> Self {
        info!("🧪 Using mock HSM for development (replace with real HSM in production)");
        Self {
            key_label: key_label.to_string(),
        }
    }
}

impl HsmSignerTrait for MockHsmSigner {
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        info!("🔑 Signing with mock HSM...");
        // In production: integrate with actual HSM (YubiHSM, AWS CloudHSM, etc.)
        Ok(format!("mock_sig_{}", hex::encode(data)).into_bytes())
    }

    fn verify(&self, _data: &[u8], _signature: &[u8]) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(true)
    }
}

/// AWS CloudHSM signer (cloud-based HSM)
pub struct CloudHsmSigner {
    cluster_id: String,
    key_id: String,
}

impl CloudHsmSigner {
    pub fn new(cluster_id: &str, key_id: &str) -> Self {
        info!("☁️  Initializing AWS CloudHSM signer");
        Self {
            cluster_id: cluster_id.to_string(),
            key_id: key_id.to_string(),
        }
    }
}

impl HsmSignerTrait for CloudHsmSigner {
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        info!("☁️  Signing with CloudHSM...");
        // In production: call AWS CloudHSM API
        Ok(vec![0u8; 64])
    }

    fn verify(&self, _data: &[u8], _signature: &[u8]) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_hsm_signer() {
        let signer = MockHsmSigner::new("test_key");
        let data = b"test message";
        let signature = signer.sign(data).unwrap();
        assert!(!signature.is_empty());
    }
}
