use crate::error::AppError;
use chrono::Local;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct TogglTimeEntry {
    start: String,
    duration: i64, // seconds, negative if timer is running
}

/// Fetch focus hours from Toggl Track for today
pub async fn fetch_focus_hours_today(
    api_token: &str,
    _workspace_id: &str, // Not used in v9 API for time entries endpoint
) -> Result<f32, AppError> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::NotConfigured(format!("HTTP client error: {}", e)))?;

    // Get today's date range
    let today = Local::now().date_naive();
    let today_start = format!("{}T00:00:00Z", today);
    let tomorrow = today
        .succ_opt()
        .ok_or_else(|| AppError::TogglError("Cannot calculate tomorrow's date".to_string()))?;
    let tomorrow_start = format!("{}T00:00:00Z", tomorrow);

    let url = format!(
        "https://api.track.toggl.com/api/v9/me/time_entries?start_date={}&end_date={}",
        urlencoding::encode(&today_start),
        urlencoding::encode(&tomorrow_start)
    );

    // Toggl uses Basic auth with api_token:api_token
    let auth = format!("{}:{}", api_token, api_token);
    let auth_b64 =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, auth.as_bytes());

    let response = client
        .get(&url)
        .header("Authorization", format!("Basic {}", auth_b64))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                AppError::NetworkTimeout("Toggl Track request timed out".to_string())
            } else if e.is_connect() {
                AppError::TogglError(
                    "Cannot reach Toggl Track API. Check your internet connection.".to_string(),
                )
            } else {
                AppError::TogglError(format!("Request failed: {}", e))
            }
        })?;

    let status = response.status();
    if status == 403 || status == 401 {
        return Err(AppError::TogglError(
            "Authentication failed. Check your API token in Settings.".to_string(),
        ));
    } else if !status.is_success() {
        return Err(AppError::TogglError(format!(
            "Toggl Track API returned error: HTTP {}",
            status
        )));
    }

    let entries: Vec<TogglTimeEntry> = response
        .json()
        .await
        .map_err(|e| AppError::TogglError(format!("Failed to parse time entries: {}", e)))?;

    // Sum durations
    let now = chrono::Utc::now().timestamp();
    let mut total_seconds: i64 = 0;

    for entry in entries {
        if entry.duration < 0 {
            // Timer is running, calculate current duration
            let start_timestamp = chrono::DateTime::parse_from_rfc3339(&entry.start)
                .map(|dt| dt.timestamp())
                .unwrap_or(0);
            total_seconds += now - start_timestamp;
        } else {
            total_seconds += entry.duration;
        }
    }

    // Convert to hours and cap at 24
    let hours = (total_seconds as f32) / 3600.0;
    let capped_hours = hours.min(24.0);

    Ok(capped_hours)
}
