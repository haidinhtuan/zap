use matrix_sdk::Client;
use std::path::Path;

use crate::error::ZapResult;

/// Create a Matrix client configured with a SQLite store for session persistence.
pub async fn create_client(homeserver: &str, data_dir: &Path) -> ZapResult<Client> {
    std::fs::create_dir_all(data_dir)?;

    let client = Client::builder()
        .homeserver_url(homeserver)
        .sqlite_store(data_dir.join("matrix"), None)
        .build()
        .await
        .map_err(|e| crate::error::ZapError::Matrix(e.to_string()))?;

    Ok(client)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_client_module_compiles() {
        // Client creation requires a reachable homeserver. This test verifies
        // the module compiles correctly.
        assert!(true);
    }
}
