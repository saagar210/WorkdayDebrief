use crate::aggregation::Ticket;
use crate::error::AppError;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct JiraResponse {
    issues: Vec<JiraIssue>,
}

#[derive(Debug, Deserialize)]
struct JiraIssue {
    key: String,
    fields: JiraFields,
}

#[derive(Debug, Deserialize)]
struct JiraFields {
    summary: String,
    status: JiraStatus,
    resolutiondate: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JiraStatus {
    name: String,
}

/// Fetch tickets updated today from Jira
/// Returns (tickets_closed, tickets_in_progress)
pub async fn fetch_tickets_today(
    base_url: &str,
    email: &str,
    api_token: &str,
    project_key: &str,
) -> Result<(Vec<Ticket>, Vec<Ticket>), AppError> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::JiraUnreachable(format!("Failed to create HTTP client: {}", e)))?;

    // JQL: assignee = currentUser() AND updated >= startOfDay() ORDER BY updated DESC
    let jql = format!(
        "assignee = currentUser() AND project = {} AND updated >= startOfDay() ORDER BY updated DESC",
        project_key
    );

    let url = format!(
        "{}/rest/api/2/search?jql={}&fields=summary,status,resolutiondate",
        base_url,
        urlencoding::encode(&jql)
    );

    // Basic auth with email and API token
    let auth = format!("{}:{}", email, api_token);
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
                AppError::JiraUnreachable("Request timed out (>10s)".to_string())
            } else if e.is_connect() {
                AppError::JiraUnreachable("Cannot connect to Jira server".to_string())
            } else {
                AppError::JiraUnreachable(e.to_string())
            }
        })?;

    // Check status code
    let status = response.status();
    if status == 401 {
        return Err(AppError::JiraUnreachable(
            "Authentication failed - check API token".to_string(),
        ));
    } else if status == 403 {
        return Err(AppError::JiraUnreachable(
            "No access to project - check project key".to_string(),
        ));
    } else if !status.is_success() {
        return Err(AppError::JiraUnreachable(format!("HTTP error: {}", status)));
    }

    let jira_response: JiraResponse = response
        .json()
        .await
        .map_err(|e| AppError::JiraUnreachable(format!("Failed to parse JSON response: {}", e)))?;

    // Split into closed vs in-progress
    let mut tickets_closed = Vec::new();
    let mut tickets_in_progress = Vec::new();

    for issue in jira_response.issues {
        let ticket = Ticket {
            id: issue.key.clone(),
            title: issue.fields.summary.clone(),
            status: issue.fields.status.name.clone(),
            url: format!("{}/browse/{}", base_url, issue.key),
            resolved_at: issue.fields.resolutiondate.clone(),
        };

        // Determine if closed or in-progress
        let status_lower = issue.fields.status.name.to_lowercase();
        if status_lower.contains("done")
            || status_lower.contains("closed")
            || status_lower.contains("resolved")
        {
            tickets_closed.push(ticket);
        } else {
            tickets_in_progress.push(ticket);
        }
    }

    Ok((tickets_closed, tickets_in_progress))
}
