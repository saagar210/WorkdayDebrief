mod aggregation;
mod commands;
mod db;
mod delivery;
mod error;
mod llm;
mod markdown;
mod oauth;
mod scheduler;
mod stronghold;

use chrono::Timelike;
use sqlx::Row;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .plugin(
            tauri_plugin_stronghold::Builder::new(|password| {
                // Generate password from machine ID
                password.into()
            })
            .build(),
        )
        .setup(|app| {
            // Get app data directory
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");

            // Initialize database and scheduler
            let handle = app.handle().clone();
            tauri::async_runtime::block_on(async move {
                let pool = db::init_db(app_data_dir)
                    .await
                    .expect("Failed to initialize database");

                handle.manage(pool.clone());

                // Initialize scheduler state
                let scheduler_state = Arc::new(Mutex::new(scheduler::SchedulerState::new()));
                handle.manage(scheduler_state.clone());

                // Load settings and start scheduler if configured
                if let Ok(settings) = load_and_start_scheduler(&handle, &pool, scheduler_state).await {
                    eprintln!("[Startup] Loaded settings, scheduler ready");

                    // Check for missed summary generation
                    check_missed_summary(&handle, &pool, &settings).await;
                } else {
                    eprintln!("[Startup] No settings or scheduler not configured");
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_today_summary,
            commands::save_summary,
            commands::list_summaries,
            commands::get_summary_by_date,
            commands::generate_summary,
            commands::regenerate_narrative,
            commands::send_summary,
            commands::test_delivery,
            commands::get_settings,
            commands::save_settings,
            commands::get_delivery_configs,
            commands::save_delivery_config,
            commands::store_secret,
            commands::get_secret,
            commands::delete_secret,
            commands::test_jira_connection,
            commands::test_toggl_connection,
            oauth::start_google_oauth,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Load settings from database and start scheduler if configured
async fn load_and_start_scheduler(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    scheduler_state: Arc<Mutex<scheduler::SchedulerState>>,
) -> Result<commands::Settings, Box<dyn std::error::Error>> {
    // Load settings from database
    let row = sqlx::query(
        r#"
        SELECT scheduled_time, default_tone, enable_llm, llm_model, llm_temperature,
               llm_timeout_secs, calendar_source, retention_days, jira_base_url,
               jira_project_key, toggl_workspace_id
        FROM settings
        WHERE id = 1
        "#,
    )
    .fetch_one(pool)
    .await?;

    let settings = commands::Settings {
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
    };

    // Start scheduler if time is configured (not default "17:00" or user has set it)
    if !settings.scheduled_time.is_empty() {
        match scheduler::start_scheduler(
            app.clone(),
            settings.scheduled_time.clone(),
            scheduler_state,
        )
        .await
        {
            Ok(_) => eprintln!("[Scheduler] Started with time: {}", settings.scheduled_time),
            Err(e) => eprintln!("[Scheduler] Failed to start: {}", e),
        }
    }

    Ok(settings)
}

/// Check if we missed today's scheduled summary generation
async fn check_missed_summary(
    app: &tauri::AppHandle,
    pool: &sqlx::SqlitePool,
    settings: &commands::Settings,
) {
    use chrono::Local;

    let now = Local::now();
    let today = now.format("%Y-%m-%d").to_string();

    // Check if summary already exists for today
    let existing = sqlx::query(
        r#"
        SELECT id FROM daily_summaries
        WHERE summary_date = ?1
        "#,
    )
    .bind(&today)
    .fetch_optional(pool)
    .await;

    if let Ok(None) = existing {
        // No summary exists for today
        // Parse scheduled time and check if we're past it
        let parts: Vec<&str> = settings.scheduled_time.split(':').collect();
        if parts.len() == 2 {
            if let (Ok(hour), Ok(minute)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                let current_hour = now.hour();
                let current_minute = now.minute();

                // If current time is past scheduled time, we missed the trigger
                if current_hour > hour || (current_hour == hour && current_minute >= minute) {
                    eprintln!(
                        "[Startup] Missed summary generation (scheduled: {}:{:02}, now: {}:{:02})",
                        hour, minute, current_hour, current_minute
                    );

                    // Emit event to frontend to trigger generation
                    if let Err(e) = app.emit("daily-summary-trigger", ()) {
                        eprintln!("[Startup] Failed to emit missed trigger event: {}", e);
                    }
                }
            }
        }
    }
}
