use chrono::Local;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};

pub struct SchedulerState {
    scheduler: Option<JobScheduler>,
}

impl SchedulerState {
    pub fn new() -> Self {
        Self { scheduler: None }
    }
}

/// Start the daily scheduler with the given time (HH:MM format)
pub async fn start_scheduler(
    app: AppHandle,
    scheduled_time: String,
    state: Arc<Mutex<SchedulerState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Parse scheduled_time (e.g., "17:00")
    let parts: Vec<&str> = scheduled_time.split(':').collect();
    if parts.len() != 2 {
        return Err("Invalid time format. Use HH:MM".into());
    }

    let hour: u32 = parts[0].parse()?;
    let minute: u32 = parts[1].parse()?;

    if hour > 23 || minute > 59 {
        return Err("Invalid hour or minute".into());
    }

    // Stop existing scheduler if running
    stop_scheduler(state.clone()).await?;

    // Create new scheduler
    let scheduler = JobScheduler::new().await?;

    // Build cron expression: "0 {minute} {hour} * * *"
    let cron_expr = format!("0 {} {} * * *", minute, hour);

    // Create job
    let job = Job::new_async(cron_expr.as_str(), move |_uuid, _l| {
        let app_clone = app.clone();
        Box::pin(async move {
            eprintln!("[Scheduler] Triggered at {}", Local::now());

            // Check if today's summary already exists
            // If not, emit event to frontend to trigger generation
            if let Err(e) = app_clone.emit("daily-summary-trigger", ()) {
                eprintln!("[Scheduler] Failed to emit event: {}", e);
            }
        })
    })?;

    scheduler.add(job).await?;
    scheduler.start().await?;

    // Store scheduler in state
    let mut state_lock = state.lock().await;
    state_lock.scheduler = Some(scheduler);

    eprintln!(
        "[Scheduler] Started - will run daily at {} (cron: {})",
        scheduled_time, cron_expr
    );

    Ok(())
}

/// Stop the scheduler
pub async fn stop_scheduler(
    state: Arc<Mutex<SchedulerState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut state_lock = state.lock().await;
    if let Some(mut scheduler) = state_lock.scheduler.take() {
        scheduler.shutdown().await?;
        eprintln!("[Scheduler] Stopped");
    }
    Ok(())
}

/// Check if a summary exists for today, generate if missing (for missed triggers)
pub async fn check_and_generate_if_missed(
    _app: AppHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    // This would be called on app startup
    // For now, we'll implement the check in the command
    eprintln!("[Scheduler] Checking for missed summary generation...");
    Ok(())
}
