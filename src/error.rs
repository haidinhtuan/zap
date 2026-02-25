use thiserror::Error;

#[derive(Debug, Error)]
pub enum ZapError {
    #[error("Matrix error: {0}")]
    Matrix(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Terminal error: {0}")]
    Terminal(String),

    #[error("Authentication failed: {0}")]
    Auth(String),
}

pub type ZapResult<T> = Result<T, ZapError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_error_display() {
        let err = ZapError::Matrix("connection refused".to_string());
        assert_eq!(err.to_string(), "Matrix error: connection refused");
    }

    #[test]
    fn test_config_error_display() {
        let err = ZapError::Config("missing field".to_string());
        assert_eq!(err.to_string(), "Configuration error: missing field");
    }

    #[test]
    fn test_io_error_from() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let zap_err: ZapError = io_err.into();
        assert!(zap_err.to_string().contains("file not found"));
    }

    #[test]
    fn test_database_error_display() {
        let err = ZapError::Database("table missing".to_string());
        assert_eq!(err.to_string(), "Database error: table missing");
    }

    #[test]
    fn test_terminal_error_display() {
        let err = ZapError::Terminal("failed to init".to_string());
        assert_eq!(err.to_string(), "Terminal error: failed to init");
    }

    #[test]
    fn test_auth_error_display() {
        let err = ZapError::Auth("bad password".to_string());
        assert_eq!(err.to_string(), "Authentication failed: bad password");
    }

    #[test]
    fn test_zap_result_ok() {
        let result: ZapResult<i32> = Ok(42);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_zap_result_err() {
        let result: ZapResult<i32> = Err(ZapError::Config("oops".to_string()));
        assert!(result.is_err());
    }
}
