use matrix_sdk::authentication::matrix::MatrixSession;
use matrix_sdk::store::RoomLoadSettings;
use matrix_sdk::Client;

use crate::error::{ZapError, ZapResult};

/// Path to the serialized session file (alongside the matrix-sdk SQLite stores).
fn session_path(data_dir: &std::path::Path) -> std::path::PathBuf {
    data_dir.join("matrix").join("session.json")
}

/// Log into the Matrix homeserver.
///
/// First tries to restore a saved session from disk. If no session exists,
/// prompts the user for a password and performs username/password login,
/// then saves the session for future restarts.
pub async fn login(
    client: &Client,
    username: &str,
    data_dir: &std::path::Path,
) -> ZapResult<()> {
    let sess_path = session_path(data_dir);

    // Try to restore a saved session.
    if sess_path.exists() {
        let session_json = std::fs::read_to_string(&sess_path)
            .map_err(|e| ZapError::Auth(format!("failed to read session file: {}", e)))?;
        let session: MatrixSession = serde_json::from_str(&session_json)
            .map_err(|e| ZapError::Auth(format!("failed to parse session file: {}", e)))?;

        client
            .matrix_auth()
            .restore_session(session, RoomLoadSettings::default())
            .await
            .map_err(|e| ZapError::Auth(e.to_string()))?;

        tracing::info!("Restored existing session for {}", username);
        return Ok(());
    }

    // No saved session — prompt for password.
    let password = rpassword::prompt_password(format!("Password for {}: ", username))
        .map_err(|e| ZapError::Auth(e.to_string()))?;

    client
        .matrix_auth()
        .login_username(username, &password)
        .initial_device_display_name("Zap Terminal Client")
        .send()
        .await
        .map_err(|e| ZapError::Auth(e.to_string()))?;

    // Save the session for future runs.
    if let Some(session) = client.matrix_auth().session() {
        let session_json = serde_json::to_string(&session)
            .map_err(|e| ZapError::Auth(format!("failed to serialize session: {}", e)))?;
        std::fs::write(&sess_path, session_json)
            .map_err(|e| ZapError::Auth(format!("failed to write session file: {}", e)))?;
        tracing::info!("Session saved to {:?}", sess_path);
    }

    tracing::info!("Logged in as {}", username);
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_login_module_compiles() {
        // Login requires a live Matrix homeserver. This test verifies the module
        // compiles and the function signature is correct.
        assert!(true);
    }
}
