use crate::aggregation::Meeting;
use crate::error::AppError;
use chrono::Local;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct CalendarResponse {
    items: Vec<CalendarEvent>,
}

#[derive(Debug, Deserialize)]
struct CalendarEvent {
    summary: Option<String>,
    start: EventDateTime,
    end: EventDateTime,
    attendees: Option<Vec<Attendee>>,
}

#[derive(Debug, Deserialize)]
struct EventDateTime {
    #[serde(rename = "dateTime")]
    date_time: Option<String>,
    date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Attendee {
    #[serde(rename = "responseStatus")]
    response_status: Option<String>,
}

/// Fetch today's calendar events from Google Calendar
pub async fn fetch_events_today(access_token: &str) -> Result<Vec<Meeting>, AppError> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|_e| {
            AppError::CalendarUnauthorized // Treat build errors as auth issues
        })?;

    // Get today's start and end in UTC
    let today = Local::now().date_naive();
    let start_of_day = today
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| AppError::CalendarError("Cannot create start of day timestamp".to_string()))?
        .and_local_timezone(Local)
        .earliest()
        .ok_or_else(|| AppError::CalendarError("Cannot convert start of day to local timezone".to_string()))?
        .to_rfc3339();
    let end_of_day = today
        .and_hms_opt(23, 59, 59)
        .ok_or_else(|| AppError::CalendarError("Cannot create end of day timestamp".to_string()))?
        .and_local_timezone(Local)
        .earliest()
        .ok_or_else(|| AppError::CalendarError("Cannot convert end of day to local timezone".to_string()))?
        .to_rfc3339();

    let url = format!(
        "https://www.googleapis.com/calendar/v3/calendars/primary/events?timeMin={}&timeMax={}&singleEvents=true&maxResults=50",
        urlencoding::encode(&start_of_day),
        urlencoding::encode(&end_of_day)
    );

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                AppError::NetworkTimeout("Google Calendar request timed out".to_string())
            } else if e.is_connect() {
                AppError::CalendarError("Cannot reach Google Calendar API. Check your internet connection.".to_string())
            } else {
                AppError::CalendarError(format!("Request failed: {}", e))
            }
        })?;

    let status = response.status();
    if status == 401 || status == 403 {
        return Err(AppError::CalendarUnauthorized);
    } else if !status.is_success() {
        return Err(AppError::CalendarError(format!(
            "Google Calendar API returned error: HTTP {}",
            status
        )));
    }

    let calendar_response: CalendarResponse = response.json().await.map_err(|e| {
        AppError::CalendarError(format!("Failed to parse calendar response: {}", e))
    })?;

    let mut meetings = Vec::new();

    for event in calendar_response.items {
        // Skip all-day events (they have date but not dateTime)
        if event.start.date_time.is_none() {
            continue;
        }

        // Skip declined events
        if let Some(attendees) = &event.attendees {
            let user_declined = attendees.iter().any(|a| {
                a.response_status
                    .as_ref()
                    .map(|s| s == "declined")
                    .unwrap_or(false)
            });
            if user_declined {
                continue;
            }
        }

        let start = event.start.date_time.ok_or_else(|| {
            AppError::CalendarError("Event missing start dateTime".to_string())
        })?;
        let end = event.end.date_time.ok_or_else(|| {
            AppError::CalendarError("Event missing end dateTime".to_string())
        })?;

        // Calculate duration
        let start_dt = chrono::DateTime::parse_from_rfc3339(&start).ok();
        let end_dt = chrono::DateTime::parse_from_rfc3339(&end).ok();
        let duration_minutes = if let (Some(s), Some(e)) = (start_dt, end_dt) {
            (e.timestamp() - s.timestamp()) / 60
        } else {
            0
        };

        meetings.push(Meeting {
            title: event
                .summary
                .unwrap_or_else(|| "Untitled meeting".to_string()),
            start,
            end,
            duration_minutes: duration_minutes as i32,
        });
    }

    Ok(meetings)
}
