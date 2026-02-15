use crate::error::AppError;
use age::secrecy::Secret;
use std::io::{Read, Write};
use std::path::PathBuf;
use tauri::Manager;

/// Store a secret in encrypted file storage
/// Uses age encryption with a derived passphrase for secure storage
pub fn store_secret(
    app: &tauri::AppHandle,
    key: &str,
    value: &str,
) -> Result<(), AppError> {
    let store_path = get_secrets_store_path(app)?;
    let mut secrets = load_secrets(&store_path, app)?;

    secrets.insert(key.to_string(), value.to_string());
    save_secrets(&store_path, &secrets, app)?;

    Ok(())
}

/// Retrieve a secret from encrypted storage
pub fn get_secret(
    app: &tauri::AppHandle,
    key: &str,
) -> Result<Option<String>, AppError> {
    let store_path = get_secrets_store_path(app)?;
    let secrets = load_secrets(&store_path, app)?;

    Ok(secrets.get(key).cloned())
}

/// Delete a secret from encrypted storage
pub fn delete_secret(
    app: &tauri::AppHandle,
    key: &str,
) -> Result<(), AppError> {
    let store_path = get_secrets_store_path(app)?;
    let mut secrets = load_secrets(&store_path, app)?;

    secrets.remove(key);
    save_secrets(&store_path, &secrets, app)?;

    Ok(())
}

/// Get the path to the secrets store file
fn get_secrets_store_path(app: &tauri::AppHandle) -> Result<PathBuf, AppError> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::NotConfigured(format!("Cannot get app data dir: {}", e)))?;

    Ok(app_data_dir.join("secrets.enc"))
}

/// Derive a passphrase for age encryption from machine-specific data
fn derive_passphrase(app: &tauri::AppHandle) -> Result<String, AppError> {
    // Use app-specific identifier + machine hostname for deterministic key derivation
    let app_name = "WorkdayDebrief";

    // Get hostname as machine identifier
    let hostname = hostname::get()
        .map_err(|e| AppError::NotConfigured(format!("Cannot get hostname: {}", e)))?
        .to_string_lossy()
        .to_string();

    // Get app config dir path as additional entropy
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::NotConfigured(format!("Cannot get config dir: {}", e)))?;

    let config_path = config_dir.to_string_lossy();

    // Combine to create deterministic passphrase
    // This is machine-specific but deterministic for the same machine
    Ok(format!("{}-{}-{}", app_name, hostname, config_path))
}

/// Load secrets from encrypted file (or create empty if doesn't exist)
fn load_secrets(
    path: &PathBuf,
    app: &tauri::AppHandle,
) -> Result<std::collections::HashMap<String, String>, AppError> {
    if !path.exists() {
        return Ok(std::collections::HashMap::new());
    }

    let encrypted_data = std::fs::read(path)
        .map_err(|e| AppError::NotConfigured(format!("Cannot read secrets file: {}", e)))?;

    // Decrypt using age with passphrase
    let passphrase = derive_passphrase(app)?;
    let decryptor = match age::Decryptor::new(&encrypted_data[..])
        .map_err(|e| AppError::NotConfigured(format!("Cannot create decryptor: {}", e)))?
    {
        age::Decryptor::Passphrase(d) => d,
        _ => return Err(AppError::NotConfigured(
            "Secrets file is encrypted with unexpected method".to_string()
        )),
    };

    let mut decrypted_data = Vec::new();
    let mut reader = decryptor
        .decrypt(&Secret::new(passphrase), None)
        .map_err(|e| AppError::NotConfigured(format!("Cannot decrypt secrets: {}", e)))?;

    reader
        .read_to_end(&mut decrypted_data)
        .map_err(|e| AppError::NotConfigured(format!("Cannot read decrypted data: {}", e)))?;

    let json_str = String::from_utf8(decrypted_data)
        .map_err(|e| AppError::NotConfigured(format!("Invalid UTF-8 in secrets: {}", e)))?;

    serde_json::from_str(&json_str)
        .map_err(|e| AppError::NotConfigured(format!("Cannot parse secrets JSON: {}", e)))
}

/// Save secrets to encrypted file
fn save_secrets(
    path: &PathBuf,
    secrets: &std::collections::HashMap<String, String>,
    app: &tauri::AppHandle,
) -> Result<(), AppError> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::NotConfigured(format!("Cannot create secrets dir: {}", e)))?;
    }

    let json_str = serde_json::to_string(secrets)
        .map_err(|e| AppError::NotConfigured(format!("Cannot serialize secrets: {}", e)))?;

    // Encrypt using age with passphrase
    let passphrase = derive_passphrase(app)?;
    let encryptor = age::Encryptor::with_user_passphrase(Secret::new(passphrase));

    let mut encrypted_data = Vec::new();
    let mut writer = encryptor
        .wrap_output(&mut encrypted_data)
        .map_err(|e| AppError::NotConfigured(format!("Cannot create encryptor: {}", e)))?;

    writer
        .write_all(json_str.as_bytes())
        .map_err(|e| AppError::NotConfigured(format!("Cannot write encrypted data: {}", e)))?;

    writer
        .finish()
        .map_err(|e| AppError::NotConfigured(format!("Cannot finalize encryption: {}", e)))?;

    std::fs::write(path, encrypted_data)
        .map_err(|e| AppError::NotConfigured(format!("Cannot write secrets file: {}", e)))?;

    Ok(())
}

/// Secret keys used in the app
pub mod keys {
    pub const SMTP_PASSWORD: &str = "smtp_password";
    pub const SLACK_WEBHOOK_URL: &str = "slack_webhook_url";
    pub const JIRA_API_TOKEN: &str = "jira_api_token";
    pub const JIRA_EMAIL: &str = "jira_email";
    pub const GOOGLE_REFRESH_TOKEN: &str = "google_refresh_token";
    pub const TOGGL_API_TOKEN: &str = "toggl_api_token";
    pub const OAUTH_CSRF_TOKEN: &str = "oauth_csrf_token";
    pub const OAUTH_PKCE_VERIFIER: &str = "oauth_pkce_verifier";
}
