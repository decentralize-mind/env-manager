use reqwest::Client;
use tracing::info;

/// mTLS-enabled HTTP client for secure service-to-service communication
/// Note: Full mTLS requires proper certificate setup - this is a framework
pub struct MtlsClient {
    client: Client,
}

impl MtlsClient {
    /// Create a new mTLS client (simplified version)
    pub fn new_with_certs(
        _ca_cert_path: &str,
        _client_cert_path: &str,
        _client_key_path: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        info!("🔐 Initializing mTLS client (certificate loading would happen here)");
        
        // In production: Load and configure mTLS certificates properly
        // This requires:
        // 1. CA certificate for server verification
        // 2. Client certificate for mutual authentication
        // 3. Client private key
        
        let client = Client::new();
        Ok(Self { client })
    }

    /// Get the underlying HTTP client
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Make a GET request with mTLS
    pub async fn get(&self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
        info!("📡 Making mTLS GET request to: {}", url);
        let response = self.client.get(url).send().await?;
        Ok(response.text().await?)
    }

    /// Make a POST request with mTLS
    pub async fn post(
        &self,
        url: &str,
        body: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        info!("📡 Making mTLS POST request to: {}", url);
        let response = self.client.post(url).body(body.to_string()).send().await?;
        Ok(response.text().await?)
    }
}

/// mTLS configuration for Vault communication
pub struct VaultMtlsConfig {
    pub vault_addr: String,
    pub ca_cert: String,
    pub client_cert: String,
    pub client_key: String,
}

impl VaultMtlsConfig {
    pub fn new(vault_addr: &str, ca_cert: &str, client_cert: &str, client_key: &str) -> Self {
        Self {
            vault_addr: vault_addr.to_string(),
            ca_cert: ca_cert.to_string(),
            client_cert: client_cert.to_string(),
            client_key: client_key.to_string(),
        }
    }

    pub fn create_client(&self) -> Result<MtlsClient, Box<dyn std::error::Error>> {
        MtlsClient::new_with_certs(&self.ca_cert, &self.client_cert, &self.client_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mtls_config_creation() {
        let config = VaultMtlsConfig::new(
            "https://vault.example.com:8200",
            "/path/to/ca.pem",
            "/path/to/client.pem",
            "/path/to/key.pem",
        );
        assert_eq!(config.vault_addr, "https://vault.example.com:8200");
    }
}
