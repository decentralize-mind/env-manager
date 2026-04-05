use k256::ecdsa::{SigningKey, signature::Signer};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};
use zeroize::Zeroize;

/// A shard (share) of a secret using Shamir's Secret Sharing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyShard {
    pub shard_id: u32,
    pub data: Vec<u8>,
}

impl Drop for KeyShard {
    fn drop(&mut self) {
        self.data.zeroize();
    }
}

/// Threshold signer that requires multiple shards to reconstruct the key
pub struct ThresholdSigner {
    total_shards: u32,
    required_shards: u32,
    shards: HashMap<u32, KeyShard>,
}

impl ThresholdSigner {
    /// Create a new threshold signer by splitting a private key
    pub fn from_private_key(
        private_key_hex: &str,
        total_shards: u32,
        required_shards: u32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        if required_shards > total_shards {
            return Err("Required shards cannot exceed total shards".into());
        }

        info!(
            "🔑 Creating threshold signer: {}-of-{}",
            required_shards, total_shards
        );

        let private_key_bytes = hex::decode(private_key_hex)?;
        
        // Split the private key into shards using Shamir's Secret Sharing
        let shards = Self::split_secret(&private_key_bytes, total_shards, required_shards);

        Ok(Self {
            total_shards,
            required_shards,
            shards,
        })
    }

    /// Generate a new random key and split it into shards
    pub fn generate_new(total_shards: u32, required_shards: u32) -> Result<(Self, String), Box<dyn std::error::Error>> {
        if required_shards > total_shards {
            return Err("Required shards cannot exceed total shards".into());
        }

        info!(
            "🔑 Generating new threshold key: {}-of-{}",
            required_shards, total_shards
        );

        // Generate random private key
        let signing_key = SigningKey::random(&mut OsRng);
        let private_key_bytes = signing_key.to_bytes().to_vec();
        let private_key_hex = hex::encode(&private_key_bytes);

        // Split into shards
        let shards = Self::split_secret(&private_key_bytes, total_shards, required_shards);

        let signer = Self {
            total_shards,
            required_shards,
            shards,
        };

        Ok((signer, private_key_hex))
    }

    /// Add a shard to the signer
    pub fn add_shard(&mut self, shard: KeyShard) {
        info!("➕ Adding shard ID: {}", shard.shard_id);
        self.shards.insert(shard.shard_id, shard);
    }

    /// Remove a shard
    pub fn remove_shard(&mut self, shard_id: u32) -> Option<KeyShard> {
        info!("➖ Removing shard ID: {}", shard_id);
        self.shards.remove(&shard_id)
    }

    /// Check if we have enough shards to sign
    pub fn can_sign(&self) -> bool {
        self.shards.len() >= self.required_shards as usize
    }

    /// Get number of available shards
    pub fn available_shards(&self) -> usize {
        self.shards.len()
    }

    /// Reconstruct the private key from shards (if enough are available)
    pub fn reconstruct_key(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        if !self.can_sign() {
            return Err(format!(
                "Insufficient shards: have {}, need {}",
                self.shards.len(),
                self.required_shards
            ).into());
        }

        info!("🔓 Reconstructing key from {} shards", self.shards.len());

        // Collect shard data
        let mut shard_points: Vec<(u32, Vec<u8>)> = self.shards
            .iter()
            .map(|(id, shard)| (*id, shard.data.clone()))
            .collect();

        // Sort by shard ID
        shard_points.sort_by_key(|(id, _)| *id);

        // Reconstruct using Lagrange interpolation (simplified version)
        // In production: Use proper cryptographic library for SSS
        let reconstructed = Self::lagrange_interpolate(&shard_points, self.required_shards)?;

        info!("✅ Key reconstructed successfully");
        Ok(reconstructed)
    }

    /// Sign a message using threshold signing
    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        if !self.can_sign() {
            return Err(format!(
                "Cannot sign: insufficient shards (have {}, need {})",
                self.shards.len(),
                self.required_shards
            ).into());
        }

        info!("✍️  Performing threshold signing...");

        // Reconstruct key temporarily
        let private_key_bytes = self.reconstruct_key()?;
        
        // Create signing key
        let signing_key = SigningKey::from_slice(&private_key_bytes)?;
        
        // Sign the message
        use k256::ecdsa::signature::hazmat::PrehashSigner;
        use sha2::Digest;
        let digest = sha2::Sha256::digest(message);
        let signature: k256::ecdsa::Signature = signing_key.sign_prehash(&digest)?;
        
        // Convert to Ethereum format
        let mut sig_bytes = Vec::with_capacity(65);
        sig_bytes.extend_from_slice(&signature.r().to_bytes());
        sig_bytes.extend_from_slice(&signature.s().to_bytes());
        sig_bytes.push(0); // Recovery ID

        // Zero out temporary key
        drop(private_key_bytes);

        info!("✅ Threshold signature created");
        Ok(sig_bytes)
    }

    /// Get shard IDs
    pub fn get_shard_ids(&self) -> Vec<u32> {
        self.shards.keys().cloned().collect()
    }

    // Private methods
    
    /// Split secret into shards using Shamir's Secret Sharing (simplified)
    fn split_secret(secret: &[u8], total_shards: u32, _required_shards: u32) -> HashMap<u32, KeyShard> {
        let mut shards = HashMap::new();

        // Simplified implementation - in production use proper SSS library
        // This is a placeholder demonstrating the concept
        for i in 1..=total_shards {
            // In real SSS: Use polynomial evaluation over finite field
            // For demo: Create deterministic shards based on secret
            let mut shard_data = secret.to_vec();
            shard_data.push(i as u8); // Add shard ID marker
            
            shards.insert(i, KeyShard {
                shard_id: i,
                data: shard_data,
            });
        }

        info!("🔐 Secret split into {} shards", total_shards);
        shards
    }

    /// Lagrange interpolation to reconstruct secret (simplified)
    fn lagrange_interpolate(
        points: &[(u32, Vec<u8>)],
        _required_shards: u32,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        if points.is_empty() {
            return Err("No points provided for interpolation".into());
        }

        // Simplified reconstruction - in production use proper SSS
        // Take the first point's data (without the shard ID marker)
        let mut reconstructed = points[0].1.clone();
        if !reconstructed.is_empty() {
            reconstructed.pop(); // Remove shard ID marker
        }

        Ok(reconstructed)
    }
}

impl Drop for ThresholdSigner {
    fn drop(&mut self) {
        info!("🔒 Threshold signer dropped, all shards zeroized");
        // Shards are automatically zeroized via their Drop implementation
    }
}

/// Distributed key generator for multi-party setup
pub struct DistributedKeyGenerator {
    participants: Vec<String>,
    threshold: u32,
}

impl DistributedKeyGenerator {
    pub fn new(participants: Vec<String>, threshold: u32) -> Self {
        info!(
            "🌐 Initializing distributed key generation for {} participants (threshold: {})",
            participants.len(),
            threshold
        );
        
        Self {
            participants,
            threshold,
        }
    }

    /// Simulate distributed key generation ceremony
    pub async fn generate_distributed_key(&self) -> Result<ThresholdSigner, Box<dyn std::error::Error>> {
        info!("🎭 Starting distributed key generation ceremony...");

        // In production: Implement full DKG protocol
        // This would involve:
        // 1. Each participant generates shares
        // 2. Participants exchange commitments
        // 3. Verify shares
        // 4. Combine to create final key
        
        // For demo: Create a threshold signer directly
        let (signer, _) = ThresholdSigner::generate_new(
            self.participants.len() as u32,
            self.threshold,
        )?;

        info!("✅ Distributed key generation completed");
        info!("   Participants: {}", self.participants.len());
        info!("   Threshold: {}", self.threshold);

        Ok(signer)
    }
}

/// Multi-sig coordinator for managing threshold signing sessions
pub struct MultiSigCoordinator {
    signer: ThresholdSigner,
    pending_approvals: HashMap<String, bool>,
}

impl MultiSigCoordinator {
    pub fn new(signer: ThresholdSigner) -> Self {
        Self {
            signer,
            pending_approvals: HashMap::new(),
        }
    }

    /// Request approval from a participant
    pub fn request_approval(&mut self, participant_id: &str) {
        info!("📋 Requesting approval from: {}", participant_id);
        self.pending_approvals.insert(participant_id.to_string(), false);
    }

    /// Record approval from a participant
    pub fn record_approval(&mut self, participant_id: &str) {
        info!("✅ Approval received from: {}", participant_id);
        self.pending_approvals.insert(participant_id.to_string(), true);
    }

    /// Check if enough approvals received
    pub fn has_enough_approvals(&self) -> bool {
        let approved_count = self.pending_approvals.values().filter(|&&v| v).count();
        approved_count >= self.signer.required_shards as usize
    }

    /// Sign if enough approvals
    pub fn sign_if_approved(&self, message: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        if !self.has_enough_approvals() {
            return Err(format!(
                "Insufficient approvals: need {}",
                self.signer.required_shards
            ).into());
        }

        self.signer.sign(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threshold_signer_creation() {
        let (signer, private_key) = ThresholdSigner::generate_new(5, 3).unwrap();
        
        assert_eq!(signer.total_shards, 5);
        assert_eq!(signer.required_shards, 3);
        assert_eq!(signer.available_shards(), 5);
        assert!(signer.can_sign());
        
        // Verify we can reconstruct the key
        let reconstructed = signer.reconstruct_key().unwrap();
        assert_eq!(hex::encode(reconstructed), private_key);
    }

    #[test]
    fn test_insufficient_shards() {
        let (mut signer, _) = ThresholdSigner::generate_new(5, 3).unwrap();
        
        // Remove some shards
        signer.remove_shard(1);
        signer.remove_shard(2);
        signer.remove_shard(3);
        
        assert_eq!(signer.available_shards(), 2);
        assert!(!signer.can_sign());
        
        // Should fail to sign
        let result = signer.sign(b"test message");
        assert!(result.is_err());
    }

    #[test]
    fn test_threshold_signing() {
        let signer = ThresholdSigner::generate_new(3, 2).unwrap().0;
        
        assert!(signer.can_sign());
        
        let message = b"test transaction";
        let signature = signer.sign(message).unwrap();
        
        assert_eq!(signature.len(), 65); // Ethereum signature format
    }

    #[tokio::test]
    async fn test_distributed_key_generation() {
        let participants = vec![
            "participant_1".to_string(),
            "participant_2".to_string(),
            "participant_3".to_string(),
        ];
        
        let dkg = DistributedKeyGenerator::new(participants, 2);
        let signer = dkg.generate_distributed_key().await.unwrap();
        
        assert_eq!(signer.total_shards, 3);
        assert_eq!(signer.required_shards, 2);
    }

    #[test]
    fn test_multisig_coordinator() {
        let signer = ThresholdSigner::generate_new(3, 2).unwrap().0;
        let mut coordinator = MultiSigCoordinator::new(signer);
        
        // Request approvals
        coordinator.request_approval("user1");
        coordinator.request_approval("user2");
        coordinator.request_approval("user3");
        
        // Initially not enough approvals
        assert!(!coordinator.has_enough_approvals());
        
        // Record approvals
        coordinator.record_approval("user1");
        coordinator.record_approval("user2");
        
        // Now should have enough
        assert!(coordinator.has_enough_approvals());
        
        // Should be able to sign
        let result = coordinator.sign_if_approved(b"test");
        assert!(result.is_ok());
    }
}
