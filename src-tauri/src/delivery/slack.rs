use crate::error::AppError;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct SlackConfig {
    pub webhook_url: String,
}

#[derive(Debug, Serialize)]
struct SlackMessage {
    text: String,
}

/// Send message to Slack via webhook
pub async fn send_slack(summary_text: &str, config: &SlackConfig) -> Result<(), AppError> {
    // Truncate if too long (Slack limit is ~4000 chars, we use 3000 to be safe)
    let mut text = summary_text.to_string();
    if text.len() > 3000 {
        text.truncate(3000);
        text.push_str("\n\n_Full summary sent via email_");
    }

    let payload = SlackMessage { text };

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::SlackWebhookInvalid(format!("HTTP client error: {}", e)))?;

    let response = client
        .post(&config.webhook_url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                AppError::SlackWebhookInvalid("Request timed out".to_string())
            } else {
                AppError::SlackWebhookInvalid(format!("Failed to send: {}", e))
            }
        })?;

    let status = response.status();

    if status == 403 {
        return Err(AppError::SlackWebhookInvalid(
            "Webhook expired or invalid (403)".to_string(),
        ));
    } else if status == 429 {
        // Rate limited - get retry-after header
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(5);

        return Err(AppError::SlackWebhookInvalid(format!(
            "Rate limited - retry after {} seconds",
            retry_after
        )));
    } else if !status.is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(AppError::SlackWebhookInvalid(format!(
            "HTTP {}: {}",
            status, error_text
        )));
    }

    Ok(())
}
