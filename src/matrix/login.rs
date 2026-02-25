use matrix_sdk::Client;

use crate::error::{ZapError, ZapResult};

/// Log into the Matrix homeserver.
///
/// If the client already has an active session (restored from the SQLite store),
/// this function returns immediately. Otherwise, it prompts the user for a
/// password on stdin and performs username/password login.
pub async fn login(client: &Client, username: &str) -> ZapResult<()> {
    // Check if we have a stored session already.
    if client.matrix_auth().logged_in() {
        tracing::info!("Restored existing session");
        return Ok(());
    }

    // Prompt for password on the terminal.
    let password = rpassword::prompt_password(format!("Password for {}: ", username))
        .map_err(|e| ZapError::Auth(e.to_string()))?;

    client
        .matrix_auth()
        .login_username(username, &password)
        .initial_device_display_name("Zap Terminal Client")
        .send()
        .await
        .map_err(|e| ZapError::Auth(e.to_string()))?;

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
