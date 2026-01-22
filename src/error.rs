//! Error types for the Hyperliquid SDK

use thiserror::Error;

/// Result type alias for SDK operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur when using the Hyperliquid SDK
#[derive(Error, Debug)]
pub enum Error {
    /// HTTP client error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// API returned an error response
    #[error("API error: {0}")]
    Api(String),

    /// Authentication error
    #[error("Authentication error: {0}")]
    Auth(String),

    /// Invalid parameter error
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    /// WebSocket error
    #[error("WebSocket error: {0}")]
    WebSocket(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_api() {
        let err = Error::Api("rate limit exceeded".to_string());
        assert_eq!(err.to_string(), "API error: rate limit exceeded");
    }

    #[test]
    fn test_error_display_auth() {
        let err = Error::Auth("invalid signature".to_string());
        assert_eq!(err.to_string(), "Authentication error: invalid signature");
    }

    #[test]
    fn test_error_display_invalid_parameter() {
        let err = Error::InvalidParameter("amount must be positive".to_string());
        assert_eq!(
            err.to_string(),
            "Invalid parameter: amount must be positive"
        );
    }

    #[test]
    fn test_error_display_websocket() {
        let err = Error::WebSocket("connection closed".to_string());
        assert_eq!(err.to_string(), "WebSocket error: connection closed");
    }

    #[tokio::test]
    async fn test_error_display_http() {
        // Create an error via the From trait with a request that will fail
        let client = reqwest::Client::new();
        let result = client.get("http://invalid.invalid.invalid").send().await;
        if let Err(reqwest_err) = result {
            let err: Error = reqwest_err.into();
            assert!(err.to_string().starts_with("HTTP error:"));
            assert!(matches!(err, Error::Http(_)));
        }
    }

    #[test]
    fn test_error_display_json() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let err: Error = json_err.into();
        assert!(err.to_string().starts_with("JSON error:"));
        assert!(matches!(err, Error::Json(_)));
    }

    #[test]
    fn test_json_error_conversion() {
        let json_str = "invalid json";
        let result: std::result::Result<serde_json::Value, Error> =
            serde_json::from_str(json_str).map_err(Error::from);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Json(_)));
    }

    #[test]
    fn test_result_type_alias() {
        fn returns_ok() -> Result<i32> {
            Ok(42)
        }

        fn returns_err() -> Result<i32> {
            Err(Error::Api("test error".to_string()))
        }

        assert!(returns_ok().is_ok());
        assert_eq!(returns_ok().unwrap(), 42);
        assert!(returns_err().is_err());
    }

    #[test]
    fn test_error_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        // These compile only if Error implements Send and Sync
        assert_send::<Error>();
        assert_sync::<Error>();
    }

    #[test]
    fn test_error_debug_impl() {
        let err = Error::Api("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Api"));
        assert!(debug_str.contains("test"));
    }
}
