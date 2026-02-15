use crate::error::AppError;
use age::secrecy::Secret;
use base64::Engine;
use rand::RngCore;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tauri::Manager;

const MASTER_KEY_FILE: &str = "master.key";

/// Store a secret in encrypted file storage.
pub fn store_secret(app: &tauri::AppHandle, key: &str, value: &str) -> Result<(), AppError> {
    let store_path = get_secrets_store_path(app)?;
    let mut secrets = load_secrets(&store_path, app)?;

    secrets.insert(key.to_string(), value.to_string());
    save_secrets(&store_path, &secrets, app)?;

    Ok(())
}

/// Retrieve a secret from encrypted storage.
pub fn get_secret(app: &tauri::AppHandle, key: &str) -> Result<Option<String>, AppError> {
    let store_path = get_secrets_store_path(app)?;
    let secrets = load_secrets(&store_path, app)?;

    Ok(secrets.get(key).cloned())
}

/// Delete a secret from encrypted storage.
pub fn delete_secret(app: &tauri::AppHandle, key: &str) -> Result<(), AppError> {
    let store_path = get_secrets_store_path(app)?;
    let mut secrets = load_secrets(&store_path, app)?;

    secrets.remove(key);
    save_secrets(&store_path, &secrets, app)?;

    Ok(())
}

/// Get the path to the encrypted secrets store.
fn get_secrets_store_path(app: &tauri::AppHandle) -> Result<PathBuf, AppError> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::NotConfigured(format!("Cannot get app data dir: {}", e)))?;

    Ok(app_data_dir.join("secrets.enc"))
}

/// Get the path to the local master key file.
fn get_master_key_path(app: &tauri::AppHandle) -> Result<PathBuf, AppError> {
    let app_config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::NotConfigured(format!("Cannot get config dir: {}", e)))?;

    Ok(app_config_dir.join(MASTER_KEY_FILE))
}

/// Legacy deterministic passphrase derivation used by older releases.
fn derive_legacy_passphrase(app: &tauri::AppHandle) -> Result<String, AppError> {
    let app_name = "WorkdayDebrief";

    let hostname = hostname::get()
        .map_err(|e| AppError::NotConfigured(format!("Cannot get hostname: {}", e)))?
        .to_string_lossy()
        .to_string();

    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::NotConfigured(format!("Cannot get config dir: {}", e)))?;

    let config_path = config_dir.to_string_lossy();

    Ok(format!("{}-{}-{}", app_name, hostname, config_path))
}

fn set_secure_permissions(path: &Path, mode: u32) -> Result<(), AppError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(|e| {
            AppError::NotConfigured(format!(
                "Cannot set secure permissions on {}: {}",
                path.display(),
                e
            ))
        })?;
    }

    #[cfg(not(unix))]
    {
        let _ = mode;
        let _ = path;
    }

    Ok(())
}

fn get_or_create_master_key(app: &tauri::AppHandle) -> Result<String, AppError> {
    if let Ok(env_key) = std::env::var("WORKDAY_DEBRIEF_MASTER_KEY") {
        let trimmed = env_key.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    let key_path = get_master_key_path(app)?;

    if key_path.exists() {
        let existing = fs::read_to_string(&key_path)
            .map_err(|e| AppError::NotConfigured(format!("Cannot read master key: {}", e)))?;

        let trimmed = existing.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    if let Some(parent) = key_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| AppError::NotConfigured(format!("Cannot create config dir: {}", e)))?;
        let _ = set_secure_permissions(parent, 0o700);
    }

    let mut key_bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut key_bytes);
    let master_key = base64::engine::general_purpose::STANDARD.encode(key_bytes);

    match OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&key_path)
    {
        Ok(mut file) => {
            file.write_all(master_key.as_bytes())
                .map_err(|e| AppError::NotConfigured(format!("Cannot write master key: {}", e)))?;
            set_secure_permissions(&key_path, 0o600)?;
            Ok(master_key)
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            let existing = fs::read_to_string(&key_path).map_err(|read_err| {
                AppError::NotConfigured(format!("Cannot read existing master key: {}", read_err))
            })?;
            let trimmed = existing.trim();
            if trimmed.is_empty() {
                Err(AppError::NotConfigured(
                    "Master key file exists but is empty".to_string(),
                ))
            } else {
                Ok(trimmed.to_string())
            }
        }
        Err(e) => Err(AppError::NotConfigured(format!(
            "Cannot create master key file: {}",
            e
        ))),
    }
}

fn decrypt_secrets_with_passphrase(
    encrypted_data: &[u8],
    passphrase: &str,
) -> Result<HashMap<String, String>, AppError> {
    let decryptor = match age::Decryptor::new(encrypted_data)
        .map_err(|e| AppError::NotConfigured(format!("Cannot create decryptor: {}", e)))?
    {
        age::Decryptor::Passphrase(d) => d,
        _ => {
            return Err(AppError::NotConfigured(
                "Secrets file is encrypted with unexpected method".to_string(),
            ))
        }
    };

    let mut decrypted_data = Vec::new();
    let mut reader = decryptor
        .decrypt(&Secret::new(passphrase.to_string()), None)
        .map_err(|e| AppError::NotConfigured(format!("Cannot decrypt secrets: {}", e)))?;

    reader
        .read_to_end(&mut decrypted_data)
        .map_err(|e| AppError::NotConfigured(format!("Cannot read decrypted data: {}", e)))?;

    let json_str = String::from_utf8(decrypted_data)
        .map_err(|e| AppError::NotConfigured(format!("Invalid UTF-8 in secrets: {}", e)))?;

    serde_json::from_str(&json_str)
        .map_err(|e| AppError::NotConfigured(format!("Cannot parse secrets JSON: {}", e)))
}

/// Load secrets from encrypted file (or create empty if missing).
fn load_secrets(
    path: &PathBuf,
    app: &tauri::AppHandle,
) -> Result<HashMap<String, String>, AppError> {
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let encrypted_data = fs::read(path)
        .map_err(|e| AppError::NotConfigured(format!("Cannot read secrets file: {}", e)))?;

    let master_key = get_or_create_master_key(app)?;

    match decrypt_secrets_with_passphrase(&encrypted_data, &master_key) {
        Ok(secrets) => Ok(secrets),
        Err(primary_err) => {
            // Backward-compatible migration path for previously deterministic encryption.
            let legacy_key = derive_legacy_passphrase(app)?;
            let legacy_secrets = decrypt_secrets_with_passphrase(&encrypted_data, &legacy_key)
                .map_err(|_| primary_err)?;

            // Re-encrypt immediately with the random master key.
            save_secrets_with_passphrase(path, &legacy_secrets, &master_key)?;
            Ok(legacy_secrets)
        }
    }
}

/// Save secrets to encrypted file.
fn save_secrets(
    path: &PathBuf,
    secrets: &HashMap<String, String>,
    app: &tauri::AppHandle,
) -> Result<(), AppError> {
    let master_key = get_or_create_master_key(app)?;
    save_secrets_with_passphrase(path, secrets, &master_key)
}

fn save_secrets_with_passphrase(
    path: &PathBuf,
    secrets: &HashMap<String, String>,
    passphrase: &str,
) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| AppError::NotConfigured(format!("Cannot create secrets dir: {}", e)))?;
        let _ = set_secure_permissions(parent, 0o700);
    }

    let json_str = serde_json::to_string(secrets)
        .map_err(|e| AppError::NotConfigured(format!("Cannot serialize secrets: {}", e)))?;

    let encryptor = age::Encryptor::with_user_passphrase(Secret::new(passphrase.to_string()));

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

    fs::write(path, encrypted_data)
        .map_err(|e| AppError::NotConfigured(format!("Cannot write secrets file: {}", e)))?;
    set_secure_permissions(path, 0o600)?;

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
