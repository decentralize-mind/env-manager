use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct VaultResponse {
    data: VaultData,
}

#[derive(Deserialize)]
struct VaultData {
    data: HashMap<String, String>,
}

pub struct VaultClient {
    client: Client,
    base_url: String,
    token: String,
}

impl VaultClient {
    pub fn new(base_url: String, token: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            token,
        }
    }

    pub async fn get_secret(&self, path: &str, key: &str) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!("{}/v1/{}", self.base_url, path);

        let res = self.client
            .get(url)
            .header("X-Vault-Token", &self.token)
            .send()
            .await?
            .json::<VaultResponse>()
            .await?;

        Ok(res.data.data.get(key).cloned().unwrap_or_default())
    }
}
