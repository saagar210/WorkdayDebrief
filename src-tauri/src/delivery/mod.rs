pub mod email;
pub mod file;
pub mod slack;

use crate::error::AppError;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryConfirmation {
    pub delivery_type: String,
    pub success: bool,
    pub message: String,
    pub timestamp: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum DeliveryConfig {
    #[serde(rename = "email")]
    Email(email::SmtpConfig),
    #[serde(rename = "slack")]
    Slack(slack::SlackConfig),
    #[serde(rename = "file")]
    File(file::FileConfig),
}

/// Send summary to multiple delivery targets with retry logic
pub async fn send_summary(
    summary_markdown: &str,
    date: &str,
    configs: Vec<DeliveryConfig>,
) -> Vec<DeliveryConfirmation> {
    let mut confirmations = Vec::new();

    for config in configs {
        let confirmation = match config {
            DeliveryConfig::Email(email_config) => {
                send_email_with_retry(summary_markdown, &email_config).await
            }
            DeliveryConfig::Slack(slack_config) => {
                send_slack_with_retry(summary_markdown, &slack_config).await
            }
            DeliveryConfig::File(file_config) => {
                send_file_with_retry(summary_markdown, date, &file_config).await
            }
        };
        confirmations.push(confirmation);
    }

    confirmations
}

/// Email delivery with retry logic
async fn send_email_with_retry(
    summary_markdown: &str,
    config: &email::SmtpConfig,
) -> DeliveryConfirmation {
    let mut last_error = None;
    let backoff_delays = [1, 3, 9];

    for (attempt, delay_secs) in backoff_delays.iter().enumerate() {
        match email::send_email(summary_markdown, config) {
            Ok(()) => {
                return DeliveryConfirmation {
                    delivery_type: "email".to_string(),
                    success: true,
                    message: format!("Sent to {}", config.to_address),
                    timestamp: Local::now().to_rfc3339(),
                };
            }
            Err(e) => {
                let is_retryable = match &e {
                    AppError::SmtpAuthFailed(msg) => {
                        !msg.contains("Wrong password") && !msg.contains("535")
                    }
                    _ => false,
                };

                last_error = Some(e);
                if !is_retryable || attempt == 2 {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(*delay_secs)).await;
            }
        }
    }

    DeliveryConfirmation {
        delivery_type: "email".to_string(),
        success: false,
        message: last_error.map(|e| e.to_string()).unwrap_or_default(),
        timestamp: Local::now().to_rfc3339(),
    }
}

/// Slack delivery with retry logic
async fn send_slack_with_retry(
    summary_text: &str,
    config: &slack::SlackConfig,
) -> DeliveryConfirmation {
    let mut last_error = None;
    let backoff_delays = [1, 3, 9];

    for (attempt, delay_secs) in backoff_delays.iter().enumerate() {
        match slack::send_slack(summary_text, config).await {
            Ok(()) => {
                return DeliveryConfirmation {
                    delivery_type: "slack".to_string(),
                    success: true,
                    message: "Posted to Slack".to_string(),
                    timestamp: Local::now().to_rfc3339(),
                };
            }
            Err(e) => {
                let is_retryable = match &e {
                    AppError::SlackWebhookInvalid(msg) => {
                        msg.contains("timeout") || msg.contains("Rate limited")
                    }
                    _ => false,
                };

                last_error = Some(e);
                if !is_retryable || attempt == 2 {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(*delay_secs)).await;
            }
        }
    }

    DeliveryConfirmation {
        delivery_type: "slack".to_string(),
        success: false,
        message: last_error.map(|e| e.to_string()).unwrap_or_default(),
        timestamp: Local::now().to_rfc3339(),
    }
}

/// File delivery with retry logic
async fn send_file_with_retry(
    summary: &str,
    date: &str,
    config: &file::FileConfig,
) -> DeliveryConfirmation {
    let mut last_error = None;
    let backoff_delays = [1, 3, 9];

    for (attempt, delay_secs) in backoff_delays.iter().enumerate() {
        match file::write_markdown(summary, config, date) {
            Ok(path) => {
                return DeliveryConfirmation {
                    delivery_type: "file".to_string(),
                    success: true,
                    message: format!("Written to {}", path.display()),
                    timestamp: Local::now().to_rfc3339(),
                };
            }
            Err(e) => {
                let is_retryable = match &e {
                    AppError::FileWriteError(msg) => {
                        !msg.contains("Permission denied") && !msg.contains("Disk full")
                    }
                    _ => false,
                };

                last_error = Some(e);
                if !is_retryable || attempt == 2 {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(*delay_secs)).await;
            }
        }
    }

    DeliveryConfirmation {
        delivery_type: "file".to_string(),
        success: false,
        message: last_error.map(|e| e.to_string()).unwrap_or_default(),
        timestamp: Local::now().to_rfc3339(),
    }
}
