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
    fn test_error_display() {
        let api_err = Error::Api("rate limit exceeded".to_string());
        assert_eq!(api_err.to_string(), "API error: rate limit exceeded");

        let auth_err = Error::Auth("invalid signature".to_string());
        assert_eq!(auth_err.to_string(), "Authentication error: invalid signature");

        let param_err = Error::InvalidParameter("amount must be positive".to_string());
        assert_eq!(
            param_err.to_string(),
            "Invalid parameter: amount must be positive"
        );

        let ws_err = Error::WebSocket("connection closed".to_string());
        assert_eq!(ws_err.to_string(), "WebSocket error: connection closed");
    }

    #[test]
    fn test_json_error_conversion() {
        let json_str = "invalid json";
        let result: std::result::Result<serde_json::Value, Error> =
            serde_json::from_str(json_str).map_err(Error::from);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Json(_)));
    }
}
