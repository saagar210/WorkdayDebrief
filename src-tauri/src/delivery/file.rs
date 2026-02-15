use crate::error::AppError;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct FileConfig {
    pub directory_path: String,
}

/// Write summary as markdown file
pub fn write_markdown(summary: &str, config: &FileConfig, date: &str) -> Result<PathBuf, AppError> {
    let dir_path = Path::new(&config.directory_path);

    // Create directory if it doesn't exist
    if !dir_path.exists() {
        fs::create_dir_all(dir_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                AppError::FileWriteError(format!(
                    "Permission denied: Cannot create directory '{}'",
                    config.directory_path
                ))
            } else {
                AppError::FileWriteError(format!("Failed to create directory: {}", e))
            }
        })?;
    }

    // Build file path
    let file_name = format!("{}.md", date);
    let file_path = dir_path.join(&file_name);

    // Write file
    fs::write(&file_path, summary).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            AppError::FileWriteError(format!(
                "Permission denied: Cannot write to '{}'",
                file_path.display()
            ))
        } else if e.raw_os_error() == Some(28) {
            // ENOSPC - No space left on device
            AppError::FileWriteError("Disk full".to_string())
        } else {
            AppError::FileWriteError(format!("Failed to write file: {}", e))
        }
    })?;

    Ok(file_path)
}
