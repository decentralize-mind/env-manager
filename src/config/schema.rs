use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub app: App,
    pub db: Database,
    pub security: Security,
}

#[derive(Debug, Deserialize)]
pub struct App {
    pub name: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct Database {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct Security {
    #[serde(rename = "jwt_secret")]
    pub jwt_secret: String,
    #[serde(rename = "encryption_key")]
    pub encryption_key: String,
}
