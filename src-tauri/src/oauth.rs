use crate::error::AppError;
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use std::net::TcpListener;
use tauri::Emitter;

/// Google OAuth2 client configuration
pub struct GoogleOAuthClient {
    client: BasicClient,
}

impl GoogleOAuthClient {
    /// Create a new Google OAuth2 client
    pub fn new(client_id: String, client_secret: String) -> Result<Self, AppError> {
        let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
            .map_err(|e| AppError::NotConfigured(format!("Invalid auth URL: {}", e)))?;

        let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
            .map_err(|e| AppError::NotConfigured(format!("Invalid token URL: {}", e)))?;

        // Find available port for redirect
        let redirect_uri = "http://localhost:8765/callback";
        let redirect_url = RedirectUrl::new(redirect_uri.to_string())
            .map_err(|e| AppError::NotConfigured(format!("Invalid redirect URL: {}", e)))?;

        let client = BasicClient::new(
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),
            auth_url,
            Some(token_url),
        )
        .set_redirect_uri(redirect_url);

        Ok(Self { client })
    }

    /// Generate authorization URL and PKCE verifier
    pub fn get_authorization_url(&self) -> (String, String, String) {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, csrf_token) = self
            .client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new(
                "https://www.googleapis.com/auth/calendar.readonly".to_string(),
            ))
            .add_scope(Scope::new(
                "https://www.googleapis.com/auth/calendar.events.readonly".to_string(),
            ))
            .set_pkce_challenge(pkce_challenge)
            .url();

        (
            auth_url.to_string(),
            csrf_token.secret().to_string(),
            pkce_verifier.secret().to_string(),
        )
    }

    /// Exchange authorization code for tokens
    pub async fn exchange_code(
        &self,
        code: String,
        pkce_verifier: String,
    ) -> Result<(String, String), AppError> {
        let pkce_verifier = oauth2::PkceCodeVerifier::new(pkce_verifier);

        let token_result = self
            .client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(pkce_verifier)
            .request_async(async_http_client)
            .await
            .map_err(|e| AppError::NotConfigured(format!("Token exchange failed: {}", e)))?;

        let access_token = token_result.access_token().secret().to_string();
        let refresh_token = token_result
            .refresh_token()
            .ok_or_else(|| AppError::NotConfigured("No refresh token received".to_string()))?
            .secret()
            .to_string();

        Ok((access_token, refresh_token))
    }

    /// Refresh access token using refresh token
    pub async fn refresh_access_token(
        &self,
        refresh_token: String,
    ) -> Result<String, AppError> {
        let refresh_token = oauth2::RefreshToken::new(refresh_token);

        let token_result = self
            .client
            .exchange_refresh_token(&refresh_token)
            .request_async(async_http_client)
            .await
            .map_err(|e| AppError::NotConfigured(format!("Token refresh failed: {}", e)))?;

        Ok(token_result.access_token().secret().to_string())
    }
}

/// Start OAuth2 callback server and wait for authorization code
/// Emits events to frontend via Tauri event system
pub async fn wait_for_callback(app: tauri::AppHandle) -> Result<String, AppError> {
    // Try to bind to port 8765, fail early if already in use
    let listener = TcpListener::bind("127.0.0.1:8765").map_err(|e| {
        if e.kind() == std::io::ErrorKind::AddrInUse {
            AppError::NotConfigured(
                "OAuth callback port 8765 is already in use. Another OAuth flow may be running."
                    .to_string(),
            )
        } else {
            AppError::NotConfigured(format!("Cannot bind callback server: {}", e))
        }
    })?;

    eprintln!("[OAuth] Callback server listening on http://localhost:8765");

    // Set socket to non-blocking so we can handle shutdown
    listener
        .set_nonblocking(false)
        .map_err(|e| AppError::NotConfigured(format!("Cannot set socket options: {}", e)))?;

    for stream in listener.incoming() {
        if let Ok(mut stream) = stream {
            use std::io::{BufRead, BufReader, Write};

            let buf_reader = BufReader::new(&stream);
            let request_line = buf_reader
                .lines()
                .next()
                .ok_or_else(|| AppError::NotConfigured("No request line".to_string()))?
                .map_err(|e| AppError::NotConfigured(format!("Read error: {}", e)))?;

            // Parse the authorization code from the request
            if request_line.starts_with("GET /callback?") {
                let query = request_line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|path| path.split('?').nth(1))
                    .ok_or_else(|| AppError::NotConfigured("No query string".to_string()))?;

                let mut code = None;
                let mut state = None;

                for param in query.split('&') {
                    if let Some(value) = param.strip_prefix("code=") {
                        code = Some(urlencoding::decode(value).unwrap_or_default().to_string());
                    } else if let Some(value) = param.strip_prefix("state=") {
                        state = Some(urlencoding::decode(value).unwrap_or_default().to_string());
                    }
                }

                let code = code.ok_or_else(|| {
                    AppError::NotConfigured("No authorization code in callback".to_string())
                })?;

                // Validate CSRF token (state parameter)
                let stored_csrf = crate::stronghold::get_secret(
                    &app,
                    crate::stronghold::keys::OAUTH_CSRF_TOKEN,
                )?;

                if let Some(stored) = stored_csrf {
                    if state.as_ref() != Some(&stored) {
                        let _ = app.emit(
                            "oauth-error",
                            "CSRF token mismatch. Possible security issue.",
                        );
                        return Err(AppError::NotConfigured(
                            "CSRF token validation failed".to_string(),
                        ));
                    }
                } else {
                    let _ = app.emit("oauth-error", "No CSRF token found in storage");
                    return Err(AppError::NotConfigured("No CSRF token stored".to_string()));
                }

                // Send success response
                let response = "HTTP/1.1 200 OK\r\n\
                               Content-Type: text/html\r\n\
                               \r\n\
                               <html><body>\
                               <h1>Authorization Successful!</h1>\
                               <p>You can close this window and return to WorkdayDebrief.</p>\
                               </body></html>";

                stream
                    .write_all(response.as_bytes())
                    .map_err(|e| AppError::NotConfigured(format!("Write error: {}", e)))?;

                // Emit success event to frontend
                let _ = app.emit("oauth-code-received", code.clone());

                return Ok(code);
            }
        }
    }

    Err(AppError::NotConfigured(
        "Callback server stopped without receiving code".to_string(),
    ))
}

/// Commands for frontend to initiate OAuth2 flow

#[tauri::command]
pub async fn start_google_oauth(app: tauri::AppHandle) -> Result<String, AppError> {
    // Validate OAuth client credentials first
    let client_id = std::env::var("GOOGLE_CLIENT_ID")
        .unwrap_or_else(|_| "YOUR_CLIENT_ID.apps.googleusercontent.com".to_string());
    let client_secret = std::env::var("GOOGLE_CLIENT_SECRET")
        .unwrap_or_else(|_| "YOUR_CLIENT_SECRET".to_string());

    // Check for placeholder values
    if client_id.contains("YOUR_CLIENT_ID") || client_secret.contains("YOUR_CLIENT_SECRET") {
        return Err(AppError::NotConfigured(
            "Google OAuth credentials not configured. Set GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET environment variables.".to_string()
        ));
    }

    if client_id.is_empty() || client_secret.is_empty() {
        return Err(AppError::NotConfigured(
            "Google OAuth credentials are empty. Check your environment variables.".to_string(),
        ));
    }

    let oauth_client = GoogleOAuthClient::new(client_id, client_secret)?;
    let (auth_url, csrf_token, pkce_verifier) = oauth_client.get_authorization_url();

    // Store CSRF token and PKCE verifier temporarily
    crate::stronghold::store_secret(
        &app,
        crate::stronghold::keys::OAUTH_CSRF_TOKEN,
        &csrf_token,
    )?;
    crate::stronghold::store_secret(
        &app,
        crate::stronghold::keys::OAUTH_PKCE_VERIFIER,
        &pkce_verifier,
    )?;

    // Open browser to authorization URL
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        Command::new("open").arg(&auth_url).spawn().ok();
    }
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        Command::new("cmd")
            .args(&["/C", "start", &auth_url])
            .spawn()
            .ok();
    }
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        Command::new("xdg-open").arg(&auth_url).spawn().ok();
    }

    // Start callback server in background
    let app_clone = app.clone();
    tokio::spawn(async move {
        let app_handle = app_clone.clone();
        match wait_for_callback(app_clone).await {
            Ok(code) => {
                eprintln!("[OAuth] Received authorization code, emitting event");
                // Event already emitted in wait_for_callback
                // Now exchange code for tokens automatically
                if let Err(e) = complete_oauth_flow(app_handle.clone(), code).await {
                    eprintln!("[OAuth] Failed to complete flow: {}", e);
                    let _ = app_handle.emit("oauth-error", e.to_string());
                }
            }
            Err(e) => {
                eprintln!("[OAuth] Callback server error: {}", e);
                let _ = app_handle.emit("oauth-error", e.to_string());
            }
        }
    });

    Ok(auth_url)
}

/// Complete OAuth flow by exchanging code for tokens (called internally)
async fn complete_oauth_flow(app: tauri::AppHandle, code: String) -> Result<(), AppError> {
    let client_id = std::env::var("GOOGLE_CLIENT_ID")
        .unwrap_or_else(|_| "YOUR_CLIENT_ID.apps.googleusercontent.com".to_string());
    let client_secret = std::env::var("GOOGLE_CLIENT_SECRET")
        .unwrap_or_else(|_| "YOUR_CLIENT_SECRET".to_string());

    let oauth_client = GoogleOAuthClient::new(client_id, client_secret)?;

    // Retrieve PKCE verifier
    let pkce_verifier = crate::stronghold::get_secret(
        &app,
        crate::stronghold::keys::OAUTH_PKCE_VERIFIER,
    )?
    .ok_or_else(|| AppError::NotConfigured("No PKCE verifier found".to_string()))?;

    // Exchange code for tokens
    let (_access_token, refresh_token) = oauth_client.exchange_code(code, pkce_verifier).await?;

    // Store refresh token in encrypted storage
    crate::stronghold::store_secret(
        &app,
        crate::stronghold::keys::GOOGLE_REFRESH_TOKEN,
        &refresh_token,
    )?;

    // Clean up temporary secrets
    crate::stronghold::delete_secret(&app, crate::stronghold::keys::OAUTH_CSRF_TOKEN)?;
    crate::stronghold::delete_secret(&app, crate::stronghold::keys::OAUTH_PKCE_VERIFIER)?;

    // Emit success event
    let _ = app.emit("oauth-completed", "Google Calendar connected successfully!");

    Ok(())
}

// Remove the old complete_google_oauth command - it's now handled internally
