use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::path::PathBuf;

pub mod queries;

/// Initialize SQLite database connection pool
pub async fn init_db(app_data_dir: PathBuf) -> Result<SqlitePool, sqlx::Error> {
    // Ensure app data directory exists
    std::fs::create_dir_all(&app_data_dir)?;

    // Database file path
    let db_path = app_data_dir.join("workday-debrief.db");
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

    // Create connection pool with single writer
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&db_url)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}
