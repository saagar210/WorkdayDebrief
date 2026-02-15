use crate::error::AppError;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub from_address: String,
    pub to_address: String,
    pub username: String,
    pub password: String,
    pub use_tls: bool,
}

/// Send email via SMTP using lettre
pub fn send_email(summary_markdown: &str, config: &SmtpConfig) -> Result<(), AppError> {
    // Build email message
    let email = Message::builder()
        .from(
            config
                .from_address
                .parse()
                .map_err(|e| AppError::SmtpAuthFailed(format!("Invalid from address: {}", e)))?,
        )
        .to(config
            .to_address
            .parse()
            .map_err(|e| AppError::SmtpAuthFailed(format!("Invalid to address: {}", e)))?)
        .subject("Work Summary")
        .header(ContentType::TEXT_PLAIN)
        .body(summary_markdown.to_string())
        .map_err(|e| AppError::SmtpAuthFailed(format!("Failed to build email: {}", e)))?;

    // Build SMTP transport
    let mailer = if config.use_tls {
        SmtpTransport::relay(&config.host)
            .map_err(|e| {
                AppError::SmtpAuthFailed(format!(
                    "Cannot connect to {}:{} - {}",
                    config.host, config.port, e
                ))
            })?
            .credentials(Credentials::new(
                config.username.clone(),
                config.password.clone(),
            ))
            .port(config.port)
            .build()
    } else {
        SmtpTransport::builder_dangerous(&config.host)
            .credentials(Credentials::new(
                config.username.clone(),
                config.password.clone(),
            ))
            .port(config.port)
            .build()
    };

    // Send email
    mailer.send(&email).map_err(|e| {
        let error_str = e.to_string();
        if error_str.contains("535") {
            AppError::SmtpAuthFailed("Wrong password or username".to_string())
        } else if error_str.contains("timeout") || error_str.contains("timed out") {
            AppError::SmtpAuthFailed("SMTP server timed out".to_string())
        } else {
            AppError::SmtpAuthFailed(format!("Failed to send email: {}", e))
        }
    })?;

    Ok(())
}
