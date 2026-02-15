use thiserror::Error;

#[derive(Debug, Error, serde::Serialize)]
pub enum AppError {
    #[error("Jira error: {0}")]
    JiraUnreachable(String),

    #[error(
        "Google Calendar requires re-authentication. Click 'Connect Google Account' in Settings."
    )]
    CalendarUnauthorized,

    #[error("Google Calendar error: {0}")]
    CalendarError(String),

    #[error("Toggl Track error: {0}")]
    TogglError(String),

    #[error("LLM generation timed out after {0}s. Try increasing timeout in Settings or using a faster model.")]
    LlmTimeout(u64),

    #[error("Ollama is not running. Start Ollama and try again, or disable LLM in Settings to use bullet-point summaries.")]
    LlmUnavailable(String),

    #[error("Email delivery failed: {0}")]
    SmtpAuthFailed(String),

    #[error("Slack delivery failed: {0}")]
    SlackWebhookInvalid(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("File operation failed: {0}")]
    FileWriteError(String),

    #[error("{0}")]
    NotConfigured(String),

    #[error("Network timeout: {0}. Check your internet connection and try again.")]
    NetworkTimeout(String),
}

// Implement From for common error types
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::DatabaseError(err.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::FileWriteError(err.to_string())
    }
}
