use tokio::time::{sleep, Duration};
use tracing::info;

pub async fn rotation_loop() {
    loop {
        info!("🔄 Rotating secrets...");

        // 1. Request new secret from Vault
        // 2. Update runtime cache
        // 3. Keep old + new temporarily

        sleep(Duration::from_secs(3600)).await; // every 1h
    }
}
