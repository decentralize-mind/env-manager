use config::{Config, Environment, File};
use crate::config::schema::AppConfig;

pub fn load_config() -> Result<AppConfig, config::ConfigError> {
    dotenvy::dotenv().ok(); // Load .env file if it exists

    let builder = Config::builder()
        .add_source(File::with_name("config").required(false))
        .add_source(
            Environment::default()
                .separator("_")
                .try_parsing(true)
        );

    let config = builder.build()?;
    config.try_deserialize()
}
