//! Miscellaneous exchange endpoints for the Hyperliquid Exchange API
//!
//! This module provides misc exchange actions including:
//! - `reserve_request_weight` - Purchase additional rate limit capacity
//! - `noop` - No-operation action (useful for testing signatures)
//! - `set_hip3_enabled` - Toggle HIP-3 abstraction mode

use serde::{Deserialize, Serialize};

use crate::auth::{Signature, Wallet};
use crate::client::Client;
use crate::error::{Error, Result};

/// Reserve request weight action - purchase additional rate limit capacity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReserveRequestWeightAction {
    #[serde(rename = "type")]
    pub action_type: String,
    /// The amount of weight to reserve (in USDC)
    pub weight: u64,
}

/// No-operation action - does nothing but validates signature
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoopAction {
    #[serde(rename = "type")]
    pub action_type: String,
}

/// Set HIP-3 enabled action - toggle HIP-3 abstraction mode
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetHip3EnabledAction {
    #[serde(rename = "type")]
    pub action_type: String,
    /// Whether HIP-3 abstraction is enabled
    pub enabled: bool,
}

/// Exchange request wrapper with authentication for misc operations
#[derive(Debug, Clone, Serialize)]
pub struct MiscExchangeRequest<T> {
    pub action: T,
    pub nonce: u64,
    pub signature: Signature,
}

/// Exchange API response for misc operations
#[derive(Debug, Clone, Deserialize)]
pub struct MiscResponse {
    pub status: String,
    pub response: Option<MiscResponseData>,
}

/// Response data for misc operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiscResponseData {
    #[serde(rename = "type")]
    pub response_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<MiscResultData>,
}

/// Result data for misc operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiscResultData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

impl Client {
    /// Reserve additional rate limit capacity
    ///
    /// This allows you to purchase additional request weight (rate limit capacity)
    /// for the current trading session. The cost is deducted from your account.
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with
    /// * `weight` - The amount of weight to reserve (in USDC)
    ///
    /// # Returns
    /// The response data if successful
    ///
    /// # Errors
    /// Returns an error if:
    /// - The weight is zero
    /// - The wallet signing fails
    /// - The API returns an error
    ///
    /// # Example
    /// ```ignore
    /// use hyperliquid_sdk::{Client, Wallet};
    ///
    /// let client = Client::mainnet()?;
    /// let wallet = Wallet::from_private_key("0x...", true)?;
    ///
    /// // Reserve 10 units of request weight
    /// let response = client.reserve_request_weight(&wallet, 10).await?;
    /// ```
    pub async fn reserve_request_weight(
        &self,
        wallet: &Wallet,
        weight: u64,
    ) -> Result<MiscResponseData> {
        // Validate inputs
        if weight == 0 {
            return Err(Error::InvalidParameter(
                "Weight must be greater than zero".to_string(),
            ));
        }

        // Create the reserve request weight action
        let action = ReserveRequestWeightAction {
            action_type: "reserveRequestWeight".to_string(),
            weight,
        };

        // Generate nonce and sign
        let nonce = Wallet::generate_nonce();
        let signature = wallet.sign_l1_action(&action, nonce, None).await?;

        // Create the exchange request
        let request = MiscExchangeRequest {
            action,
            nonce,
            signature,
        };

        // Send request and parse response
        let response: MiscResponse = self.post_exchange(&request).await?;

        // Check for API-level errors
        if response.status != "ok" {
            return Err(Error::Api(format!(
                "Reserve request weight failed: {}",
                response.status
            )));
        }

        response
            .response
            .ok_or_else(|| Error::Api("No response data received".to_string()))
    }

    /// Send a no-operation action
    ///
    /// This action does nothing but validates the signature and nonce.
    /// It can be useful for testing wallet connectivity and signing.
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with
    ///
    /// # Returns
    /// The response data if successful
    ///
    /// # Errors
    /// Returns an error if:
    /// - The wallet signing fails
    /// - The API returns an error
    ///
    /// # Example
    /// ```ignore
    /// use hyperliquid_sdk::{Client, Wallet};
    ///
    /// let client = Client::mainnet()?;
    /// let wallet = Wallet::from_private_key("0x...", true)?;
    ///
    /// // Send a no-op action to test connectivity
    /// let response = client.noop(&wallet).await?;
    /// ```
    pub async fn noop(&self, wallet: &Wallet) -> Result<MiscResponseData> {
        // Create the noop action
        let action = NoopAction {
            action_type: "noop".to_string(),
        };

        // Generate nonce and sign
        let nonce = Wallet::generate_nonce();
        let signature = wallet.sign_l1_action(&action, nonce, None).await?;

        // Create the exchange request
        let request = MiscExchangeRequest {
            action,
            nonce,
            signature,
        };

        // Send request and parse response
        let response: MiscResponse = self.post_exchange(&request).await?;

        // Check for API-level errors
        if response.status != "ok" {
            return Err(Error::Api(format!("Noop failed: {}", response.status)));
        }

        response
            .response
            .ok_or_else(|| Error::Api("No response data received".to_string()))
    }

    /// Set HIP-3 abstraction mode
    ///
    /// HIP-3 is an abstraction layer that simplifies some trading operations.
    /// This action enables or disables HIP-3 mode for your account.
    ///
    /// # Arguments
    /// * `wallet` - The wallet to sign the action with
    /// * `enabled` - Whether to enable or disable HIP-3 abstraction
    ///
    /// # Returns
    /// The response data if successful
    ///
    /// # Errors
    /// Returns an error if:
    /// - The wallet signing fails
    /// - The API returns an error
    ///
    /// # Example
    /// ```ignore
    /// use hyperliquid_sdk::{Client, Wallet};
    ///
    /// let client = Client::mainnet()?;
    /// let wallet = Wallet::from_private_key("0x...", true)?;
    ///
    /// // Enable HIP-3 abstraction
    /// let response = client.set_hip3_enabled(&wallet, true).await?;
    /// ```
    pub async fn set_hip3_enabled(
        &self,
        wallet: &Wallet,
        enabled: bool,
    ) -> Result<MiscResponseData> {
        // Create the set HIP-3 enabled action
        let action = SetHip3EnabledAction {
            action_type: "setHip3Enabled".to_string(),
            enabled,
        };

        // Generate nonce and sign
        let nonce = Wallet::generate_nonce();
        let signature = wallet.sign_l1_action(&action, nonce, None).await?;

        // Create the exchange request
        let request = MiscExchangeRequest {
            action,
            nonce,
            signature,
        };

        // Send request and parse response
        let response: MiscResponse = self.post_exchange(&request).await?;

        // Check for API-level errors
        if response.status != "ok" {
            return Err(Error::Api(format!(
                "Set HIP-3 enabled failed: {}",
                response.status
            )));
        }

        response
            .response
            .ok_or_else(|| Error::Api("No response data received".to_string()))
    }
}

/// Check if a misc operation was successful
pub fn is_misc_successful(response: &MiscResponseData) -> bool {
    !response.response_type.is_empty()
}

/// Get the transaction hash from a misc response
pub fn get_misc_hash(response: &MiscResponseData) -> Option<&str> {
    response.data.as_ref().and_then(|d| d.hash.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use reqwest::Client as ReqwestClient;
    use serde::de::DeserializeOwned;

    const TEST_PRIVATE_KEY: &str =
        "0x0123456789012345678901234567890123456789012345678901234567890123";

    /// Test client that can be configured with a custom base URL
    struct TestClient {
        http: ReqwestClient,
        base_url: String,
    }

    impl TestClient {
        fn new(base_url: &str) -> Self {
            Self {
                http: ReqwestClient::new(),
                base_url: base_url.to_string(),
            }
        }

        fn exchange_url(&self) -> String {
            format!("{}/exchange", self.base_url)
        }

        async fn post_exchange<T, R>(&self, request: &T) -> Result<R>
        where
            T: Serialize + ?Sized,
            R: DeserializeOwned,
        {
            let url = self.exchange_url();
            let response = self
                .http
                .post(&url)
                .json(request)
                .send()
                .await
                .map_err(Error::Http)?;

            let status = response.status();
            let body = response.text().await.map_err(Error::Http)?;

            if !status.is_success() {
                return Err(Error::Api(format!("HTTP {} - {}", status.as_u16(), body)));
            }

            serde_json::from_str(&body).map_err(|e| {
                if let Ok(error_obj) = serde_json::from_str::<serde_json::Value>(&body) {
                    if let Some(error_msg) = error_obj.get("error").and_then(|v| v.as_str()) {
                        return Error::Api(error_msg.to_string());
                    }
                }
                Error::Json(e)
            })
        }

        async fn reserve_request_weight(
            &self,
            wallet: &Wallet,
            weight: u64,
        ) -> Result<MiscResponseData> {
            if weight == 0 {
                return Err(Error::InvalidParameter(
                    "Weight must be greater than zero".to_string(),
                ));
            }

            let action = ReserveRequestWeightAction {
                action_type: "reserveRequestWeight".to_string(),
                weight,
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await?;

            let request = MiscExchangeRequest {
                action,
                nonce,
                signature,
            };

            let response: MiscResponse = self.post_exchange(&request).await?;

            if response.status != "ok" {
                return Err(Error::Api(format!(
                    "Reserve request weight failed: {}",
                    response.status
                )));
            }

            response
                .response
                .ok_or_else(|| Error::Api("No response data received".to_string()))
        }

        async fn noop(&self, wallet: &Wallet) -> Result<MiscResponseData> {
            let action = NoopAction {
                action_type: "noop".to_string(),
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await?;

            let request = MiscExchangeRequest {
                action,
                nonce,
                signature,
            };

            let response: MiscResponse = self.post_exchange(&request).await?;

            if response.status != "ok" {
                return Err(Error::Api(format!("Noop failed: {}", response.status)));
            }

            response
                .response
                .ok_or_else(|| Error::Api("No response data received".to_string()))
        }

        async fn set_hip3_enabled(
            &self,
            wallet: &Wallet,
            enabled: bool,
        ) -> Result<MiscResponseData> {
            let action = SetHip3EnabledAction {
                action_type: "setHip3Enabled".to_string(),
                enabled,
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await?;

            let request = MiscExchangeRequest {
                action,
                nonce,
                signature,
            };

            let response: MiscResponse = self.post_exchange(&request).await?;

            if response.status != "ok" {
                return Err(Error::Api(format!(
                    "Set HIP-3 enabled failed: {}",
                    response.status
                )));
            }

            response
                .response
                .ok_or_else(|| Error::Api("No response data received".to_string()))
        }
    }

    // ============================================================================
    // Serialization Tests
    // ============================================================================

    #[test]
    fn test_reserve_request_weight_action_serialization() {
        let action = ReserveRequestWeightAction {
            action_type: "reserveRequestWeight".to_string(),
            weight: 10,
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"reserveRequestWeight\""));
        assert!(json.contains("\"weight\":10"));
    }

    #[test]
    fn test_noop_action_serialization() {
        let action = NoopAction {
            action_type: "noop".to_string(),
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"noop\""));
    }

    #[test]
    fn test_set_hip3_enabled_action_serialization_true() {
        let action = SetHip3EnabledAction {
            action_type: "setHip3Enabled".to_string(),
            enabled: true,
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"setHip3Enabled\""));
        assert!(json.contains("\"enabled\":true"));
    }

    #[test]
    fn test_set_hip3_enabled_action_serialization_false() {
        let action = SetHip3EnabledAction {
            action_type: "setHip3Enabled".to_string(),
            enabled: false,
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"type\":\"setHip3Enabled\""));
        assert!(json.contains("\"enabled\":false"));
    }

    #[test]
    fn test_misc_exchange_request_serialization() {
        let action = NoopAction {
            action_type: "noop".to_string(),
        };

        let request = MiscExchangeRequest {
            action,
            nonce: 1700000000000,
            signature: Signature {
                r: "0x1234".to_string(),
                s: "0x5678".to_string(),
                v: 27,
            },
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"nonce\":1700000000000"));
        assert!(json.contains("\"signature\":"));
    }

    // ============================================================================
    // Response Deserialization Tests
    // ============================================================================

    #[test]
    fn test_misc_response_deserialization() {
        let json = r#"{
            "status": "ok",
            "response": {
                "type": "noop",
                "data": {
                    "hash": "0xabcdef1234567890"
                }
            }
        }"#;

        let response: MiscResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.status, "ok");
        assert!(response.response.is_some());
        let data = response.response.unwrap();
        assert_eq!(data.response_type, "noop");
        assert!(data.data.is_some());
        let result = data.data.unwrap();
        assert_eq!(result.hash, Some("0xabcdef1234567890".to_string()));
    }

    #[test]
    fn test_misc_response_minimal() {
        let json = r#"{
            "status": "ok",
            "response": {
                "type": "reserveRequestWeight"
            }
        }"#;

        let response: MiscResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.status, "ok");
        assert!(response.response.is_some());
        let data = response.response.unwrap();
        assert_eq!(data.response_type, "reserveRequestWeight");
        assert!(data.data.is_none());
    }

    // ============================================================================
    // Helper Function Tests
    // ============================================================================

    #[test]
    fn test_is_misc_successful_true() {
        let response = MiscResponseData {
            response_type: "noop".to_string(),
            data: Some(MiscResultData {
                hash: Some("0xabcdef".to_string()),
            }),
        };
        assert!(is_misc_successful(&response));
    }

    #[test]
    fn test_is_misc_successful_no_data() {
        let response = MiscResponseData {
            response_type: "reserveRequestWeight".to_string(),
            data: None,
        };
        assert!(is_misc_successful(&response));
    }

    #[test]
    fn test_get_misc_hash_some() {
        let response = MiscResponseData {
            response_type: "noop".to_string(),
            data: Some(MiscResultData {
                hash: Some("0xabcdef1234".to_string()),
            }),
        };
        assert_eq!(get_misc_hash(&response), Some("0xabcdef1234"));
    }

    #[test]
    fn test_get_misc_hash_none() {
        let response = MiscResponseData {
            response_type: "noop".to_string(),
            data: None,
        };
        assert_eq!(get_misc_hash(&response), None);
    }

    // ============================================================================
    // Validation Tests
    // ============================================================================

    #[test]
    fn test_reserve_request_weight_zero_weight() {
        let client = TestClient::new("http://localhost:12345");
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.reserve_request_weight(&wallet, 0));

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidParameter(msg) => {
                assert!(msg.contains("greater than zero"));
            }
            e => panic!("Expected InvalidParameter error, got: {:?}", e),
        }
    }

    // ============================================================================
    // Integration Tests with Mocked HTTP Responses
    // ============================================================================

    #[tokio::test]
    async fn test_reserve_request_weight_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "reserveRequestWeight",
                        "data": {
                            "hash": "0xreservehash123"
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client.reserve_request_weight(&wallet, 10).await.unwrap();

        assert_eq!(response.response_type, "reserveRequestWeight");
        assert!(is_misc_successful(&response));
        assert_eq!(get_misc_hash(&response), Some("0xreservehash123"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_reserve_request_weight_large_amount() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "reserveRequestWeight",
                        "data": {
                            "hash": "0xlargehash456"
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client.reserve_request_weight(&wallet, 1000).await.unwrap();

        assert_eq!(response.response_type, "reserveRequestWeight");
        assert!(is_misc_successful(&response));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_noop_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "noop",
                        "data": {
                            "hash": "0xnoophash789"
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client.noop(&wallet).await.unwrap();

        assert_eq!(response.response_type, "noop");
        assert!(is_misc_successful(&response));
        assert_eq!(get_misc_hash(&response), Some("0xnoophash789"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_set_hip3_enabled_true() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "setHip3Enabled",
                        "data": {
                            "hash": "0xhip3hash123"
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client.set_hip3_enabled(&wallet, true).await.unwrap();

        assert_eq!(response.response_type, "setHip3Enabled");
        assert!(is_misc_successful(&response));
        assert_eq!(get_misc_hash(&response), Some("0xhip3hash123"));

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_set_hip3_enabled_false() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "status": "ok",
                    "response": {
                        "type": "setHip3Enabled",
                        "data": {
                            "hash": "0xhip3hash456"
                        }
                    }
                }"#,
            )
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let response = client.set_hip3_enabled(&wallet, false).await.unwrap();

        assert_eq!(response.response_type, "setHip3Enabled");
        assert!(is_misc_successful(&response));

        mock.assert_async().await;
    }

    // ============================================================================
    // Error Handling Tests
    // ============================================================================

    #[tokio::test]
    async fn test_reserve_request_weight_http_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client.reserve_request_weight(&wallet, 10).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("500"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_noop_http_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client.noop(&wallet).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("500"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_set_hip3_enabled_http_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client.set_hip3_enabled(&wallet, true).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("500"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_reserve_request_weight_api_error_status() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"status": "err", "response": null}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client.reserve_request_weight(&wallet, 10).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("Reserve request weight failed"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_noop_api_error_status() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"status": "err", "response": null}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client.noop(&wallet).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("Noop failed"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_set_hip3_enabled_api_error_status() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"status": "err", "response": null}"#)
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client.set_hip3_enabled(&wallet, true).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("Set HIP-3 enabled failed"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_reserve_request_weight_unauthorized() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(401)
            .with_body("Unauthorized")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client.reserve_request_weight(&wallet, 10).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("401"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_noop_rate_limited() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/exchange")
            .with_status(429)
            .with_body("Rate limit exceeded")
            .create_async()
            .await;

        let client = TestClient::new(&server.url());
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let result = client.noop(&wallet).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Api(msg) => {
                assert!(msg.contains("429"));
            }
            e => panic!("Expected Api error, got: {:?}", e),
        }

        mock.assert_async().await;
    }

    // ============================================================================
    // Wallet Signing Tests
    // ============================================================================

    #[test]
    fn test_wallet_signing_reserve_request_weight() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let action = ReserveRequestWeightAction {
                action_type: "reserveRequestWeight".to_string(),
                weight: 10,
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await.unwrap();

            assert!(signature.r.starts_with("0x"));
            assert!(signature.s.starts_with("0x"));
            assert!(signature.v == 27 || signature.v == 28);
        });
    }

    #[test]
    fn test_wallet_signing_noop() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let action = NoopAction {
                action_type: "noop".to_string(),
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await.unwrap();

            assert!(signature.r.starts_with("0x"));
            assert!(signature.s.starts_with("0x"));
            assert!(signature.v == 27 || signature.v == 28);
        });
    }

    #[test]
    fn test_wallet_signing_set_hip3_enabled() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, true).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let action = SetHip3EnabledAction {
                action_type: "setHip3Enabled".to_string(),
                enabled: true,
            };

            let nonce = Wallet::generate_nonce();
            let signature = wallet.sign_l1_action(&action, nonce, None).await.unwrap();

            assert!(signature.r.starts_with("0x"));
            assert!(signature.s.starts_with("0x"));
            assert!(signature.v == 27 || signature.v == 28);
        });
    }
}
