pub mod prompts;

use crate::aggregation::AggregatedData;
use crate::commands::SummaryInput;
use crate::error::AppError;
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::models::ModelOptions;
use ollama_rs::Ollama;
use std::time::Duration;

/// Generate narrative summary using Ollama LLM
pub async fn generate_narrative(
    data: &AggregatedData,
    user_fields: &SummaryInput,
    tone: &str,
    model: &str,
    temperature: f32,
    timeout_secs: u64,
) -> Result<String, AppError> {
    // Build the prompt
    let prompt = prompts::build_prompt(data, user_fields, tone);

    // Create Ollama client (localhost:11434)
    let ollama = Ollama::default();

    // Build generation options
    let options = ModelOptions::default()
        .temperature(temperature)
        .num_ctx(4096); // Context window

    // Build generation request
    let request = GenerationRequest::new(model.to_string(), prompt).options(options);

    // Execute with timeout
    let result = tokio::time::timeout(Duration::from_secs(timeout_secs), async {
        ollama
            .generate(request)
            .await
            .map_err(|e| AppError::LlmUnavailable(e.to_string()))
    })
    .await;

    match result {
        Ok(Ok(response)) => Ok(response.response.trim().to_string()),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(AppError::LlmTimeout(timeout_secs)),
    }
}

/// Generate bullet-list fallback when LLM is unavailable
pub fn generate_bullet_fallback(data: &AggregatedData, user_fields: &SummaryInput) -> String {
    let mut lines = Vec::new();

    // Tickets closed
    if !data.tickets_closed.is_empty() {
        lines.push(format!(
            "**Tickets Closed ({}):** {}",
            data.tickets_closed.len(),
            data.tickets_closed
                .iter()
                .map(|t| format!("{} ({})", t.id, t.title))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    // Tickets in progress
    if !data.tickets_in_progress.is_empty() {
        lines.push(format!(
            "**In Progress ({}):** {}",
            data.tickets_in_progress.len(),
            data.tickets_in_progress
                .iter()
                .map(|t| format!("{} ({})", t.id, t.title))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    // Meetings
    if !data.meetings.is_empty() {
        let total_meeting_minutes: i32 = data.meetings.iter().map(|m| m.duration_minutes).sum();
        lines.push(format!(
            "**Meetings ({}, {}m total):** {}",
            data.meetings.len(),
            total_meeting_minutes,
            data.meetings
                .iter()
                .map(|m| format!("{} ({}m)", m.title, m.duration_minutes))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    // Focus time
    if data.focus_hours > 0.0 {
        lines.push(format!("**Focus Time:** {:.1} hours", data.focus_hours));
    }

    // Blockers
    if let Some(blockers) = &user_fields.blockers {
        if !blockers.is_empty() {
            lines.push(format!("**Blockers:** {}", blockers));
        }
    }

    // Tomorrow's priorities
    if let Some(priorities) = &user_fields.tomorrow_priorities {
        if !priorities.is_empty() {
            lines.push(format!("**Tomorrow:** {}", priorities));
        }
    }

    if lines.is_empty() {
        "No activity recorded for today.".to_string()
    } else {
        lines.join("\n\n")
    }
}
