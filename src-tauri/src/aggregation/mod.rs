pub mod calendar;
pub mod jira;
pub mod toggl;

use crate::error::AppError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Ticket {
    pub id: String,
    pub title: String,
    pub status: String,
    pub url: String,
    #[serde(rename = "resolvedAt")]
    pub resolved_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Meeting {
    pub title: String,
    pub start: String,
    pub end: String,
    #[serde(rename = "durationMinutes")]
    pub duration_minutes: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "status")]
pub enum SourceStatusDetail {
    Ok {
        #[serde(rename = "fetchedAt")]
        fetched_at: String,
    },
    Failed {
        error: String,
    },
    NotConfigured,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataSourcesStatus {
    pub jira: SourceStatusDetail,
    pub calendar: SourceStatusDetail,
    pub toggl: SourceStatusDetail,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AggregatedData {
    #[serde(rename = "ticketsClosed")]
    pub tickets_closed: Vec<Ticket>,
    #[serde(rename = "ticketsInProgress")]
    pub tickets_in_progress: Vec<Ticket>,
    pub meetings: Vec<Meeting>,
    #[serde(rename = "focusHours")]
    pub focus_hours: f32,
    #[serde(rename = "dataSourcesStatus")]
    pub data_sources_status: DataSourcesStatus,
}

/// Main aggregation function - fetches from all sources in parallel
pub async fn aggregate_today(
    jira_base_url: Option<String>,
    jira_email: Option<String>,
    jira_api_token: Option<String>,
    jira_project_key: Option<String>,
    calendar_access_token: Option<String>,
    toggl_api_token: Option<String>,
    toggl_workspace_id: Option<String>,
) -> AggregatedData {
    let now = chrono::Local::now().to_rfc3339();

    // Fetch all sources in parallel
    let (jira_result, calendar_result, toggl_result) = tokio::join!(
        async {
            if let (Some(url), Some(email), Some(token), Some(project)) = (
                jira_base_url.as_ref(),
                jira_email.as_ref(),
                jira_api_token.as_ref(),
                jira_project_key.as_ref(),
            ) {
                jira::fetch_tickets_today(url, email, token, project).await
            } else {
                Err(AppError::NotConfigured("Jira not configured".to_string()))
            }
        },
        async {
            if let Some(token) = calendar_access_token.as_ref() {
                calendar::fetch_events_today(token).await
            } else {
                Err(AppError::NotConfigured(
                    "Calendar not configured".to_string(),
                ))
            }
        },
        async {
            if let (Some(token), Some(workspace)) =
                (toggl_api_token.as_ref(), toggl_workspace_id.as_ref())
            {
                toggl::fetch_focus_hours_today(token, workspace).await
            } else {
                Err(AppError::NotConfigured("Toggl not configured".to_string()))
            }
        }
    );

    // Process Jira result
    let (tickets_closed, tickets_in_progress, jira_status) = match jira_result {
        Ok((closed, in_progress)) => (
            closed,
            in_progress,
            SourceStatusDetail::Ok {
                fetched_at: now.clone(),
            },
        ),
        Err(AppError::NotConfigured(_)) => (vec![], vec![], SourceStatusDetail::NotConfigured),
        Err(e) => (
            vec![],
            vec![],
            SourceStatusDetail::Failed {
                error: e.to_string(),
            },
        ),
    };

    // Process Calendar result
    let (meetings, calendar_status) = match calendar_result {
        Ok(events) => (
            events,
            SourceStatusDetail::Ok {
                fetched_at: now.clone(),
            },
        ),
        Err(AppError::NotConfigured(_)) => (vec![], SourceStatusDetail::NotConfigured),
        Err(e) => (
            vec![],
            SourceStatusDetail::Failed {
                error: e.to_string(),
            },
        ),
    };

    // Process Toggl result
    let (focus_hours, toggl_status) = match toggl_result {
        Ok(hours) => (
            hours,
            SourceStatusDetail::Ok {
                fetched_at: now.clone(),
            },
        ),
        Err(AppError::NotConfigured(_)) => (0.0, SourceStatusDetail::NotConfigured),
        Err(e) => (
            0.0,
            SourceStatusDetail::Failed {
                error: e.to_string(),
            },
        ),
    };

    AggregatedData {
        tickets_closed,
        tickets_in_progress,
        meetings,
        focus_hours,
        data_sources_status: DataSourcesStatus {
            jira: jira_status,
            calendar: calendar_status,
            toggl: toggl_status,
        },
    }
}
