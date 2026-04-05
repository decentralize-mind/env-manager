use serde::Serialize;
use tracing::{info, warn, error};

/// Telegram notification service
pub struct TelegramNotifier {
    bot_token: String,
    chat_id: String,
    client: reqwest::Client,
}

#[derive(Serialize)]
struct TelegramMessage {
    chat_id: String,
    text: String,
    parse_mode: String,
}

impl TelegramNotifier {
    /// Create new Telegram notifier
    pub fn new(bot_token: &str, chat_id: &str) -> Self {
        Self {
            bot_token: bot_token.to_string(),
            chat_id: chat_id.to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Create from environment variables
    pub fn from_env() -> Option<Self> {
        let bot_token = std::env::var("TELEGRAM_BOT_TOKEN").ok()?;
        let chat_id = std::env::var("TELEGRAM_CHAT_ID").ok()?;
        
        if bot_token.is_empty() || chat_id.is_empty() {
            return None;
        }
        
        Some(Self::new(&bot_token, &chat_id))
    }

    /// Send deployment notification
    pub async fn send_deployment_notification(
        &self,
        environment: &str,
        version: &str,
        status: &str,
        commit_hash: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let emoji = match status {
            "success" => "✅",
            "failure" => "❌",
            "started" => "🚀",
            _ => "📢",
        };

        let message = format!(
            r#"<b>{emoji} Deployment {status}</b>

<b>Environment:</b> {environment}
<b>Version:</b> {version}
<b>Commit:</b> <code>{commit_hash}</code>
<b>Time:</b> {time}

<b>Repository:</b> env-manager
<b>Status:</b> {status_msg}"#,
            emoji = emoji,
            status = status.to_uppercase(),
            environment = environment,
            version = version,
            commit_hash = commit_hash,
            time = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            status_msg = match status {
                "success" => "Deployment completed successfully!",
                "failure" => "Deployment failed! Check logs immediately.",
                "started" => "Deployment started...",
                _ => "Status update",
            }
        );

        self.send_message(&message).await
    }

    /// Send security alert
    pub async fn send_security_alert(
        &self,
        alert_type: &str,
        details: &str,
        severity: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let emoji = match severity {
            "critical" => "🚨",
            "high" => "⚠️",
            "medium" => "🔶",
            _ => "ℹ️",
        };

        let message = format!(
            r#"<b>{emoji} Security Alert</b>

<b>Type:</b> {alert_type}
<b>Severity:</b> {severity}
<b>Time:</b> {time}

<b>Details:</b>
{details}

<b>Action Required:</b> Please investigate immediately."#,
            emoji = emoji,
            alert_type = alert_type,
            severity = severity,
            time = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            details = details
        );

        self.send_message(&message).await
    }

    /// Send emergency shutdown notification
    pub async fn send_emergency_shutdown(
        &self,
        reason: &str,
        triggered_by: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let message = format!(
            r#"<b>🚨 EMERGENCY SHUTDOWN TRIGGERED</b>

<b>Reason:</b> {reason}
<b>Triggered By:</b> {triggered_by}
<b>Time:</b> {time}

<b>Status:</b> All operations halted
<b>Action:</b> System is now in lockdown mode

<i>Please follow incident response procedures.</i>"#,
            reason = reason,
            triggered_by = triggered_by,
            time = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );

        self.send_message(&message).await
    }

    /// Send test message
    pub async fn send_test_message(&self) -> Result<(), Box<dyn std::error::Error>> {
        let message = format!(
            r#"<b>🔔 Telegram Notification Test</b>

<b>Time:</b> {}
<b>Status:</b> ✅ Notifications are working!

This is a test message from env-manager."#,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );

        self.send_message(&message).await
    }

    /// Send message to Telegram
    async fn send_message(&self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token);
        
        let message = TelegramMessage {
            chat_id: self.chat_id.clone(),
            text: text.to_string(),
            parse_mode: "HTML".to_string(),
        };

        info!("📤 Sending Telegram notification...");

        let response = self.client
            .post(&url)
            .json(&message)
            .send()
            .await?;

        if response.status().is_success() {
            info!("✅ Telegram notification sent successfully");
            Ok(())
        } else {
            let error_text = response.text().await?;
            error!("❌ Telegram API error: {}", error_text);
            Err(format!("Telegram API error: {}", error_text).into())
        }
    }
}

/// Convenience function to send notification if configured
pub async fn notify_if_configured(message: &str) {
    if let Some(notifier) = TelegramNotifier::from_env() {
        match notifier.send_message(message).await {
            Ok(_) => info!("✅ Telegram notification sent"),
            Err(e) => warn!("⚠️  Failed to send Telegram notification: {}", e),
        }
    } else {
        info!("ℹ️  Telegram not configured (set TELEGRAM_BOT_TOKEN and TELEGRAM_CHAT_ID)");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notifier_creation() {
        let notifier = TelegramNotifier::new("test_token", "test_chat");
        assert_eq!(notifier.bot_token, "test_token");
        assert_eq!(notifier.chat_id, "test_chat");
    }

    #[tokio::test]
    async fn test_from_env_not_configured() {
        // Clear env vars
        std::env::remove_var("TELEGRAM_BOT_TOKEN");
        std::env::remove_var("TELEGRAM_CHAT_ID");
        
        let notifier = TelegramNotifier::from_env();
        assert!(notifier.is_none());
    }
}
