use crate::db::queries;
use crate::error::AppError;
use chrono::Local;
use serde::Deserialize;
use sqlx::{Row, SqlitePool};
use tauri::{AppHandle, State};

// ── Phase 0-1: Core ──

#[derive(Debug, Deserialize)]
pub struct SummaryInput {
    pub blockers: Option<String>,
    #[serde(rename = "tomorrowPriorities")]
    pub tomorrow_priorities: Option<String>,
    #[serde(rename = "manualNotes")]
    pub manual_notes: Option<String>,
    pub narrative: Option<String>,
    pub tone: Option<String>,
}

#[tauri::command]
pub async fn get_today_summary(
    db: State<'_, SqlitePool>,
) -> Result<Option<serde_json::Value>, AppError> {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let summary = queries::get_summary_by_date(&db, &today).await?;
    Ok(summary)
}

#[tauri::command]
pub async fn save_summary(
    db: State<'_, SqlitePool>,
    input: SummaryInput,
) -> Result<serde_json::Value, AppError> {
    let today = Local::now().format("%Y-%m-%d").to_string();

    // Upsert the summary
    let _id = queries::upsert_summary(
        &db,
        &today,
        input.blockers.as_deref(),
        input.tomorrow_priorities.as_deref(),
        input.manual_notes.as_deref(),
        input.narrative.as_deref(),
        input.tone.as_deref(),
    )
    .await?;

    // Fetch and return the updated summary
    let summary = queries::get_summary_by_date(&db, &today)
        .await?
        .ok_or_else(|| AppError::DatabaseError("Failed to retrieve saved summary".to_string()))?;

    Ok(summary)
}

#[tauri::command]
pub async fn list_summaries(
    db: State<'_, SqlitePool>,
    days_back: i32,
) -> Result<Vec<serde_json::Value>, AppError> {
    let metas = queries::list_summary_metas(&db, days_back).await?;
    Ok(metas)
}

#[tauri::command]
pub async fn get_summary_by_date(
    db: State<'_, SqlitePool>,
    date: String,
) -> Result<Option<serde_json::Value>, AppError> {
    let summary = queries::get_summary_by_date(&db, &date).await?;
    Ok(summary)
}

// ── Phase 2: Aggregation ──

#[tauri::command]
pub async fn generate_summary(
    db: State<'_, SqlitePool>,
    app: AppHandle,
) -> Result<serde_json::Value, AppError> {
    let today = Local::now().format("%Y-%m-%d").to_string();

    // Load settings from database
    let settings_row = sqlx::query(
        r#"
        SELECT jira_base_url, jira_project_key, toggl_workspace_id
        FROM settings WHERE id = 1
        "#,
    )
    .fetch_one(db.inner())
    .await?;

    let jira_base_url: Option<String> = settings_row.get("jira_base_url");
    let jira_project_key: Option<String> = settings_row.get("jira_project_key");
    let toggl_workspace_id: Option<String> = settings_row.get("toggl_workspace_id");

    // Load secrets from encrypted storage
    let jira_email = crate::stronghold::get_secret(&app, crate::stronghold::keys::JIRA_EMAIL)?;
    let jira_api_token =
        crate::stronghold::get_secret(&app, crate::stronghold::keys::JIRA_API_TOKEN)?;
    let toggl_api_token =
        crate::stronghold::get_secret(&app, crate::stronghold::keys::TOGGL_API_TOKEN)?;

    // Get Google Calendar access token (refresh if needed)
    let calendar_access_token = if let Some(refresh_token) =
        crate::stronghold::get_secret(&app, crate::stronghold::keys::GOOGLE_REFRESH_TOKEN)?
    {
        // Refresh the access token
        let client_id = std::env::var("GOOGLE_CLIENT_ID")
            .unwrap_or_else(|_| "YOUR_CLIENT_ID.apps.googleusercontent.com".to_string());
        let client_secret = std::env::var("GOOGLE_CLIENT_SECRET")
            .unwrap_or_else(|_| "YOUR_CLIENT_SECRET".to_string());

        match crate::oauth::GoogleOAuthClient::new(client_id, client_secret) {
            Ok(oauth_client) => oauth_client.refresh_access_token(refresh_token).await.ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    // Aggregate data from all sources
    let aggregated_data = crate::aggregation::aggregate_today(
        jira_base_url,
        jira_email,
        jira_api_token,
        jira_project_key,
        calendar_access_token,
        toggl_api_token,
        toggl_workspace_id,
    )
    .await;

    // Convert aggregated data to JSON strings for storage
    let tickets_closed_json = serde_json::to_string(&aggregated_data.tickets_closed)
        .map_err(|e| AppError::DatabaseError(format!("Cannot serialize tickets_closed: {}", e)))?;
    let tickets_in_progress_json = serde_json::to_string(&aggregated_data.tickets_in_progress)
        .map_err(|e| AppError::DatabaseError(format!("Cannot serialize tickets_in_progress: {}", e)))?;
    let meetings_json = serde_json::to_string(&aggregated_data.meetings)
        .map_err(|e| AppError::DatabaseError(format!("Cannot serialize meetings: {}", e)))?;
    let sources_status_json = serde_json::to_string(&aggregated_data.data_sources_status)
        .map_err(|e| AppError::DatabaseError(format!("Cannot serialize sources_status: {}", e)))?;

    // Insert/update in database
    sqlx::query(
        r#"
        INSERT INTO daily_summaries (
            summary_date,
            tickets_closed,
            tickets_in_progress,
            meetings,
            focus_hours,
            sources_status
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(summary_date) DO UPDATE SET
            tickets_closed = ?2,
            tickets_in_progress = ?3,
            meetings = ?4,
            focus_hours = ?5,
            sources_status = ?6,
            updated_at = datetime('now')
        "#,
    )
    .bind(&today)
    .bind(&tickets_closed_json)
    .bind(&tickets_in_progress_json)
    .bind(&meetings_json)
    .bind(aggregated_data.focus_hours)
    .bind(&sources_status_json)
    .execute(db.inner())
    .await?;

    // Fetch and return the updated summary
    let summary = queries::get_summary_by_date(&db, &today)
        .await?
        .ok_or_else(|| {
            AppError::DatabaseError("Failed to retrieve generated summary".to_string())
        })?;

    Ok(summary)
}

// ── Phase 3: LLM ──

#[tauri::command]
pub async fn regenerate_narrative(
    db: State<'_, SqlitePool>,
    summary_id: i64,
    tone: String,
) -> Result<String, AppError> {
    // Load the summary from database
    let row = sqlx::query(
        r#"
        SELECT summary_date, tickets_closed, tickets_in_progress, meetings, focus_hours,
               blockers, tomorrow_priorities, manual_notes
        FROM daily_summaries
        WHERE id = ?1
        "#,
    )
    .bind(summary_id)
    .fetch_optional(db.inner())
    .await?;

    let row = row.ok_or_else(|| {
        AppError::DatabaseError(format!("Summary {} not found", summary_id))
    })?;

    // Parse JSON fields - log errors but use defaults to avoid breaking regeneration
    let tickets_closed_str: String = row.get("tickets_closed");
    let tickets_closed: Vec<crate::aggregation::Ticket> =
        serde_json::from_str(&tickets_closed_str).unwrap_or_else(|e| {
            eprintln!("Warning: Failed to parse tickets_closed for summary {}: {}", summary_id, e);
            Vec::new()
        });

    let tickets_in_progress_str: String = row.get("tickets_in_progress");
    let tickets_in_progress: Vec<crate::aggregation::Ticket> =
        serde_json::from_str(&tickets_in_progress_str).unwrap_or_else(|e| {
            eprintln!("Warning: Failed to parse tickets_in_progress for summary {}: {}", summary_id, e);
            Vec::new()
        });

    let meetings_str: String = row.get("meetings");
    let meetings: Vec<crate::aggregation::Meeting> =
        serde_json::from_str(&meetings_str).unwrap_or_else(|e| {
            eprintln!("Warning: Failed to parse meetings for summary {}: {}", summary_id, e);
            Vec::new()
        });
    let focus_hours: f32 = row.get("focus_hours");

    // Build AggregatedData from stored data
    let aggregated_data = crate::aggregation::AggregatedData {
        tickets_closed,
        tickets_in_progress,
        meetings,
        focus_hours,
        data_sources_status: crate::aggregation::DataSourcesStatus {
            jira: crate::aggregation::SourceStatusDetail::NotConfigured,
            calendar: crate::aggregation::SourceStatusDetail::NotConfigured,
            toggl: crate::aggregation::SourceStatusDetail::NotConfigured,
        },
    };

    // Build user fields
    let user_fields = SummaryInput {
        blockers: Some(row.get::<String, _>("blockers")),
        tomorrow_priorities: Some(row.get::<String, _>("tomorrow_priorities")),
        manual_notes: Some(row.get::<String, _>("manual_notes")),
        narrative: None,
        tone: Some(tone.clone()),
    };

    // LLM settings - use defaults for now (Phase 4 will load from settings table)
    let model = "qwen3:14b";
    let temperature = 0.7;
    let timeout_secs = 15;

    // Try to generate narrative with LLM
    let narrative = match crate::llm::generate_narrative(
        &aggregated_data,
        &user_fields,
        &tone,
        model,
        temperature,
        timeout_secs,
    )
    .await
    {
        Ok(text) => text,
        Err(e) => {
            // LLM failed, use bullet fallback
            eprintln!("LLM generation failed: {}. Using bullet fallback.", e);
            crate::llm::generate_bullet_fallback(&aggregated_data, &user_fields)
        }
    };

    // Update narrative in database
    sqlx::query(
        r#"
        UPDATE daily_summaries
        SET narrative = ?1, tone = ?2
        WHERE id = ?3
        "#,
    )
    .bind(&narrative)
    .bind(&tone)
    .bind(summary_id)
    .execute(db.inner())
    .await?;

    Ok(narrative)
}

// ── Phase 4: Delivery ──

#[derive(Debug, serde::Deserialize)]
pub struct DeliveryConfigInput {
    #[serde(rename = "deliveryType")]
    delivery_type: String,
    config: serde_json::Map<String, serde_json::Value>,
    #[serde(rename = "isEnabled")]
    is_enabled: bool,
}

#[tauri::command]
pub async fn send_summary(
    db: State<'_, SqlitePool>,
    summary_id: i64,
    delivery_configs: Vec<DeliveryConfigInput>,
    app: AppHandle,
) -> Result<Vec<crate::delivery::DeliveryConfirmation>, AppError> {
    // Convert frontend configs to backend enum format, injecting secrets
    let mut backend_configs: Vec<crate::delivery::DeliveryConfig> = Vec::new();

    for input in delivery_configs {
        let mut config_map = input.config;

        // Inject secrets from vault
        if input.delivery_type == "email" {
            if let Some(password) = crate::stronghold::get_secret(&app, "delivery_email_password")? {
                config_map.insert("password".to_string(), serde_json::Value::String(password));
            }

            // Convert to enum variant
            let json_value = serde_json::Value::Object(config_map);
            if let Ok(email_config) = serde_json::from_value(json_value) {
                backend_configs.push(crate::delivery::DeliveryConfig::Email(email_config));
            }
        } else if input.delivery_type == "slack" {
            if let Some(webhook) = crate::stronghold::get_secret(&app, "delivery_slack_webhook")? {
                config_map.insert("webhookUrl".to_string(), serde_json::Value::String(webhook));
            }

            // Convert to enum variant
            let json_value = serde_json::Value::Object(config_map);
            if let Ok(slack_config) = serde_json::from_value(json_value) {
                backend_configs.push(crate::delivery::DeliveryConfig::Slack(slack_config));
            }
        } else if input.delivery_type == "file" {
            // Convert to enum variant
            let json_value = serde_json::Value::Object(config_map);
            if let Ok(file_config) = serde_json::from_value(json_value) {
                backend_configs.push(crate::delivery::DeliveryConfig::File(file_config));
            }
        }
    }
    // Load summary from database
    let row = sqlx::query(
        r#"
        SELECT summary_date, tickets_closed, tickets_in_progress, meetings, focus_hours,
               blockers, tomorrow_priorities, manual_notes, narrative, delivered_to
        FROM daily_summaries
        WHERE id = ?1
        "#,
    )
    .bind(summary_id)
    .fetch_optional(db.inner())
    .await?;

    let row = row.ok_or_else(|| {
        AppError::DatabaseError(format!("Summary {} not found", summary_id))
    })?;

    // Parse data - log errors but use defaults to avoid breaking delivery
    let tickets_closed_str: String = row.get("tickets_closed");
    let tickets_closed: Vec<crate::aggregation::Ticket> =
        serde_json::from_str(&tickets_closed_str).unwrap_or_else(|e| {
            eprintln!("Warning: Failed to parse tickets_closed for summary {}: {}", summary_id, e);
            Vec::new()
        });

    let tickets_in_progress_str: String = row.get("tickets_in_progress");
    let tickets_in_progress: Vec<crate::aggregation::Ticket> =
        serde_json::from_str(&tickets_in_progress_str).unwrap_or_else(|e| {
            eprintln!("Warning: Failed to parse tickets_in_progress for summary {}: {}", summary_id, e);
            Vec::new()
        });

    let meetings_str: String = row.get("meetings");
    let meetings: Vec<crate::aggregation::Meeting> =
        serde_json::from_str(&meetings_str).unwrap_or_else(|e| {
            eprintln!("Warning: Failed to parse meetings for summary {}: {}", summary_id, e);
            Vec::new()
        });
    let focus_hours: f32 = row.get("focus_hours");
    let blockers: String = row.get("blockers");
    let tomorrow_priorities: String = row.get("tomorrow_priorities");
    let manual_notes: String = row.get("manual_notes");
    let narrative: String = row.get("narrative");
    let summary_date: String = row.get("summary_date");

    // Render to markdown
    let markdown = crate::markdown::render_summary_to_markdown(
        &summary_date,
        &narrative,
        &tickets_closed,
        &tickets_in_progress,
        &meetings,
        focus_hours,
        &blockers,
        &tomorrow_priorities,
        &manual_notes,
    );

    // Send to all targets
    let confirmations =
        crate::delivery::send_summary(&markdown, &summary_date, backend_configs).await;

    // Update delivered_to field with successful deliveries
    let successful_deliveries: Vec<String> = confirmations
        .iter()
        .filter(|c| c.success)
        .map(|c| c.delivery_type.clone())
        .collect();

    if !successful_deliveries.is_empty() {
        let mut current_delivered: Vec<String> =
            serde_json::from_str(&row.get::<String, _>("delivered_to")).unwrap_or_default();
        current_delivered.extend(successful_deliveries);
        current_delivered.sort();
        current_delivered.dedup();

        let delivered_json = serde_json::to_string(&current_delivered).unwrap_or_default();

        sqlx::query(
            r#"
            UPDATE daily_summaries
            SET delivered_to = ?1
            WHERE id = ?2
            "#,
        )
        .bind(&delivered_json)
        .bind(summary_id)
        .execute(db.inner())
        .await?;
    }

    Ok(confirmations)
}

#[tauri::command]
pub async fn test_delivery(
    delivery_type: String,
    config: crate::delivery::DeliveryConfig,
) -> Result<String, AppError> {
    let test_markdown = "# Test Summary\n\nThis is a test delivery from WorkdayDebrief.";
    let test_date = "2026-02-14";

    let confirmations =
        crate::delivery::send_summary(test_markdown, test_date, vec![config]).await;

    if let Some(confirmation) = confirmations.first() {
        if confirmation.success {
            Ok(format!(
                "{} delivery test successful: {}",
                delivery_type, confirmation.message
            ))
        } else {
            Err(AppError::NotConfigured(format!(
                "{} test failed: {}",
                delivery_type, confirmation.message
            )))
        }
    } else {
        Err(AppError::NotConfigured("No confirmation received".to_string()))
    }
}

// ── Settings ──

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub scheduled_time: String,       // "17:00"
    pub default_tone: String,         // "professional", "casual", "detailed"
    pub enable_llm: bool,
    pub llm_model: String,            // "qwen3:14b"
    pub llm_temperature: f32,         // 0.0-1.0
    pub llm_timeout_secs: u64,        // 5-30
    pub calendar_source: String,      // "google", "none"
    pub retention_days: i32,          // 7-365
    pub jira_base_url: Option<String>,
    pub jira_project_key: Option<String>,
    pub toggl_workspace_id: Option<String>,
}

#[tauri::command]
pub async fn get_settings(db: State<'_, SqlitePool>) -> Result<Settings, AppError> {
    let row = sqlx::query(
        r#"
        SELECT scheduled_time, default_tone, enable_llm, llm_model, llm_temperature,
               llm_timeout_secs, calendar_source, retention_days, jira_base_url,
               jira_project_key, toggl_workspace_id
        FROM settings
        WHERE id = 1
        "#,
    )
    .fetch_one(db.inner())
    .await?;

    Ok(Settings {
        scheduled_time: row.get("scheduled_time"),
        default_tone: row.get("default_tone"),
        enable_llm: row.get::<i32, _>("enable_llm") != 0,
        llm_model: row.get("llm_model"),
        llm_temperature: row.get("llm_temperature"),
        llm_timeout_secs: row.get::<i32, _>("llm_timeout_secs") as u64,
        calendar_source: row.get("calendar_source"),
        retention_days: row.get("retention_days"),
        jira_base_url: row.get("jira_base_url"),
        jira_project_key: row.get("jira_project_key"),
        toggl_workspace_id: row.get("toggl_workspace_id"),
    })
}

#[tauri::command]
pub async fn save_settings(
    db: State<'_, SqlitePool>,
    settings: Settings,
    app: AppHandle,
) -> Result<Settings, AppError> {
    // Validate inputs
    let parts: Vec<&str> = settings.scheduled_time.split(':').collect();
    if parts.len() != 2 {
        return Err(AppError::NotConfigured("Invalid time format. Use HH:MM".to_string()));
    }
    let hour: u32 = parts[0].parse().map_err(|_| AppError::NotConfigured("Invalid hour".to_string()))?;
    let minute: u32 = parts[1].parse().map_err(|_| AppError::NotConfigured("Invalid minute".to_string()))?;
    if hour > 23 || minute > 59 {
        return Err(AppError::NotConfigured("Hour must be 0-23, minute must be 0-59".to_string()));
    }

    if !(0.0..=1.0).contains(&settings.llm_temperature) {
        return Err(AppError::NotConfigured("Temperature must be 0.0-1.0".to_string()));
    }

    if !(5..=30).contains(&settings.llm_timeout_secs) {
        return Err(AppError::NotConfigured("Timeout must be 5-30 seconds".to_string()));
    }

    if settings.retention_days < 7 || settings.retention_days > 365 {
        return Err(AppError::NotConfigured("Retention days must be 7-365".to_string()));
    }

    // Update settings
    sqlx::query(
        r#"
        UPDATE settings
        SET scheduled_time = ?1,
            default_tone = ?2,
            enable_llm = ?3,
            llm_model = ?4,
            llm_temperature = ?5,
            llm_timeout_secs = ?6,
            calendar_source = ?7,
            retention_days = ?8,
            jira_base_url = ?9,
            jira_project_key = ?10,
            toggl_workspace_id = ?11,
            updated_at = datetime('now')
        WHERE id = 1
        "#,
    )
    .bind(&settings.scheduled_time)
    .bind(&settings.default_tone)
    .bind(if settings.enable_llm { 1 } else { 0 })
    .bind(&settings.llm_model)
    .bind(settings.llm_temperature)
    .bind(settings.llm_timeout_secs as i32)
    .bind(&settings.calendar_source)
    .bind(settings.retention_days)
    .bind(&settings.jira_base_url)
    .bind(&settings.jira_project_key)
    .bind(&settings.toggl_workspace_id)
    .execute(db.inner())
    .await?;

    // Restart scheduler with new scheduled_time
    use tauri::Manager;
    type SchedulerStateType = std::sync::Arc<tokio::sync::Mutex<crate::scheduler::SchedulerState>>;

    if let Some(scheduler_state) = app.try_state::<SchedulerStateType>() {
        let state_arc: SchedulerStateType = scheduler_state.inner().clone();

        // Stop existing scheduler
        let _ = crate::scheduler::stop_scheduler(state_arc.clone()).await;

        // Start with new time
        if let Err(e) = crate::scheduler::start_scheduler(
            app.clone(),
            settings.scheduled_time.clone(),
            state_arc,
        )
        .await
        {
            eprintln!("[Settings] Failed to restart scheduler: {}", e);
        } else {
            eprintln!("[Settings] Scheduler restarted with time: {}", settings.scheduled_time);
        }
    }

    Ok(settings)
}

// ── Delivery Config ──

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct DeliveryConfigRow {
    pub id: i64,
    pub delivery_type: String,  // "email", "slack", "file"
    pub config: serde_json::Value,  // JSON blob with type-specific config
    pub is_enabled: bool,
}

#[tauri::command]
pub async fn get_delivery_configs(
    db: State<'_, SqlitePool>,
    app: AppHandle,
) -> Result<Vec<DeliveryConfigRow>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT id, delivery_type, config, is_enabled
        FROM delivery_configs
        WHERE is_enabled = 1
        ORDER BY delivery_type
        "#,
    )
    .fetch_all(db.inner())
    .await?;

    let mut configs = Vec::new();
    for row in rows {
        let config_str: String = row.get("config");
        let mut config: serde_json::Value = serde_json::from_str(&config_str)
            .unwrap_or_else(|_| serde_json::json!({}));

        let delivery_type: String = row.get("delivery_type");

        // Add secrets from vault (masked for display)
        if let Some(obj) = config.as_object_mut() {
            if delivery_type == "email" {
                // Check if password exists in vault
                if crate::stronghold::get_secret(&app, "delivery_email_password")?.is_some() {
                    obj.insert("password".to_string(), serde_json::Value::String("••••••".to_string()));
                }
            }

            if delivery_type == "slack" {
                // Check if webhook exists in vault
                if crate::stronghold::get_secret(&app, "delivery_slack_webhook")?.is_some() {
                    obj.insert("webhookUrl".to_string(), serde_json::Value::String("••••••".to_string()));
                }
            }
        }

        configs.push(DeliveryConfigRow {
            id: row.get("id"),
            delivery_type,
            config,
            is_enabled: row.get::<i32, _>("is_enabled") != 0,
        });
    }

    Ok(configs)
}

#[derive(Debug, serde::Deserialize)]
pub struct SaveDeliveryConfigInput {
    pub delivery_type: String,
    pub config: serde_json::Value,
    pub is_enabled: bool,
}

#[tauri::command]
pub async fn save_delivery_config(
    db: State<'_, SqlitePool>,
    input: SaveDeliveryConfigInput,
    app: AppHandle,
) -> Result<(), AppError> {
    // Validate delivery_type
    if !["email", "slack", "file"].contains(&input.delivery_type.as_str()) {
        return Err(AppError::NotConfigured("Invalid delivery type".to_string()));
    }

    // Extract secrets and store in encrypted vault
    let mut final_config = input.config.clone();

    if let Some(obj) = final_config.as_object_mut() {
        // Extract and store SMTP password
        if input.delivery_type == "email" {
            if let Some(password) = obj.get("password").and_then(|v| v.as_str()) {
                // If not masked, store in vault
                if password != "••••••" {
                    crate::stronghold::store_secret(
                        &app,
                        "delivery_email_password",
                        password,
                    )?;
                }
                // Remove from config JSON
                obj.remove("password");
            }
        }

        // Extract and store Slack webhook URL
        if input.delivery_type == "slack" {
            if let Some(webhook) = obj.get("webhookUrl").and_then(|v| v.as_str()) {
                // If not masked, store in vault
                if webhook != "••••••" {
                    crate::stronghold::store_secret(
                        &app,
                        "delivery_slack_webhook",
                        webhook,
                    )?;
                }
                // Remove from config JSON
                obj.remove("webhookUrl");
            }
        }
    }

    let config_str = serde_json::to_string(&final_config)
        .map_err(|e| AppError::NotConfigured(format!("Invalid config JSON: {}", e)))?;

    // Upsert config
    sqlx::query(
        r#"
        INSERT INTO delivery_configs (delivery_type, config, is_enabled)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(delivery_type) DO UPDATE SET
            config = ?2,
            is_enabled = ?3
        "#,
    )
    .bind(&input.delivery_type)
    .bind(&config_str)
    .bind(if input.is_enabled { 1 } else { 0 })
    .execute(db.inner())
    .await?;

    Ok(())
}

// ── Stronghold (Secret Storage) ──

#[tauri::command]
pub fn store_secret(
    app: AppHandle,
    key: String,
    value: String,
) -> Result<(), AppError> {
    crate::stronghold::store_secret(&app, &key, &value)
}

#[tauri::command]
pub fn get_secret(
    app: AppHandle,
    key: String,
) -> Result<Option<String>, AppError> {
    crate::stronghold::get_secret(&app, &key)
}

#[tauri::command]
pub fn delete_secret(
    app: AppHandle,
    key: String,
) -> Result<(), AppError> {
    crate::stronghold::delete_secret(&app, &key)
}

// ── Connection Testing ──

#[tauri::command]
pub async fn test_jira_connection(
    _app: AppHandle,
    base_url: String,
    email: String,
    api_token: String,
    project_key: String,
) -> Result<String, AppError> {
    // Test by attempting to fetch tickets
    match crate::aggregation::jira::fetch_tickets_today(&base_url, &email, &api_token, &project_key).await {
        Ok((closed, in_progress)) => {
            Ok(format!(
                "Connected successfully! Found {} closed and {} in-progress tickets today.",
                closed.len(),
                in_progress.len()
            ))
        }
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub async fn test_toggl_connection(
    _app: AppHandle,
    api_token: String,
    workspace_id: String,
) -> Result<String, AppError> {
    // Test by attempting to fetch focus hours
    match crate::aggregation::toggl::fetch_focus_hours_today(&api_token, &workspace_id).await {
        Ok(hours) => {
            Ok(format!(
                "Connected successfully! Tracked {:.1} hours today.",
                hours
            ))
        }
        Err(e) => Err(e),
    }
}
